use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Read, Write},
    path::Path,
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::NamedTempFile;
use url::Url;

pub const PROOF_VERSION: &str = "tlsn-curl/v0";
const DEFAULT_MAX_SENT_DATA: usize = 4096;
const DEFAULT_MAX_RECV_DATA: usize = 4096;
const MAX_PRESENTATION_BYTES: usize = 64 * 1024 * 1024;

const CURL_USAGE: &str = "Usage: tlsn-curl <https-url> --out <proof.json|-> [options]

Options:
  --out <proof.json|->                 Write proof envelope to file, or stdout with -
  --verifier <addr-or-ws-url>          Override TLSNotary verifier endpoint
  -H, --header \"Name: Value\"           Add non-sensitive request header
  --header-env \"Name: ENV_VAR\"         Add sensitive request header from env
  --redact-recv-range start:end        Hide received transcript byte range
  --redact-response-json /path         Hide JSON Pointer value in response body
  --reveal-response-json /path         Reveal only JSON Pointer value in response body
  --max-sent-data <bytes>              Set prover sent data limit
  --max-recv-data <bytes>              Set prover received data limit
  --prover-bin <file>                  Override tlsn-prove binary
  --socks-proxy <addr>                 Use SOCKS proxy for proving
  --pending                            Write a pending proof envelope without proving
  -h, --help                           Show this help";

const VERIFY_USAGE: &str = "Usage: tlsn-verify <proof.json|-> [--verifier-bin <file>]

Use - to read the proof envelope JSON from stdin.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct ParsedCurlCli {
    pub url: Url,
    pub out: String,
    pub method: String,
    pub request_headers: BTreeMap<String, String>,
    pub request_header_env: BTreeMap<String, String>,
    pub recv_redact_ranges: Vec<ByteRange>,
    pub redact_response_json_pointers: Vec<String>,
    pub reveal_response_json_pointers: Vec<String>,
    pub prover_bin: Option<String>,
    pub verifier: Option<String>,
    pub socks_proxy: Option<String>,
    pub max_sent_data: usize,
    pub max_recv_data: usize,
    pub pending: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedVerifyCli {
    pub proof_path: String,
    pub verifier_bin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed<T> {
    Help,
    Args(T),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpFetchClaim {
    pub url: String,
    pub method: String,
    #[serde(rename = "requestHeaders")]
    pub request_headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TlsnPresentationProof {
    pub format: String,
    pub encoding: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier: Option<String>,
    pub max_sent_data: usize,
    pub max_recv_data: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TlsnProofEnvelope {
    pub version: String,
    pub kind: String,
    pub created_at: String,
    pub claim: HttpFetchClaim,
    pub tlsn: TlsnProofState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum TlsnProofState {
    Pending {
        proof: Option<TlsnPresentationProof>,
    },
    Complete {
        proof: TlsnPresentationProof,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevealedTextSegment {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VerifierJson {
    pub valid: Option<bool>,
    pub server_name: Option<String>,
    pub time: Option<u64>,
    pub revealed_headers: Option<String>,
    pub revealed_body: Option<String>,
    pub revealed_body_segments: Option<Vec<RevealedTextSegment>>,
    pub revealed_sent: Option<String>,
    pub revealed_recv: Option<String>,
    pub revealed_recv_segments: Option<Vec<RevealedTextSegment>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FetchVerificationResult {
    Ok(VerifiedFetchResult),
    Failed(FailedFetchVerification),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedFetchResult {
    pub ok: bool,
    pub checked_at: String,
    pub claim: HttpFetchClaim,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_body_segments: Option<Vec<RevealedTextSegment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_json_values: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_sent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_recv: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revealed_recv_segments: Option<Vec<RevealedTextSegment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FailedFetchVerification {
    pub ok: bool,
    pub checked_at: String,
    pub reason: String,
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn normalize_http_method(method: &str) -> Result<String> {
    let normalized = method.trim().to_uppercase();
    if normalized.is_empty() || !normalized.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(anyhow!("invalid HTTP method: {method}"));
    }
    Ok(normalized)
}

pub fn create_proof_envelope(
    url: &Url,
    method: &str,
    created_at: String,
    request_headers: BTreeMap<String, String>,
    tlsn_proof: Option<TlsnPresentationProof>,
) -> Result<TlsnProofEnvelope> {
    if url.scheme() != "https" {
        return Err(anyhow!("tlsn-curl only accepts https URLs"));
    }

    Ok(TlsnProofEnvelope {
        version: PROOF_VERSION.to_string(),
        kind: "tlsnotary-fetch-proof".to_string(),
        created_at,
        claim: HttpFetchClaim {
            url: url.to_string(),
            method: normalize_http_method(method)?,
            request_headers,
        },
        tlsn: match tlsn_proof {
            Some(proof) => TlsnProofState::Complete { proof },
            None => TlsnProofState::Pending { proof: None },
        },
    })
}

fn take_value(args: &[String], index: usize, flag: &str) -> Result<String> {
    let value = args
        .get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| anyhow!("{flag} requires a value"))?;
    Ok(value.clone())
}

fn parse_header(value: &str) -> Result<(String, String)> {
    let Some((name, rest)) = value.split_once(':') else {
        return Err(anyhow!(
            "{value} is not a valid header; expected \"Name: Value\""
        ));
    };
    let header_name = name.trim();
    if header_name.is_empty() {
        return Err(anyhow!(
            "{value} is not a valid header; expected \"Name: Value\""
        ));
    }
    Ok((header_name.to_string(), rest.trim().to_string()))
}

fn is_sensitive_header_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization" | "proxy-authorization" | "cookie" | "set-cookie" | "x-api-key" | "api-key"
    )
}

fn reject_sensitive_header_arg(name: &str) -> Result<()> {
    if is_sensitive_header_name(name) {
        return Err(anyhow!(
            "sensitive header \"{name}\" must be passed with --header-env"
        ));
    }
    Ok(())
}

pub fn parse_byte_range(value: &str, flag: &str) -> Result<ByteRange> {
    let Some((start, end)) = value.split_once(':') else {
        return Err(anyhow!("{flag} must use start:end byte offsets"));
    };
    let start = start
        .parse::<usize>()
        .map_err(|_| anyhow!("{flag} must satisfy 0 <= start < end"))?;
    let end = end
        .parse::<usize>()
        .map_err(|_| anyhow!("{flag} must satisfy 0 <= start < end"))?;
    if end <= start {
        return Err(anyhow!("{flag} must satisfy 0 <= start < end"));
    }
    Ok(ByteRange { start, end })
}

fn parse_positive_integer(value: &str, flag: &str) -> Result<usize> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| anyhow!("{flag} must be a positive integer"))?;
    if parsed == 0 {
        return Err(anyhow!("{flag} must be a positive integer"));
    }
    Ok(parsed)
}

fn unsupported_curl_message(option: &str) -> Option<&'static str> {
    match option {
        "-o" | "--output" => Some(
            "curl --output writes the response body; tlsn-curl writes proof envelopes with --out",
        ),
        "-O" => Some("saving response bodies is not supported"),
        "-X" | "--request" => Some("custom methods are not supported by the TLSNotary prover yet"),
        "-d" | "--data" | "--data-raw" | "--data-binary" | "--json" => {
            Some("request bodies are not supported by the TLSNotary prover yet")
        }
        "-I" | "--head" => Some("HEAD requests are not supported by the TLSNotary prover yet"),
        "-L" | "--location" => Some("redirect following is not supported"),
        "-b" | "--cookie" => Some("cookie parsing is not supported; use --header-env for Cookie"),
        "-u" | "--user" => {
            Some("credential parsing is not supported; use --header-env for Authorization")
        }
        "-F" | "--form" => Some("multipart form uploads are not supported"),
        _ => None,
    }
}

fn reject_unsupported_curl_option(arg: &str) -> Result<()> {
    const OPTIONS: &[&str] = &[
        "-o",
        "--output",
        "-O",
        "-X",
        "--request",
        "-d",
        "--data",
        "--data-raw",
        "--data-binary",
        "--json",
        "-I",
        "--head",
        "-L",
        "--location",
        "-b",
        "--cookie",
        "-u",
        "--user",
        "-F",
        "--form",
    ];

    for option in OPTIONS {
        let matches = arg == *option
            || (option.starts_with("--") && arg.starts_with(&format!("{option}=")))
            || (!option.starts_with("--") && arg.starts_with(option) && arg.len() > option.len());
        if matches {
            return Err(anyhow!(
                "unsupported curl-like option {option}: {}",
                unsupported_curl_message(option).unwrap_or("unsupported option")
            ));
        }
    }
    Ok(())
}

pub fn parse_curl_cli_args<I>(args: I) -> Result<Parsed<ParsedCurlCli>>
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(Parsed::Help);
    }

    let mut out = None;
    let mut method = "GET".to_string();
    let mut prover_bin = None;
    let mut verifier = None;
    let mut socks_proxy = None;
    let mut max_sent_data = DEFAULT_MAX_SENT_DATA;
    let mut max_recv_data = DEFAULT_MAX_RECV_DATA;
    let mut pending = false;
    let mut request_headers = BTreeMap::new();
    let mut request_header_env = BTreeMap::new();
    let mut recv_redact_ranges = Vec::new();
    let mut redact_response_json_pointers = Vec::new();
    let mut reveal_response_json_pointers = Vec::new();
    let mut positional = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--out" {
            out = Some(take_value(&args, i, "--out")?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--out=") {
            out = Some(value.to_string());
        } else if arg == "--method" {
            method = take_value(&args, i, "--method")?;
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--method=") {
            method = value.to_string();
        } else if arg == "--header" || arg == "-H" {
            let (name, value) = parse_header(&take_value(&args, i, arg)?)?;
            reject_sensitive_header_arg(&name)?;
            request_headers.insert(name, value);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--header=") {
            let (name, value) = parse_header(value)?;
            reject_sensitive_header_arg(&name)?;
            request_headers.insert(name, value);
        } else if arg == "--header-env" {
            let (name, value) = parse_header(&take_value(&args, i, "--header-env")?)?;
            request_header_env.insert(name, value);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--header-env=") {
            let (name, value) = parse_header(value)?;
            request_header_env.insert(name, value);
        } else if arg == "--prover-bin" {
            prover_bin = Some(take_value(&args, i, "--prover-bin")?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--prover-bin=") {
            prover_bin = Some(value.to_string());
        } else if arg == "--verifier" {
            verifier = Some(take_value(&args, i, "--verifier")?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--verifier=") {
            verifier = Some(value.to_string());
        } else if arg == "--socks-proxy" {
            socks_proxy = Some(take_value(&args, i, "--socks-proxy")?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--socks-proxy=") {
            socks_proxy = Some(value.to_string());
        } else if arg == "--max-sent-data" {
            max_sent_data = parse_positive_integer(
                &take_value(&args, i, "--max-sent-data")?,
                "--max-sent-data",
            )?;
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--max-sent-data=") {
            max_sent_data = parse_positive_integer(value, "--max-sent-data")?;
        } else if arg == "--max-recv-data" {
            max_recv_data = parse_positive_integer(
                &take_value(&args, i, "--max-recv-data")?,
                "--max-recv-data",
            )?;
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--max-recv-data=") {
            max_recv_data = parse_positive_integer(value, "--max-recv-data")?;
        } else if arg == "--redact-recv-range" {
            recv_redact_ranges.push(parse_byte_range(
                &take_value(&args, i, "--redact-recv-range")?,
                "--redact-recv-range",
            )?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--redact-recv-range=") {
            recv_redact_ranges.push(parse_byte_range(value, "--redact-recv-range")?);
        } else if arg == "--redact-response-json" {
            redact_response_json_pointers.push(take_value(&args, i, arg)?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--redact-response-json=") {
            redact_response_json_pointers.push(value.to_string());
        } else if arg == "--reveal-response-json" {
            reveal_response_json_pointers.push(take_value(&args, i, arg)?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--reveal-response-json=") {
            reveal_response_json_pointers.push(value.to_string());
        } else if arg == "--pending" {
            pending = true;
        } else if arg.starts_with('-') {
            reject_unsupported_curl_option(arg)?;
            return Err(anyhow!("unknown option: {arg}"));
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }

    if positional.len() != 1 {
        return Err(anyhow!("expected exactly one URL"));
    }
    let out = out.ok_or_else(|| anyhow!("--out is required"))?;
    let url = Url::parse(&positional[0]).map_err(|error| anyhow!("{error}"))?;
    if url.scheme() != "https" {
        return Err(anyhow!("tlsn-curl only accepts https URLs"));
    }

    Ok(Parsed::Args(ParsedCurlCli {
        url,
        out,
        method,
        request_headers,
        request_header_env,
        recv_redact_ranges,
        redact_response_json_pointers,
        reveal_response_json_pointers,
        prover_bin,
        verifier,
        socks_proxy,
        max_sent_data,
        max_recv_data,
        pending,
    }))
}

pub fn parse_verify_cli_args<I>(args: I) -> Result<Parsed<ParsedVerifyCli>>
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return Ok(Parsed::Help);
    }

    let mut verifier_bin = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--verifier-bin" {
            verifier_bin = Some(take_value(&args, i, "--verifier-bin")?);
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--verifier-bin=") {
            verifier_bin = Some(value.to_string());
        } else if arg.starts_with("--") {
            return Err(anyhow!("unknown option: {arg}"));
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }

    if positional.len() != 1 {
        return Err(anyhow!("expected exactly one proof file"));
    }

    Ok(Parsed::Args(ParsedVerifyCli {
        proof_path: positional.remove(0),
        verifier_bin,
    }))
}

fn claim_request_headers(
    headers: &BTreeMap<String, String>,
    header_env: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut claim_headers = headers.clone();
    for name in header_env.keys() {
        claim_headers.insert(name.clone(), "[REDACTED]".to_string());
    }
    claim_headers
}

pub fn default_sidecar_bin(env_name: &str, installed_name: &str, repo_local_path: &str) -> String {
    if let Ok(value) = env::var(env_name) {
        if !value.is_empty() {
            return value;
        }
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(file_name) = current_exe.file_name().and_then(|name| name.to_str()) {
            if matches!(
                file_name,
                "tlsn-curl" | "tlsn-curl.exe" | "tlsn-verify" | "tlsn-verify.exe"
            ) {
                if let Some(parent) = current_exe.parent() {
                    let sidecar = parent.join(installed_name);
                    if sidecar.is_file() {
                        return sidecar.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    repo_local_path.to_string()
}

pub fn default_prover_bin() -> String {
    default_sidecar_bin(
        "TLSN_PROVER_BIN",
        "tlsn-prove",
        "crates/tlsn-prover/target/debug/tlsn-prove",
    )
}

pub fn default_verifier_bin() -> String {
    default_sidecar_bin(
        "TLSN_VERIFIER_BIN",
        "tlsn-verifier",
        "crates/tlsn-verifier/target/release/tlsn-verifier",
    )
}

pub fn build_tlsn_prover_command(
    parsed: &ParsedCurlCli,
    output_path: &Path,
) -> Result<(String, Vec<String>)> {
    if parsed.url.scheme() != "https" {
        return Err(anyhow!("tlsn-curl only accepts https URLs"));
    }
    let method = normalize_http_method(&parsed.method)?;
    if method != "GET" {
        return Err(anyhow!(
            "TLSNotary sidecar currently supports GET requests only"
        ));
    }

    let command = parsed.prover_bin.clone().unwrap_or_else(default_prover_bin);
    let mut args = vec![
        parsed.url.to_string(),
        "--output".to_string(),
        output_path.to_string_lossy().to_string(),
        "--max-sent-data".to_string(),
        parsed.max_sent_data.to_string(),
        "--max-recv-data".to_string(),
        parsed.max_recv_data.to_string(),
    ];

    if let Some(verifier) = &parsed.verifier {
        args.push("--verifier".to_string());
        args.push(verifier.clone());
    }
    if let Some(socks_proxy) = &parsed.socks_proxy {
        args.push("--socks-proxy".to_string());
        args.push(socks_proxy.clone());
    }
    for range in &parsed.recv_redact_ranges {
        args.push("--redact-recv-range".to_string());
        args.push(format!("{}:{}", range.start, range.end));
    }
    for pointer in &parsed.redact_response_json_pointers {
        args.push("--redact-response-json".to_string());
        args.push(pointer.clone());
    }
    for pointer in &parsed.reveal_response_json_pointers {
        args.push("--reveal-response-json".to_string());
        args.push(pointer.clone());
    }
    for (name, value) in &parsed.request_headers {
        args.push("--header".to_string());
        args.push(format!("{name}: {value}"));
    }
    for (name, env_name) in &parsed.request_header_env {
        args.push("--header-env".to_string());
        args.push(format!("{name}: {env_name}"));
    }
    for name in parsed.request_header_env.keys() {
        args.push("--redact-sent-header".to_string());
        args.push(name.clone());
    }

    Ok((command, args))
}

pub fn create_tlsn_presentation_proof(parsed: &ParsedCurlCli) -> Result<TlsnPresentationProof> {
    let temp = NamedTempFile::new().context("failed to create temporary presentation path")?;
    let output_path = temp.path().to_path_buf();
    let (command, args) = build_tlsn_prover_command(parsed, &output_path)?;
    let output = Command::new(&command)
        .args(&args)
        .output()
        .with_context(|| format!("failed to run {command}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        return Err(anyhow!(
            "tlsn-prove failed with exit code {}{}",
            output.status.code().unwrap_or(-1),
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {stderr}")
            }
        ));
    }
    if stdout.is_empty() {
        return Err(anyhow!("tlsn-prove did not emit a base64 presentation"));
    }

    Ok(TlsnPresentationProof {
        format: "presentation.tlsn".to_string(),
        encoding: "base64".to_string(),
        data: stdout,
        verifier: parsed.verifier.clone(),
        max_sent_data: parsed.max_sent_data,
        max_recv_data: parsed.max_recv_data,
    })
}

pub fn run_curl_cli<I>(args: I) -> CliResult
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    match run_curl_cli_inner(args) {
        Ok(result) => result,
        Err(error) => CliResult {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n{CURL_USAGE}\n"),
        },
    }
}

fn run_curl_cli_inner<I>(args: I) -> Result<CliResult>
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let parsed = match parse_curl_cli_args(args)? {
        Parsed::Help => {
            return Ok(CliResult {
                code: 0,
                stdout: format!("{CURL_USAGE}\n"),
                stderr: String::new(),
            });
        }
        Parsed::Args(parsed) => parsed,
    };

    let tlsn_proof = if parsed.pending {
        None
    } else {
        Some(create_tlsn_presentation_proof(&parsed)?)
    };
    let envelope = create_proof_envelope(
        &parsed.url,
        &parsed.method,
        now_iso(),
        claim_request_headers(&parsed.request_headers, &parsed.request_header_env),
        tlsn_proof,
    )?;
    let envelope_text = format!("{}\n", serde_json::to_string_pretty(&envelope)?);

    if parsed.out == "-" {
        return Ok(CliResult {
            code: 0,
            stdout: envelope_text,
            stderr: String::new(),
        });
    }

    fs::write(&parsed.out, envelope_text)?;
    let status = match envelope.tlsn {
        TlsnProofState::Complete { .. } => "proof",
        TlsnProofState::Pending { .. } => "pending proof",
    };
    Ok(CliResult {
        code: 0,
        stdout: String::new(),
        stderr: format!("wrote TLSNotary {status} envelope to {}\n", parsed.out),
    })
}

pub fn verify_proof_envelope(
    proof: &TlsnProofEnvelope,
    verifier_bin: Option<&str>,
) -> FetchVerificationResult {
    verify_proof_envelope_at(proof, verifier_bin, now_iso())
}

pub fn verify_proof_envelope_at(
    proof: &TlsnProofEnvelope,
    verifier_bin: Option<&str>,
    checked_at: String,
) -> FetchVerificationResult {
    let TlsnProofState::Complete {
        proof: presentation,
    } = &proof.tlsn
    else {
        return FetchVerificationResult::Failed(FailedFetchVerification {
            ok: false,
            checked_at,
            reason: "TLSNotary proof envelope is pending".to_string(),
        });
    };

    let result = (|| -> Result<FetchVerificationResult> {
        let estimated_bytes = presentation
            .data
            .split_whitespace()
            .collect::<String>()
            .len()
            * 3
            / 4;
        if estimated_bytes > MAX_PRESENTATION_BYTES {
            return Err(anyhow!(
                "TLSNotary presentation is too large; max {MAX_PRESENTATION_BYTES} bytes"
            ));
        }
        let presentation_bytes = STANDARD
            .decode(presentation.data.split_whitespace().collect::<String>())
            .context("failed to decode base64 TLSNotary presentation")?;
        if presentation_bytes.len() > MAX_PRESENTATION_BYTES {
            return Err(anyhow!(
                "TLSNotary presentation is too large; max {MAX_PRESENTATION_BYTES} bytes"
            ));
        }

        let verifier_result = run_verifier_command_with_temp_file(
            &presentation_bytes,
            verifier_bin
                .map(ToOwned::to_owned)
                .unwrap_or_else(default_verifier_bin)
                .as_str(),
        )?;
        let parsed: Option<VerifierJson> = serde_json::from_str(&verifier_result.stdout).ok();

        if verifier_result.code != 0 || !matches!(parsed.as_ref().and_then(|v| v.valid), Some(true))
        {
            let reason = parsed
                .and_then(|value| value.error)
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    if verifier_result.stderr.is_empty() {
                        None
                    } else {
                        Some(verifier_result.stderr.clone())
                    }
                })
                .unwrap_or_else(|| {
                    format!(
                        "tlsn-verifier failed with exit code {}",
                        verifier_result.code
                    )
                });
            return Ok(FetchVerificationResult::Failed(FailedFetchVerification {
                ok: false,
                checked_at: checked_at.clone(),
                reason,
            }));
        }

        let parsed = parsed.expect("validated parsed verifier JSON");
        let expected_host = Url::parse(&proof.claim.url)
            .context("claim URL is invalid")?
            .host_str()
            .ok_or_else(|| anyhow!("claim URL has no host"))?
            .to_string();
        if parsed.server_name.as_deref() != Some(expected_host.as_str()) {
            return Ok(FetchVerificationResult::Failed(FailedFetchVerification {
                ok: false,
                checked_at: checked_at.clone(),
                reason: format!(
                    "TLSNotary server name \"{}\" does not match \"{}\"",
                    parsed.server_name.as_deref().unwrap_or("unknown"),
                    expected_host
                ),
            }));
        }

        if let Some(reason) =
            validate_revealed_request_matches_claim(&proof.claim, parsed.revealed_sent.as_deref())
        {
            return Ok(FetchVerificationResult::Failed(FailedFetchVerification {
                ok: false,
                checked_at: checked_at.clone(),
                reason,
            }));
        }

        let revealed_json_values =
            parse_revealed_json_values(parsed.revealed_body_segments.as_deref())
                .or_else(|| parse_revealed_json_values(parsed.revealed_recv_segments.as_deref()));

        Ok(FetchVerificationResult::Ok(VerifiedFetchResult {
            ok: true,
            checked_at: checked_at.clone(),
            claim: proof.claim.clone(),
            server_name: parsed.server_name,
            session_time: parsed.time,
            revealed_headers: parsed.revealed_headers,
            revealed_body: parsed.revealed_body,
            revealed_body_segments: parsed.revealed_body_segments,
            revealed_json_values,
            revealed_sent: parsed.revealed_sent,
            revealed_recv: parsed.revealed_recv,
            revealed_recv_segments: parsed.revealed_recv_segments,
        }))
    })();

    result.unwrap_or_else(|error| {
        FetchVerificationResult::Failed(FailedFetchVerification {
            ok: false,
            checked_at: checked_at.clone(),
            reason: format!("TLSNotary verifier failed: {error}"),
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierCommandResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_verifier_command_with_temp_file(
    presentation: &[u8],
    verifier_bin: &str,
) -> Result<VerifierCommandResult> {
    let mut temp = NamedTempFile::new().context("failed to create temporary presentation file")?;
    temp.write_all(presentation)?;
    let output = Command::new(verifier_bin)
        .args(["verify", temp.path().to_string_lossy().as_ref()])
        .output()
        .with_context(|| format!("failed to run {verifier_bin}"))?;
    Ok(VerifierCommandResult {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn parse_revealed_json_values(segments: Option<&[RevealedTextSegment]>) -> Option<Vec<Value>> {
    let segments = segments?;
    if segments.is_empty() {
        return None;
    }
    let mut values = Vec::new();
    for segment in segments {
        values.push(serde_json::from_str(&segment.text).ok()?);
    }
    Some(values)
}

fn expected_request_target(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{}", url.path(), query),
        None => url.path().to_string(),
    }
}

fn parse_header_value(header_lines: &[&str], name: &str) -> Option<String> {
    let expected = name.to_ascii_lowercase();
    for line in header_lines {
        let Some((header_name, value)) = line.split_once(':') else {
            continue;
        };
        if header_name.trim().to_ascii_lowercase() == expected {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn is_redacted_claim_value(value: &str) -> bool {
    value == "[REDACTED]"
}

pub fn validate_revealed_request_matches_claim(
    claim: &HttpFetchClaim,
    revealed_sent: Option<&str>,
) -> Option<String> {
    let revealed_sent = revealed_sent?;
    let request_end = revealed_sent
        .find("\r\n\r\n")
        .or_else(|| revealed_sent.find("\n\n"));
    let request_text = request_end
        .map(|index| &revealed_sent[..index])
        .unwrap_or(revealed_sent);
    let lines: Vec<&str> = request_text.lines().collect();
    let request_line = lines.first().map(|line| line.trim()).unwrap_or_default();
    if request_line.is_empty() || request_line.contains("[REDACTED]") {
        return Some(
            "TLSNotary proof did not reveal the complete sent HTTP request line".to_string(),
        );
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() != 3 || !parts[2].starts_with("HTTP/") {
        return Some("TLSNotary proof revealed an invalid sent HTTP request line".to_string());
    }

    let claimed_url = match Url::parse(&claim.url) {
        Ok(url) => url,
        Err(_) => return Some("claim URL is invalid".to_string()),
    };
    let expected_method = match normalize_http_method(&claim.method) {
        Ok(method) => method,
        Err(error) => return Some(error.to_string()),
    };
    let expected_target = expected_request_target(&claimed_url);

    if parts[0] != expected_method {
        return Some(format!(
            "TLSNotary sent request method \"{}\" does not match claim \"{}\"",
            parts[0], expected_method
        ));
    }
    if parts[1] != expected_target {
        return Some(format!(
            "TLSNotary sent request target \"{}\" does not match claim \"{}\"",
            parts[1], expected_target
        ));
    }

    let header_lines = if lines.len() > 1 { &lines[1..] } else { &[] };
    let host = parse_header_value(header_lines, "host");
    if host
        .as_deref()
        .is_none_or(|host| host.contains("[REDACTED]"))
    {
        return Some("TLSNotary proof did not reveal the complete sent Host header".to_string());
    }
    let expected_host = claimed_url
        .host_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if host.as_deref().unwrap().to_ascii_lowercase() != expected_host {
        return Some(format!(
            "TLSNotary sent Host header \"{}\" does not match claim \"{}\"",
            host.unwrap(),
            expected_host
        ));
    }

    for (name, expected_value) in &claim.request_headers {
        if is_redacted_claim_value(expected_value) {
            continue;
        }
        let actual_value = parse_header_value(header_lines, name);
        if actual_value
            .as_deref()
            .is_none_or(|value| value.contains("[REDACTED]"))
        {
            return Some(format!(
                "TLSNotary proof did not reveal the complete {name} header"
            ));
        }
        if actual_value.as_deref() != Some(expected_value.as_str()) {
            return Some(format!("TLSNotary sent {name} header does not match claim"));
        }
    }

    None
}

pub fn run_verify_cli<I>(args: I) -> CliResult
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    match run_verify_cli_inner(args) {
        Ok(result) => result,
        Err(error) => CliResult {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n{VERIFY_USAGE}\n"),
        },
    }
}

fn run_verify_cli_inner<I>(args: I) -> Result<CliResult>
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let parsed = match parse_verify_cli_args(args)? {
        Parsed::Help => {
            return Ok(CliResult {
                code: 0,
                stdout: format!("{VERIFY_USAGE}\n"),
                stderr: String::new(),
            });
        }
        Parsed::Args(parsed) => parsed,
    };

    let proof_text = if parsed.proof_path == "-" {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        text
    } else {
        fs::read_to_string(&parsed.proof_path)?
    };
    let proof: TlsnProofEnvelope = serde_json::from_str(&proof_text)?;
    let result = verify_proof_envelope(&proof, parsed.verifier_bin.as_deref());
    let code = match result {
        FetchVerificationResult::Ok(_) => 0,
        FetchVerificationResult::Failed(_) => 1,
    };
    Ok(CliResult {
        code,
        stdout: format!("{}\n", serde_json::to_string_pretty(&result)?),
        stderr: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_time() -> String {
        "2026-05-16T00:00:00.000Z".to_string()
    }

    fn complete_proof(overrides: Option<BTreeMap<String, String>>) -> TlsnProofEnvelope {
        create_proof_envelope(
            &Url::parse("https://example.com/v1/data?symbol=BTC").unwrap(),
            "GET",
            fixed_time(),
            overrides.unwrap_or_default(),
            Some(TlsnPresentationProof {
                format: "presentation.tlsn".to_string(),
                encoding: "base64".to_string(),
                data: STANDARD.encode("presentation"),
                verifier: None,
                max_sent_data: 4096,
                max_recv_data: 4096,
            }),
        )
        .unwrap()
    }

    #[test]
    fn parse_curl_cli_accepts_url_and_out_file() {
        let Parsed::Args(parsed) =
            parse_curl_cli_args(["https://example.com/path", "--out", "proof.json"]).unwrap()
        else {
            panic!("expected args");
        };

        assert_eq!(parsed.url.as_str(), "https://example.com/path");
        assert_eq!(parsed.out, "proof.json");
        assert_eq!(parsed.method, "GET");
        assert!(parsed.request_headers.is_empty());
        assert_eq!(parsed.max_sent_data, 4096);
        assert_eq!(parsed.max_recv_data, 4096);
    }

    #[test]
    fn parse_curl_cli_accepts_header_alias_and_rejects_sensitive_header() {
        let Parsed::Args(parsed) = parse_curl_cli_args([
            "https://example.com/data",
            "--out=proof.json",
            "-H",
            "Accept-Language: en-US",
        ])
        .unwrap() else {
            panic!("expected args");
        };
        assert_eq!(
            parsed.request_headers.get("Accept-Language"),
            Some(&"en-US".to_string())
        );

        let error = parse_curl_cli_args([
            "https://example.com/data",
            "--out=proof.json",
            "--header",
            "Authorization: Bearer secret",
        ])
        .unwrap_err();
        assert!(error.to_string().contains("--header-env"));
    }

    #[test]
    fn parse_curl_cli_rejects_unsupported_curl_options() {
        for flag in ["-o", "--output=response.json", "--json"] {
            let error = parse_curl_cli_args(["https://example.com/data", "--out=proof.json", flag])
                .unwrap_err();
            assert!(error.to_string().contains("unsupported curl-like option"));
        }
    }

    #[test]
    fn parse_curl_cli_accepts_redaction_options() {
        let Parsed::Args(parsed) = parse_curl_cli_args([
            "https://example.com/data",
            "--out=proof.json",
            "--redact-recv-range",
            "12:34",
            "--redact-response-json",
            "/private",
            "--reveal-response-json=/data/amount",
        ])
        .unwrap() else {
            panic!("expected args");
        };

        assert_eq!(
            parsed.recv_redact_ranges,
            [ByteRange { start: 12, end: 34 }]
        );
        assert_eq!(parsed.redact_response_json_pointers, ["/private"]);
        assert_eq!(parsed.reveal_response_json_pointers, ["/data/amount"]);
    }

    #[test]
    fn create_proof_envelope_creates_pending_envelope() {
        let envelope = create_proof_envelope(
            &Url::parse("https://example.com/").unwrap(),
            "get",
            fixed_time(),
            BTreeMap::new(),
            None,
        )
        .unwrap();

        assert_eq!(envelope.version, PROOF_VERSION);
        assert_eq!(envelope.kind, "tlsnotary-fetch-proof");
        assert_eq!(envelope.claim.method, "GET");
        assert!(matches!(
            envelope.tlsn,
            TlsnProofState::Pending { proof: None }
        ));
    }

    #[test]
    fn build_tlsn_prover_command_maps_cli_options_to_sidecar() {
        let Parsed::Args(parsed) = parse_curl_cli_args([
            "https://api.example.com/status",
            "--out=proof.json",
            "--prover-bin=tlsn-prove",
            "--verifier=localhost:7047",
            "--header",
            "X-Feature: public",
            "--header-env",
            "X-Api-Key: API_KEY",
            "--redact-recv-range=10:20",
            "--redact-response-json=/private",
            "--reveal-response-json=/data/amount",
            "--max-sent-data=1024",
            "--max-recv-data=8192",
        ])
        .unwrap() else {
            panic!("expected args");
        };
        let (command, args) =
            build_tlsn_prover_command(&parsed, Path::new("out.presentation.tlsn")).unwrap();

        assert_eq!(command, "tlsn-prove");
        assert_eq!(
            args,
            [
                "https://api.example.com/status",
                "--output",
                "out.presentation.tlsn",
                "--max-sent-data",
                "1024",
                "--max-recv-data",
                "8192",
                "--verifier",
                "localhost:7047",
                "--redact-recv-range",
                "10:20",
                "--redact-response-json",
                "/private",
                "--reveal-response-json",
                "/data/amount",
                "--header",
                "X-Feature: public",
                "--header-env",
                "X-Api-Key: API_KEY",
                "--redact-sent-header",
                "X-Api-Key",
            ]
        );
    }

    #[test]
    fn validate_revealed_request_matches_claim_accepts_and_rejects_headers() {
        let mut claim = complete_proof(None).claim;
        claim
            .request_headers
            .insert("X-Feature".to_string(), "public".to_string());

        assert_eq!(
            validate_revealed_request_matches_claim(
                &claim,
                Some("GET /v1/data?symbol=BTC HTTP/1.1\r\nHost: example.com\r\nX-Feature: public\r\n\r\n"),
            ),
            None
        );

        let reason = validate_revealed_request_matches_claim(
            &claim,
            Some("GET /v1/data?symbol=BTC HTTP/1.1\r\nHost: example.com\r\nX-Feature: [REDACTED]\r\n\r\n"),
        )
        .unwrap();
        assert!(reason.contains("X-Feature header"));
    }

    #[test]
    fn verify_proof_rejects_pending_and_server_name_mismatch_without_running_verifier() {
        let pending = create_proof_envelope(
            &Url::parse("https://example.com/").unwrap(),
            "GET",
            fixed_time(),
            BTreeMap::new(),
            None,
        )
        .unwrap();
        let FetchVerificationResult::Failed(result) =
            verify_proof_envelope_at(&pending, Some("unused"), fixed_time())
        else {
            panic!("expected failure");
        };
        assert!(result.reason.contains("pending"));
    }

    #[test]
    fn parse_verify_cli_accepts_proof_path_and_verifier_binary() {
        let Parsed::Args(parsed) =
            parse_verify_cli_args(["proof.json", "--verifier-bin", "tlsn-verifier"]).unwrap()
        else {
            panic!("expected args");
        };
        assert_eq!(parsed.proof_path, "proof.json");
        assert_eq!(parsed.verifier_bin.as_deref(), Some("tlsn-verifier"));
    }

    #[test]
    fn parse_revealed_json_values_maps_segments() {
        let segments = vec![
            RevealedTextSegment {
                start: 12,
                end: 20,
                text: "\"123.45\"".to_string(),
            },
            RevealedTextSegment {
                start: 32,
                end: 36,
                text: "true".to_string(),
            },
        ];
        assert_eq!(
            parse_revealed_json_values(Some(&segments)),
            Some(vec![Value::String("123.45".to_string()), Value::Bool(true)])
        );
    }
}
