//! TLSNotary Prover — generates a .presentation.tlsn file for a given URL.
//!
//! Runs both Prover and Verifier in-process (via tokio::io::duplex),
//! connects to the target HTTPS server, and outputs a presentation file
//! that can be verified by `tlsn-verifier`.

use std::ops::Range;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;
use http_body_util::Empty;
use hyper::{body::Bytes, Request};
use hyper_util::rt::TokioIo;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::oneshot,
};
use tokio_util::compat::{FuturesAsyncReadCompatExt, TokioAsyncReadCompatExt};

use tlsn::{
    attestation::{
        presentation::Presentation,
        request::{Request as AttestationRequest, RequestConfig},
        signing::Secp256k1Signer,
        Attestation, AttestationConfig, CryptoProvider, Secrets,
    },
    config::{
        prove::ProveConfig,
        prover::ProverConfig,
        tls::TlsClientConfig,
        tls_commit::{mpc::MpcTlsConfig, TlsCommitConfig},
        verifier::VerifierConfig,
    },
    connection::{ConnectionInfo, HandshakeData, ServerName, TranscriptLength},
    prover::{state::Committed, Prover, ProverOutput},
    transcript::{ContentType, Transcript, TranscriptCommitConfig},
    verifier::VerifierOutput,
    Session,
};

#[derive(Parser)]
#[command(
    name = "tlsn-prove",
    about = "Generate a TLSNotary presentation for a URL"
)]
struct Cli {
    /// Target URL to fetch
    url: String,

    /// Output file path (default: presentation.tlsn)
    #[arg(short, long, default_value = "presentation.tlsn")]
    output: PathBuf,

    /// Verifier server address (e.g. localhost:7047). If omitted, runs verifier in-process.
    #[arg(short, long)]
    verifier: Option<String>,

    /// SOCKS5 proxy for target connections (e.g. socks5://127.0.0.1:9050 for Tor)
    #[arg(long)]
    socks_proxy: Option<String>,

    /// Custom HTTP header (format: "Key: Value"). Can be specified multiple times.
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,

    /// Custom HTTP header from an environment variable (format: "Key: ENV_NAME").
    /// Prefer this for secrets so header values do not appear in process args.
    #[arg(long = "header-env")]
    header_env: Vec<String>,

    /// Maximum bytes of sent data for MPC-TLS circuit (default: 4096).
    #[arg(long, default_value_t = 4096)]
    max_sent_data: usize,

    /// Maximum bytes of received data for MPC-TLS circuit (default: 4096).
    /// Smaller values reduce MPC computation time. Set close to expected response size.
    #[arg(long, default_value_t = 4096)]
    max_recv_data: usize,

    /// Header names to redact from the sent HTTP request (selective disclosure).
    /// Values of these headers will not be revealed in the presentation.
    /// Can be specified multiple times. Case-insensitive.
    /// Example: --redact-sent-header authorization --redact-sent-header cookie
    #[arg(long = "redact-sent-header")]
    redact_sent_headers: Vec<String>,

    /// Received transcript byte range to redact from the presentation.
    /// Format: start:end. Can be specified multiple times.
    #[arg(long = "redact-recv-range", value_parser = parse_byte_range)]
    redact_recv_ranges: Vec<ByteRange>,

    /// JSON Pointer value in the HTTP response body to redact.
    /// The raw JSON value must be uniquely identifiable in an unchunked UTF-8 body.
    #[arg(long = "redact-response-json")]
    redact_response_json: Vec<String>,

    /// JSON Pointer value in the HTTP response body to reveal, redacting all other received bytes.
    /// The raw JSON value must be uniquely identifiable in an unchunked UTF-8 body.
    #[arg(long = "reveal-response-json")]
    reveal_response_json: Vec<String>,
}

#[derive(Clone, Debug)]
struct ByteRange(Range<usize>);

fn parse_byte_range(value: &str) -> Result<ByteRange, String> {
    let (start, end) = value
        .split_once(':')
        .ok_or_else(|| "expected start:end byte offsets".to_string())?;
    let start = start
        .parse::<usize>()
        .map_err(|_| "range start must be a non-negative integer".to_string())?;
    let end = end
        .parse::<usize>()
        .map_err(|_| "range end must be a non-negative integer".to_string())?;
    if start >= end {
        return Err("range must satisfy start < end".to_string());
    }
    Ok(ByteRange(start..end))
}

#[derive(Clone, Debug, Default)]
struct ResponseRedactions {
    redact_json_pointers: Vec<String>,
    reveal_json_pointers: Vec<String>,
}

async fn connect_target(
    host: &str,
    port: u16,
    socks_proxy: Option<&str>,
) -> Result<tokio::net::TcpStream> {
    match socks_proxy {
        Some(proxy) => {
            let addr = proxy.strip_prefix("socks5://").unwrap_or(proxy);
            eprintln!(
                "[tlsn-prove] Connecting to {}:{} via SOCKS5 ({})",
                host, port, addr
            );
            let stream = tokio_socks::tcp::Socks5Stream::connect(addr, (host, port))
                .await
                .map_err(|e| anyhow!("SOCKS5 connect failed: {e}"))?;
            Ok(stream.into_inner())
        }
        None => Ok(tokio::net::TcpStream::connect((host, port)).await?),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let parsed_url = url::Url::parse(&cli.url).map_err(|e| anyhow!("Invalid URL: {e}"))?;
    let host = parsed_url
        .host_str()
        .ok_or_else(|| anyhow!("URL has no host"))?
        .to_string();
    let port = parsed_url
        .port_or_known_default()
        .ok_or_else(|| anyhow!("Cannot determine port"))?;
    let path = if let Some(q) = parsed_url.query() {
        format!("{}?{}", parsed_url.path(), q)
    } else {
        parsed_url.path().to_string()
    };

    eprintln!("[tlsn-prove] Target: {}:{}{}", host, port, path);

    let socks_proxy = cli.socks_proxy.as_deref();

    let mut custom_headers: Vec<(String, String)> = cli
        .headers
        .iter()
        .filter_map(|h| {
            let (k, v) = h.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();
    let mut redact_sent_headers = cli.redact_sent_headers.clone();
    for h in &cli.header_env {
        let (k, env_name) = h
            .split_once(':')
            .ok_or_else(|| anyhow!("Invalid --header-env value, expected \"Key: ENV_NAME\""))?;
        let value = std::env::var(env_name.trim())
            .map_err(|_| anyhow!("Environment variable {} is not set", env_name.trim()))?;
        let header_name = k.trim().to_string();
        redact_sent_headers.push(header_name.clone());
        custom_headers.push((header_name, value));
    }
    let redact_recv_ranges = cli
        .redact_recv_ranges
        .iter()
        .map(|range| range.0.clone())
        .collect::<Vec<_>>();
    let response_redactions = ResponseRedactions {
        redact_json_pointers: cli.redact_response_json.clone(),
        reveal_json_pointers: cli.reveal_response_json.clone(),
    };

    let max_sent = cli.max_sent_data;
    let max_recv = cli.max_recv_data;
    eprintln!(
        "[tlsn-prove] MPC limits: max_sent={}, max_recv={}",
        max_sent, max_recv
    );

    let (attestation, secrets) = if let Some(ref verifier_addr) = cli.verifier {
        if verifier_addr.starts_with("wss://") || verifier_addr.starts_with("ws://") {
            eprintln!("[tlsn-prove] Using WebSocket verifier: {}", verifier_addr);
            run_with_ws_verifier(
                verifier_addr,
                &host,
                port,
                &path,
                socks_proxy,
                &custom_headers,
                &redact_sent_headers,
                &redact_recv_ranges,
                &response_redactions,
                max_sent,
                max_recv,
            )
            .await?
        } else {
            eprintln!("[tlsn-prove] Using TCP verifier: {}", verifier_addr);
            run_with_remote_verifier(
                verifier_addr,
                &host,
                port,
                &path,
                socks_proxy,
                &custom_headers,
                &redact_sent_headers,
                &redact_recv_ranges,
                &response_redactions,
                max_sent,
                max_recv,
            )
            .await?
        }
    } else {
        eprintln!("[tlsn-prove] Using in-process verifier");
        run_with_local_verifier(
            &host,
            port,
            &path,
            socks_proxy,
            &custom_headers,
            &redact_sent_headers,
            &redact_recv_ranges,
            &response_redactions,
            max_sent,
            max_recv,
        )
        .await?
    };

    let presentation = build_presentation(
        attestation,
        secrets,
        &redact_sent_headers,
        &redact_recv_ranges,
        &response_redactions,
    )?;

    let bytes = bincode::serialize(&presentation)?;
    std::fs::write(&cli.output, &bytes)?;

    eprintln!(
        "[tlsn-prove] Presentation saved to {}",
        cli.output.display()
    );
    eprintln!("[tlsn-prove] Size: {} bytes", bytes.len());

    // Also print base64 to stdout for easy piping
    use std::io::Write;
    let b64 = base64_encode(&bytes);
    std::io::stdout().write_all(b64.as_bytes())?;
    std::io::stdout().write_all(b"\n")?;

    Ok(())
}

async fn run_with_local_verifier(
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<(Attestation, Secrets)> {
    let (verifier_socket, prover_socket) = tokio::io::duplex(1 << 23);
    let (request_tx, request_rx) = oneshot::channel::<AttestationRequest>();
    let (attestation_tx, attestation_rx) = oneshot::channel::<Attestation>();

    let host_clone = host.to_string();
    tokio::spawn(async move {
        if let Err(e) = run_verifier(verifier_socket, request_rx, attestation_tx, &host_clone).await
        {
            eprintln!("[tlsn-prove] Verifier error: {e:#}");
        }
    });

    run_prover(
        prover_socket,
        request_tx,
        attestation_rx,
        host,
        port,
        path,
        socks_proxy,
        custom_headers,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
        max_sent_data,
        max_recv_data,
    )
    .await
}

async fn run_with_ws_verifier(
    verifier_url: &str,
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<(Attestation, Secrets)> {
    use async_tungstenite::tokio::connect_async;
    use async_tungstenite::tungstenite::Message;
    use futures::StreamExt;

    let session_ws_url = format!("{}/session", verifier_url);
    eprintln!(
        "[tlsn-prove] Connecting to session endpoint: {}",
        session_ws_url
    );

    let (mut session_ws, _) = connect_async(&session_ws_url)
        .await
        .map_err(|e| anyhow!("Failed to connect to session WS: {e}"))?;

    let register_msg = serde_json::json!({
        "type": "register",
        "maxRecvData": max_recv_data,
        "maxSentData": max_sent_data,
        "sessionData": {}
    });
    session_ws
        .send(Message::Text(register_msg.to_string().into()))
        .await?;

    let resp = session_ws
        .next()
        .await
        .ok_or_else(|| anyhow!("Session WS closed"))??;
    let resp_text = resp.into_text()?;
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)?;
    let session_id = resp_json["sessionId"]
        .as_str()
        .ok_or_else(|| anyhow!("No sessionId in response: {}", resp_text))?;
    eprintln!("[tlsn-prove] Session registered: {}", session_id);

    let verifier_ws_url = format!("{}/verifier?sessionId={}", verifier_url, session_id);
    eprintln!("[tlsn-prove] Connecting to verifier: {}", verifier_ws_url);

    let (verifier_ws, _) = connect_async(&verifier_ws_url)
        .await
        .map_err(|e| anyhow!("Failed to connect to verifier WS: {e}"))?;

    let ws_stream = ws_stream_tungstenite::WsStream::new(verifier_ws);

    let prover_output = run_prover_mpc_futures_stream(
        ws_stream,
        host,
        port,
        path,
        socks_proxy,
        custom_headers,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
        max_sent_data,
        max_recv_data,
    )
    .await?;

    eprintln!("[tlsn-prove] MPC complete, waiting for session result...");

    let resp = session_ws
        .next()
        .await
        .ok_or_else(|| anyhow!("Session WS closed before completion"))??;
    let resp_text = resp.into_text()?;
    let resp_json: serde_json::Value = serde_json::from_str(&resp_text)?;

    let resp_type = resp_json["type"].as_str().unwrap_or("");
    eprintln!("[tlsn-prove] Session response type: {}", resp_type);

    if resp_type == "error" {
        return Err(anyhow!(
            "Verifier error: {}",
            resp_json["message"].as_str().unwrap_or("unknown")
        ));
    }

    if resp_type != "session_completed" {
        return Err(anyhow!("Unexpected response type: {}", resp_type));
    }

    let has_verifier_data = resp_json["connectionInfo"].is_string()
        && resp_json["serverEphemeralKey"].is_string()
        && resp_json["transcriptCommitments"].is_string();

    let (attestation, secrets) = if has_verifier_data {
        eprintln!("[tlsn-prove] Building attestation from server-provided verifier data");
        build_attestation_from_server_data(prover_output, &resp_json)?
    } else {
        return Err(anyhow!(
            "Server did not return verifier data. Use a self-hosted Verifier Server."
        ));
    };

    session_ws.close(None).await.ok();

    Ok((attestation, secrets))
}

/// Build attestation using verifier data returned from a self-hosted server via WS.
fn build_attestation_from_server_data(
    output: ProverMpcOutput,
    resp: &serde_json::Value,
) -> Result<(Attestation, Secrets)> {
    let conn_info_bytes = base64_decode(resp["connectionInfo"].as_str().unwrap_or(""))?;
    let eph_key_bytes = base64_decode(resp["serverEphemeralKey"].as_str().unwrap_or(""))?;
    let commitments_bytes = base64_decode(resp["transcriptCommitments"].as_str().unwrap_or(""))?;

    let connection_info: ConnectionInfo = bincode::deserialize(&conn_info_bytes)?;
    let server_ephemeral_key = bincode::deserialize(&eph_key_bytes)?;
    let transcript_commitments = bincode::deserialize(&commitments_bytes)?;

    let signing_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
    let signer = Box::new(Secp256k1Signer::new(&signing_key.to_bytes())?);
    let mut provider = CryptoProvider::default();
    provider.signer.set_signer(signer);

    let att_config = AttestationConfig::builder()
        .supported_signature_algs(Vec::from_iter(provider.signer.supported_algs()))
        .build()?;

    let mut builder = Attestation::builder(&att_config).accept_request(output.request)?;
    builder
        .connection_info(connection_info)
        .server_ephemeral_key(server_ephemeral_key)
        .transcript_commitments(transcript_commitments);

    let attestation = builder.build(&provider)?;

    Ok((attestation, output.secrets))
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    const TABLE: [u8; 128] = {
        let mut t = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            t[chars[i] as usize] = i as u8;
            i += 1;
        }
        t
    };
    let bytes: Vec<u8> = s
        .bytes()
        .filter(|&b| b != b'=' && b != b'\n' && b != b'\r')
        .collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut buf = [0u32; 4];
        for (i, &b) in chunk.iter().enumerate() {
            buf[i] = TABLE.get(b as usize).copied().unwrap_or(0) as u32;
        }
        let triple = (buf[0] << 18) | (buf[1] << 12) | (buf[2] << 6) | buf[3];
        out.push((triple >> 16) as u8);
        if chunk.len() > 2 {
            out.push((triple >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

/// Run the MPC-TLS prover over a futures AsyncRead+AsyncWrite stream (for WebSocket).
async fn run_prover_mpc_futures_stream<
    S: futures::AsyncRead + futures::AsyncWrite + Send + Unpin + 'static,
>(
    stream: S,
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<ProverMpcOutput> {
    let session = Session::new(stream);
    let (driver, mut handle) = session.split();
    let driver_task = tokio::spawn(driver);

    let prover = handle
        .new_prover(ProverConfig::builder().build()?)?
        .commit(
            TlsCommitConfig::builder()
                .protocol(
                    MpcTlsConfig::builder()
                        .max_sent_data(max_sent_data)
                        .max_recv_data(max_recv_data)
                        .build()?,
                )
                .build()?,
        )
        .await?;

    let target_tcp = connect_target(host, port, socks_proxy).await?;
    eprintln!("[tlsn-prove] Connected to {}:{}", host, port);

    let (tls_connection, prover_fut) = prover
        .connect(
            TlsClientConfig::builder()
                .server_name(ServerName::Dns(host.try_into()?))
                .root_store(tlsn::webpki::RootCertStore::mozilla())
                .build()?,
            target_tcp.compat(),
        )
        .await?;
    let tls_connection = TokioIo::new(tls_connection.compat());
    let prover_task = tokio::spawn(prover_fut);

    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(tls_connection).await?;
    tokio::spawn(connection);

    let mut request_builder = Request::builder()
        .uri(path)
        .header("Host", host)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .header("User-Agent", "tlsn-curl-prover/0.1.0");
    for (k, v) in custom_headers {
        request_builder = request_builder.header(k.as_str(), v.as_str());
    }
    let request = request_builder.body(Empty::<Bytes>::new())?;

    eprintln!("[tlsn-prove] Sending HTTP request...");
    let response = request_sender.send_request(request).await?;
    eprintln!("[tlsn-prove] Response status: {}", response.status());

    let mut prover = prover_task.await??;

    let transcript = prover.transcript();
    let transcript_commit = build_transcript_commit_config(
        transcript,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
    )?;

    let mut req_builder = RequestConfig::builder();
    req_builder.transcript_commit(transcript_commit);
    let request_config = req_builder.build()?;

    let mut prove_builder = ProveConfig::builder(prover.transcript());
    if let Some(tc) = request_config.transcript_commit() {
        prove_builder.transcript_commit(tc.clone());
    }
    let disclosure_config = prove_builder.build()?;

    let ProverOutput {
        transcript_commitments,
        transcript_secrets,
        ..
    } = prover.prove(&disclosure_config).await?;

    let transcript = prover.transcript().clone();
    let tls_transcript = prover.tls_transcript().clone();
    prover.close().await?;

    let mut att_builder = AttestationRequest::builder(&request_config);
    att_builder
        .server_name(ServerName::Dns(host.try_into()?))
        .handshake_data(HandshakeData {
            certs: tls_transcript
                .server_cert_chain()
                .expect("cert chain")
                .to_vec(),
            sig: tls_transcript
                .server_signature()
                .expect("signature")
                .clone(),
            binding: tls_transcript.certificate_binding().clone(),
        })
        .transcript(transcript)
        .transcript_commitments(transcript_secrets, transcript_commitments);

    let (request, secrets) = att_builder.build(&CryptoProvider::default())?;

    handle.close();
    driver_task.await??;

    Ok(ProverMpcOutput { request, secrets })
}

/// Run the MPC-TLS prover over any tokio AsyncRead+AsyncWrite stream (for TCP).
#[allow(dead_code)]
async fn run_prover_mpc_stream<
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
>(
    stream: S,
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<ProverMpcOutput> {
    use tokio_util::compat::TokioAsyncReadCompatExt;

    let session = Session::new(stream.compat());
    let (driver, mut handle) = session.split();
    let driver_task = tokio::spawn(driver);

    let prover = handle
        .new_prover(ProverConfig::builder().build()?)?
        .commit(
            TlsCommitConfig::builder()
                .protocol(
                    MpcTlsConfig::builder()
                        .max_sent_data(max_sent_data)
                        .max_recv_data(max_recv_data)
                        .build()?,
                )
                .build()?,
        )
        .await?;

    let target_tcp = connect_target(host, port, socks_proxy).await?;
    eprintln!("[tlsn-prove] Connected to {}:{}", host, port);

    let (tls_connection, prover_fut) = prover
        .connect(
            TlsClientConfig::builder()
                .server_name(ServerName::Dns(host.try_into()?))
                .root_store(tlsn::webpki::RootCertStore::mozilla())
                .build()?,
            target_tcp.compat(),
        )
        .await?;
    let tls_connection = TokioIo::new(tls_connection.compat());
    let prover_task = tokio::spawn(prover_fut);

    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(tls_connection).await?;
    tokio::spawn(connection);

    let mut request_builder = Request::builder()
        .uri(path)
        .header("Host", host)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .header("User-Agent", "tlsn-curl-prover/0.1.0");
    for (k, v) in custom_headers {
        request_builder = request_builder.header(k.as_str(), v.as_str());
    }
    let request = request_builder.body(Empty::<Bytes>::new())?;

    eprintln!("[tlsn-prove] Sending HTTP request...");
    let response = request_sender.send_request(request).await?;
    eprintln!("[tlsn-prove] Response status: {}", response.status());

    let mut prover = prover_task.await??;

    let transcript = prover.transcript();
    let transcript_commit = build_transcript_commit_config(
        transcript,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
    )?;

    let mut req_builder = RequestConfig::builder();
    req_builder.transcript_commit(transcript_commit);
    let request_config = req_builder.build()?;

    let mut prove_builder = ProveConfig::builder(prover.transcript());
    if let Some(tc) = request_config.transcript_commit() {
        prove_builder.transcript_commit(tc.clone());
    }
    let disclosure_config = prove_builder.build()?;

    let ProverOutput {
        transcript_commitments,
        transcript_secrets,
        ..
    } = prover.prove(&disclosure_config).await?;

    let transcript = prover.transcript().clone();
    let tls_transcript = prover.tls_transcript().clone();
    prover.close().await?;

    let mut att_builder = AttestationRequest::builder(&request_config);
    att_builder
        .server_name(ServerName::Dns(host.try_into()?))
        .handshake_data(HandshakeData {
            certs: tls_transcript
                .server_cert_chain()
                .expect("cert chain")
                .to_vec(),
            sig: tls_transcript
                .server_signature()
                .expect("signature")
                .clone(),
            binding: tls_transcript.certificate_binding().clone(),
        })
        .transcript(transcript)
        .transcript_commitments(transcript_secrets, transcript_commitments);

    let (request, secrets) = att_builder.build(&CryptoProvider::default())?;

    handle.close();
    driver_task.await??;

    Ok(ProverMpcOutput { request, secrets })
}

async fn run_with_remote_verifier(
    verifier_addr: &str,
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<(Attestation, Secrets)> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let session_id: [u8; 16] = rand::random();
    let sid_hex = hex::encode(&session_id[..8]);
    eprintln!("[tlsn-prove] Session ID: {}", sid_hex);

    // Two-connection protocol: 'M' opens MPC, 'A' exchanges attestation after MPC closes.
    let mut mpc_tcp = tokio::net::TcpStream::connect(verifier_addr).await?;
    mpc_tcp.write_all(&[b'M']).await?;
    mpc_tcp.write_all(&session_id).await?;
    mpc_tcp.flush().await?;

    eprintln!("[tlsn-prove] MPC connection established");

    let prover_output = run_prover_mpc(
        mpc_tcp,
        host,
        port,
        path,
        socks_proxy,
        custom_headers,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
        max_sent_data,
        max_recv_data,
    )
    .await?;

    eprintln!("[tlsn-prove] MPC complete, requesting attestation...");

    let mut att_tcp = tokio::net::TcpStream::connect(verifier_addr).await?;
    att_tcp.write_all(&[b'A']).await?;
    att_tcp.write_all(&session_id).await?;

    let req_bytes = bincode::serialize(&prover_output.request)?;
    att_tcp
        .write_all(&(req_bytes.len() as u32).to_be_bytes())
        .await?;
    att_tcp.write_all(&req_bytes).await?;
    att_tcp.flush().await?;

    let mut len_buf = [0u8; 4];
    att_tcp.read_exact(&mut len_buf).await?;
    let att_len = u32::from_be_bytes(len_buf) as usize;
    let mut att_buf = vec![0u8; att_len];
    att_tcp.read_exact(&mut att_buf).await?;
    let attestation: Attestation = bincode::deserialize(&att_buf)?;

    let provider = CryptoProvider::default();
    prover_output.request.validate(&attestation, &provider)?;

    eprintln!("[tlsn-prove] Attestation received and validated");

    Ok((attestation, prover_output.secrets))
}

/// Output of the prover MPC phase (before attestation exchange).
struct ProverMpcOutput {
    request: AttestationRequest,
    secrets: Secrets,
}

async fn run_prover_mpc(
    tcp: tokio::net::TcpStream,
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<ProverMpcOutput> {
    let session = Session::new(tcp.compat());
    let (driver, mut handle) = session.split();
    let driver_task = tokio::spawn(driver);

    let prover = handle
        .new_prover(ProverConfig::builder().build()?)?
        .commit(
            TlsCommitConfig::builder()
                .protocol(
                    MpcTlsConfig::builder()
                        .max_sent_data(max_sent_data)
                        .max_recv_data(max_recv_data)
                        .build()?,
                )
                .build()?,
        )
        .await?;

    let target_tcp = connect_target(host, port, socks_proxy).await?;
    eprintln!("[tlsn-prove] Connected to {}:{}", host, port);

    let (tls_connection, prover_fut) = prover
        .connect(
            TlsClientConfig::builder()
                .server_name(ServerName::Dns(host.try_into()?))
                .root_store(tlsn::webpki::RootCertStore::mozilla())
                .build()?,
            target_tcp.compat(),
        )
        .await?;
    let tls_connection = TokioIo::new(tls_connection.compat());
    let prover_task = tokio::spawn(prover_fut);

    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(tls_connection).await?;
    tokio::spawn(connection);

    let mut request_builder = Request::builder()
        .uri(path)
        .header("Host", host)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .header("User-Agent", "tlsn-curl-prover/0.1.0");
    for (k, v) in custom_headers {
        request_builder = request_builder.header(k.as_str(), v.as_str());
    }
    let request = request_builder.body(Empty::<Bytes>::new())?;

    eprintln!("[tlsn-prove] Sending HTTP request...");
    let response = request_sender.send_request(request).await?;
    eprintln!("[tlsn-prove] Response status: {}", response.status());

    let mut prover = prover_task.await??;

    let transcript = prover.transcript();
    let transcript_commit = build_transcript_commit_config(
        transcript,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
    )?;

    let mut req_builder = RequestConfig::builder();
    req_builder.transcript_commit(transcript_commit);
    let request_config = req_builder.build()?;

    let mut prove_builder = ProveConfig::builder(prover.transcript());
    if let Some(tc) = request_config.transcript_commit() {
        prove_builder.transcript_commit(tc.clone());
    }
    let disclosure_config = prove_builder.build()?;

    let ProverOutput {
        transcript_commitments,
        transcript_secrets,
        ..
    } = prover.prove(&disclosure_config).await?;

    let transcript = prover.transcript().clone();
    let tls_transcript = prover.tls_transcript().clone();
    prover.close().await?;

    let mut att_builder = AttestationRequest::builder(&request_config);
    att_builder
        .server_name(ServerName::Dns(host.try_into()?))
        .handshake_data(HandshakeData {
            certs: tls_transcript
                .server_cert_chain()
                .expect("cert chain")
                .to_vec(),
            sig: tls_transcript
                .server_signature()
                .expect("signature")
                .clone(),
            binding: tls_transcript.certificate_binding().clone(),
        })
        .transcript(transcript)
        .transcript_commitments(transcript_secrets, transcript_commitments);

    let (request, secrets) = att_builder.build(&CryptoProvider::default())?;

    handle.close();
    driver_task.await??;

    Ok(ProverMpcOutput { request, secrets })
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

async fn run_prover<S: AsyncWrite + AsyncRead + Send + Sync + Unpin + 'static>(
    socket: S,
    req_tx: oneshot::Sender<AttestationRequest>,
    resp_rx: oneshot::Receiver<Attestation>,
    host: &str,
    port: u16,
    path: &str,
    socks_proxy: Option<&str>,
    custom_headers: &[(String, String)],
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
    max_sent_data: usize,
    max_recv_data: usize,
) -> Result<(Attestation, Secrets)> {
    let session = Session::new(socket.compat());
    let (driver, mut handle) = session.split();
    let driver_task = tokio::spawn(driver);

    let prover = handle
        .new_prover(ProverConfig::builder().build()?)?
        .commit(
            TlsCommitConfig::builder()
                .protocol(
                    MpcTlsConfig::builder()
                        .max_sent_data(max_sent_data)
                        .max_recv_data(max_recv_data)
                        .build()?,
                )
                .build()?,
        )
        .await?;

    let tcp = connect_target(host, port, socks_proxy).await?;
    eprintln!("[tlsn-prove] Connected to {}:{}", host, port);

    let (tls_connection, prover_fut) = prover
        .connect(
            TlsClientConfig::builder()
                .server_name(ServerName::Dns(host.try_into()?))
                .root_store(tlsn::webpki::RootCertStore::mozilla())
                .build()?,
            tcp.compat(),
        )
        .await?;
    let tls_connection = TokioIo::new(tls_connection.compat());
    let prover_task = tokio::spawn(prover_fut);

    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(tls_connection).await?;
    tokio::spawn(connection);

    let mut request_builder = Request::builder()
        .uri(path)
        .header("Host", host)
        .header("Accept", "application/json")
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .header("User-Agent", "tlsn-curl-prover/0.1.0");
    for (k, v) in custom_headers {
        request_builder = request_builder.header(k.as_str(), v.as_str());
    }
    let request = request_builder.body(Empty::<Bytes>::new())?;

    eprintln!("[tlsn-prove] Sending HTTP request...");
    let response = request_sender.send_request(request).await?;
    eprintln!("[tlsn-prove] Response status: {}", response.status());

    let prover = prover_task.await??;

    let transcript = prover.transcript();
    let transcript_commit = build_transcript_commit_config(
        transcript,
        redact_sent_headers,
        redact_recv_ranges,
        response_redactions,
    )?;

    let mut builder = RequestConfig::builder();
    builder.transcript_commit(transcript_commit);
    let request_config = builder.build()?;

    let (attestation, secrets) =
        attestation_protocol(prover, &request_config, host, req_tx, resp_rx).await?;

    handle.close();
    driver_task.await??;

    Ok((attestation, secrets))
}

async fn attestation_protocol(
    mut prover: Prover<Committed>,
    config: &RequestConfig,
    host: &str,
    request_tx: oneshot::Sender<AttestationRequest>,
    attestation_rx: oneshot::Receiver<Attestation>,
) -> Result<(Attestation, Secrets)> {
    let mut builder = ProveConfig::builder(prover.transcript());
    if let Some(config) = config.transcript_commit() {
        builder.transcript_commit(config.clone());
    }
    let disclosure_config = builder.build()?;

    let ProverOutput {
        transcript_commitments,
        transcript_secrets,
        ..
    } = prover.prove(&disclosure_config).await?;

    let transcript = prover.transcript().clone();
    let tls_transcript = prover.tls_transcript().clone();
    prover.close().await?;

    let mut builder = AttestationRequest::builder(config);
    builder
        .server_name(ServerName::Dns(host.try_into()?))
        .handshake_data(HandshakeData {
            certs: tls_transcript
                .server_cert_chain()
                .expect("server cert chain")
                .to_vec(),
            sig: tls_transcript
                .server_signature()
                .expect("server signature")
                .clone(),
            binding: tls_transcript.certificate_binding().clone(),
        })
        .transcript(transcript)
        .transcript_commitments(transcript_secrets, transcript_commitments);

    let (request, secrets) = builder.build(&CryptoProvider::default())?;

    request_tx
        .send(request.clone())
        .map_err(|_| anyhow!("verifier not receiving"))?;

    let attestation = attestation_rx
        .await
        .map_err(|e| anyhow!("verifier did not respond: {e}"))?;

    let provider = CryptoProvider::default();
    request.validate(&attestation, &provider)?;

    eprintln!("[tlsn-prove] Attestation received and validated");

    Ok((attestation, secrets))
}

async fn run_verifier<S: AsyncWrite + AsyncRead + Send + Sync + Unpin + 'static>(
    socket: S,
    request_rx: oneshot::Receiver<AttestationRequest>,
    attestation_tx: oneshot::Sender<Attestation>,
    _host: &str,
) -> Result<()> {
    let session = Session::new(socket.compat());
    let (driver, mut handle) = session.split();
    let driver_task = tokio::spawn(driver);

    let verifier = handle
        .new_verifier(
            VerifierConfig::builder()
                .root_store(tlsn::webpki::RootCertStore::mozilla())
                .build()?,
        )?
        .commit()
        .await?
        .accept()
        .await?
        .run()
        .await?;

    let (
        VerifierOutput {
            transcript_commitments,
            ..
        },
        verifier,
    ) = verifier.verify().await?.accept().await?;

    let tls_transcript = verifier.tls_transcript().clone();
    verifier.close().await?;

    let sent_len = tls_transcript
        .sent()
        .iter()
        .filter_map(|r| {
            if let ContentType::ApplicationData = r.typ {
                Some(r.ciphertext.len())
            } else {
                None
            }
        })
        .sum::<usize>();
    let recv_len = tls_transcript
        .recv()
        .iter()
        .filter_map(|r| {
            if let ContentType::ApplicationData = r.typ {
                Some(r.ciphertext.len())
            } else {
                None
            }
        })
        .sum::<usize>();

    let request = request_rx.await?;

    let signing_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
    let signer = Box::new(Secp256k1Signer::new(&signing_key.to_bytes())?);
    let mut provider = CryptoProvider::default();
    provider.signer.set_signer(signer);

    let att_config = AttestationConfig::builder()
        .supported_signature_algs(Vec::from_iter(provider.signer.supported_algs()))
        .build()?;

    let mut builder = Attestation::builder(&att_config).accept_request(request)?;
    builder
        .connection_info(ConnectionInfo {
            time: tls_transcript.time(),
            version: *tls_transcript.version(),
            transcript_length: TranscriptLength {
                sent: sent_len as u32,
                received: recv_len as u32,
            },
        })
        .server_ephemeral_key(tls_transcript.server_ephemeral_key().clone())
        .transcript_commitments(transcript_commitments);

    let attestation = builder.build(&provider)?;

    attestation_tx
        .send(attestation)
        .map_err(|_| anyhow!("prover not receiving attestation"))?;

    handle.close();
    driver_task.await??;

    Ok(())
}

/// Parse the sent HTTP request to find byte ranges of header values that should be redacted.
/// Returns a list of (start, end) byte ranges to EXCLUDE from reveal.
fn find_header_value_ranges(
    sent_bytes: &[u8],
    redact_names: &[String],
) -> Vec<std::ops::Range<usize>> {
    if redact_names.is_empty() {
        return vec![];
    }

    let sent_str = String::from_utf8_lossy(sent_bytes);
    let mut redact_ranges = Vec::new();

    // Headers are between the first \r\n (after request line) and \r\n\r\n
    let header_start = match sent_str.find("\r\n") {
        Some(pos) => pos + 2,
        None => return vec![],
    };
    let header_end = match sent_str.find("\r\n\r\n") {
        Some(pos) => pos,
        None => sent_bytes.len(),
    };

    let header_section = &sent_str[header_start..header_end];
    let mut offset = header_start;

    for line in header_section.split("\r\n") {
        if let Some(colon_pos) = line.find(':') {
            let name = line[..colon_pos].trim();
            let value_start_in_line = colon_pos + 1;
            let value_with_space = &line[value_start_in_line..];
            let trimmed_start = value_with_space.len() - value_with_space.trim_start().len();
            let value_byte_start = offset + value_start_in_line + trimmed_start;
            let value_byte_end = offset + line.len();

            if redact_names.iter().any(|r| r.eq_ignore_ascii_case(name)) {
                if value_byte_start < value_byte_end {
                    redact_ranges.push(value_byte_start..value_byte_end);
                    eprintln!(
                        "[tlsn-prove] Redacting header: {} (bytes {}..{})",
                        name, value_byte_start, value_byte_end
                    );
                }
            }
        }
        offset += line.len() + 2; // +2 for \r\n
    }

    redact_ranges
}

fn sent_reveal_ranges(sent_bytes: &[u8], redact_names: &[String]) -> (Vec<Range<usize>>, usize) {
    let sent_len = sent_bytes.len();
    if sent_len == 0 {
        return (vec![], 0);
    }

    let redact_ranges = find_header_value_ranges(sent_bytes, redact_names);
    let redacted_count = redact_ranges.len();
    let reveal_ranges = if redact_ranges.is_empty() {
        vec![0..sent_len]
    } else {
        subtract_ranges(sent_len, &redact_ranges)
            .into_iter()
            .filter(|range| range.start < range.end)
            .collect()
    };

    (reveal_ranges, redacted_count)
}

fn build_transcript_commit_config(
    transcript: &Transcript,
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
) -> Result<TranscriptCommitConfig> {
    let mut builder = TranscriptCommitConfig::builder(transcript);
    let (sent_ranges, redacted_count) = sent_reveal_ranges(transcript.sent(), redact_sent_headers);
    for range in &sent_ranges {
        builder.commit_sent(range)?;
    }

    let effective_recv_redactions = effective_recv_redact_ranges(
        transcript.received(),
        redact_recv_ranges,
        response_redactions,
    )?;
    let recv_len = transcript.received().len();
    let recv_ranges = recv_reveal_ranges(recv_len, &effective_recv_redactions)?;
    for range in &recv_ranges {
        builder.commit_recv(range)?;
    }

    if redacted_count > 0 {
        eprintln!(
            "[tlsn-prove] Selective disclosure commitments: {} byte range(s) redacted from sent data",
            redacted_count
        );
    }
    if !effective_recv_redactions.is_empty() {
        eprintln!(
            "[tlsn-prove] Selective disclosure commitments: {} byte range(s) redacted from received data",
            effective_recv_redactions.len()
        );
    }

    Ok(builder.build()?)
}

fn effective_recv_redact_ranges(
    recv_bytes: &[u8],
    explicit_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
) -> Result<Vec<Range<usize>>> {
    if !response_redactions.reveal_json_pointers.is_empty() {
        if !explicit_ranges.is_empty() || !response_redactions.redact_json_pointers.is_empty() {
            return Err(anyhow!(
                "--reveal-response-json cannot be combined with received redaction ranges or --redact-response-json"
            ));
        }
        let reveal_ranges =
            resolve_response_json_ranges(recv_bytes, &response_redactions.reveal_json_pointers)?;
        return Ok(subtract_ranges(recv_bytes.len(), &reveal_ranges));
    }

    let mut ranges = explicit_ranges.to_vec();
    ranges.extend(resolve_response_json_ranges(
        recv_bytes,
        &response_redactions.redact_json_pointers,
    )?);
    Ok(ranges)
}

fn resolve_response_json_ranges(
    recv_bytes: &[u8],
    pointers: &[String],
) -> Result<Vec<Range<usize>>> {
    if pointers.is_empty() {
        return Ok(vec![]);
    }

    let (body_start, body) = response_body(recv_bytes)?;
    let body_text = std::str::from_utf8(body)
        .map_err(|_| anyhow!("structured response redaction requires a UTF-8 response body"))?;
    let json: serde_json::Value = serde_json::from_str(body_text)
        .map_err(|e| anyhow!("structured response redaction requires a JSON response body: {e}"))?;

    let mut ranges = Vec::new();
    for pointer in pointers {
        if !pointer.starts_with('/') {
            return Err(anyhow!("JSON Pointer must start with '/': {}", pointer));
        }
        let value = json
            .pointer(pointer)
            .ok_or_else(|| anyhow!("JSON Pointer not found in response body: {}", pointer))?;
        let raw_value = serde_json::to_string(value)?;
        let matches = find_unique_matches(body_text, &raw_value);
        if matches.len() != 1 {
            return Err(anyhow!(
                "JSON Pointer {} resolved to a value that appears {} times; refusing ambiguous redaction",
                pointer,
                matches.len()
            ));
        }
        let start = body_start + matches[0];
        ranges.push(start..start + raw_value.len());
    }

    Ok(ranges)
}

fn response_body(recv_bytes: &[u8]) -> Result<(usize, &[u8])> {
    let header_end = recv_bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("structured response redaction requires HTTP response headers"))?;
    let headers = String::from_utf8_lossy(&recv_bytes[..header_end]).to_lowercase();
    if headers.contains("transfer-encoding:") && headers.contains("chunked") {
        return Err(anyhow!(
            "structured response redaction does not support chunked responses"
        ));
    }

    let body_start = header_end + 4;
    Ok((body_start, &recv_bytes[body_start..]))
}

fn find_unique_matches(haystack: &str, needle: &str) -> Vec<usize> {
    let mut matches = Vec::new();
    let mut offset = 0;
    while let Some(index) = haystack[offset..].find(needle) {
        let absolute = offset + index;
        matches.push(absolute);
        offset = absolute + needle.len();
    }
    matches
}

fn recv_reveal_ranges(total_len: usize, redact: &[Range<usize>]) -> Result<Vec<Range<usize>>> {
    for range in redact {
        if range.start >= range.end {
            return Err(anyhow!(
                "invalid received redaction range {}:{}",
                range.start,
                range.end
            ));
        }
        if range.end > total_len {
            return Err(anyhow!(
                "received redaction range {}:{} exceeds received transcript length {}",
                range.start,
                range.end,
                total_len
            ));
        }
    }

    Ok(subtract_ranges(total_len, redact)
        .into_iter()
        .filter(|range| range.start < range.end)
        .collect())
}

/// Compute reveal ranges by subtracting redact ranges from [0..total_len].
fn subtract_ranges(total_len: usize, redact: &[Range<usize>]) -> Vec<Range<usize>> {
    if redact.is_empty() {
        return vec![0..total_len];
    }

    let mut sorted = redact.to_vec();
    sorted.sort_by_key(|r| r.start);

    let mut result = Vec::new();
    let mut cursor = 0;

    for range in &sorted {
        if cursor < range.start {
            result.push(cursor..range.start);
        }
        cursor = range.end.max(cursor);
    }

    if cursor < total_len {
        result.push(cursor..total_len);
    }

    result
}

fn build_presentation(
    attestation: Attestation,
    secrets: Secrets,
    redact_sent_headers: &[String],
    redact_recv_ranges: &[Range<usize>],
    response_redactions: &ResponseRedactions,
) -> Result<Presentation> {
    let transcript = secrets.transcript();
    let sent_bytes = transcript.sent();
    let recv_len = transcript.received().len();

    let mut builder = secrets.transcript_proof_builder();

    let (sent_ranges, redacted_count) = sent_reveal_ranges(sent_bytes, redact_sent_headers);
    for range in &sent_ranges {
        builder.reveal_sent(range)?;
    }
    if redacted_count > 0 {
        eprintln!(
            "[tlsn-prove] Selective disclosure: {} byte range(s) redacted from sent data",
            redacted_count
        );
    }

    let effective_recv_redactions = effective_recv_redact_ranges(
        transcript.received(),
        redact_recv_ranges,
        response_redactions,
    )?;
    let recv_ranges = recv_reveal_ranges(recv_len, &effective_recv_redactions)?;
    for range in &recv_ranges {
        builder.reveal_recv(range)?;
    }
    if !effective_recv_redactions.is_empty() {
        eprintln!(
            "[tlsn-prove] Selective disclosure: {} byte range(s) redacted from received data",
            effective_recv_redactions.len()
        );
    }

    let transcript_proof = builder.build()?;

    let provider = CryptoProvider::default();
    let mut builder = attestation.presentation_builder(&provider);
    builder
        .identity_proof(secrets.identity_proof())
        .transcript_proof(transcript_proof);

    let presentation = builder.build()?;
    Ok(presentation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_sent_request() -> Vec<u8> {
        b"GET /v1/me HTTP/1.1\r\n\
Host: api.example.test\r\n\
Authorization: Bearer secret-token\r\n\
X-Trace: public-id\r\n\
\r\n"
            .to_vec()
    }

    #[test]
    fn sent_reveal_ranges_omit_redacted_header_values() {
        let sent = sample_sent_request();
        let redact_headers = vec!["authorization".to_string()];
        let (ranges, redacted_count) = sent_reveal_ranges(&sent, &redact_headers);

        assert_eq!(redacted_count, 1);
        let mut revealed = Vec::new();
        for range in ranges {
            revealed.extend_from_slice(&sent[range]);
        }
        let revealed = String::from_utf8(revealed).unwrap();

        assert!(revealed.contains("Authorization: "));
        assert!(!revealed.contains("Bearer secret-token"));
        assert!(revealed.contains("X-Trace: public-id"));
    }

    #[test]
    fn parse_byte_range_accepts_valid_range() {
        let range = parse_byte_range("12:34").unwrap();

        assert_eq!(range.0, 12..34);
    }

    #[test]
    fn parse_byte_range_rejects_malformed_input() {
        assert!(parse_byte_range("12-34").unwrap_err().contains("start:end"));
    }

    #[test]
    fn parse_byte_range_rejects_non_numeric_bounds() {
        assert!(parse_byte_range("x:34").unwrap_err().contains("start"));
        assert!(parse_byte_range("12:y").unwrap_err().contains("end"));
    }

    #[test]
    fn parse_byte_range_rejects_empty_bounds() {
        assert!(parse_byte_range(":34").unwrap_err().contains("start"));
        assert!(parse_byte_range("12:").unwrap_err().contains("end"));
    }

    #[test]
    fn parse_byte_range_rejects_start_greater_than_or_equal_to_end() {
        assert!(parse_byte_range("34:34")
            .unwrap_err()
            .contains("start < end"));
        assert!(parse_byte_range("35:34")
            .unwrap_err()
            .contains("start < end"));
    }

    #[test]
    fn find_header_value_ranges_matches_repeated_headers_case_insensitively() {
        let sent = b"GET / HTTP/1.1\r\n\
x-token: first\r\n\
X-Token: second\r\n\
\r\n";
        let ranges = find_header_value_ranges(sent, &["X-TOKEN".to_string()]);
        let values = ranges
            .iter()
            .map(|range| std::str::from_utf8(&sent[range.clone()]).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(values, vec!["first", "second"]);
    }

    #[test]
    fn find_header_value_ranges_ignores_missing_headers() {
        let sent = sample_sent_request();

        assert!(find_header_value_ranges(&sent, &["cookie".to_string()]).is_empty());
    }

    #[test]
    fn find_header_value_ranges_keeps_colons_inside_values() {
        let sent = b"GET / HTTP/1.1\r\n\
X-Signed: scheme part:with:colons\r\n\
\r\n";
        let ranges = find_header_value_ranges(sent, &["x-signed".to_string()]);

        assert_eq!(
            std::str::from_utf8(&sent[ranges[0].clone()]).unwrap(),
            "scheme part:with:colons"
        );
    }

    #[test]
    fn transcript_commit_config_uses_selective_sent_ranges() {
        let sent = sample_sent_request();
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        let transcript = Transcript::new(sent, recv);
        let redact_headers = vec!["Authorization".to_string()];

        build_transcript_commit_config(
            &transcript,
            &redact_headers,
            &[],
            &ResponseRedactions::default(),
        )
        .unwrap();
    }

    #[test]
    fn recv_reveal_ranges_omit_redacted_bytes() {
        let ranges = recv_reveal_ranges(10, &[2..5, 7..9]).unwrap();

        assert_eq!(ranges, vec![0..2, 5..7, 9..10]);
    }

    #[test]
    fn recv_reveal_ranges_reject_out_of_bounds_ranges() {
        let error = recv_reveal_ranges(10, &[8..11]).unwrap_err();

        assert!(error
            .to_string()
            .contains("exceeds received transcript length"),);
    }

    #[test]
    fn response_json_redaction_resolves_unique_value_range() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 43\r\n\r\n{\"data\":{\"amount\":\"123.45\",\"base\":\"BTC\"}}";
        let ranges = resolve_response_json_ranges(recv, &["/data/amount".to_string()]).unwrap();

        assert_eq!(&recv[ranges[0].clone()], b"\"123.45\"");
    }

    #[test]
    fn response_json_redaction_rejects_duplicate_values() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 31\r\n\r\n{\"a\":\"same\",\"b\":\"same\"}";
        let error = resolve_response_json_ranges(recv, &["/a".to_string()]).unwrap_err();

        assert!(error.to_string().contains("ambiguous redaction"));
    }

    #[test]
    fn response_json_redaction_rejects_missing_http_headers() {
        let recv = b"{\"usd\":64000}";
        let error = resolve_response_json_ranges(recv, &["/usd".to_string()]).unwrap_err();

        assert!(error.to_string().contains("HTTP response headers"));
    }

    #[test]
    fn response_json_redaction_rejects_non_utf8_body() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n\xff\xfe";
        let error = resolve_response_json_ranges(recv, &["/usd".to_string()]).unwrap_err();

        assert!(error.to_string().contains("UTF-8"));
    }

    #[test]
    fn response_json_redaction_rejects_invalid_json_body() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nnot-json";
        let error = resolve_response_json_ranges(recv, &["/usd".to_string()]).unwrap_err();

        assert!(error.to_string().contains("JSON response body"));
    }

    #[test]
    fn response_json_redaction_rejects_missing_pointer() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\n{\"base\":\"BTC\"}";
        let error = resolve_response_json_ranges(recv, &["/usd".to_string()]).unwrap_err();

        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn reveal_response_json_rejects_combined_redaction_modes() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 25\r\n\r\n{\"usd\":64000,\"base\":\"BTC\"}";
        let error = effective_recv_redact_ranges(
            recv,
            &[0..1],
            &ResponseRedactions {
                reveal_json_pointers: vec!["/usd".to_string()],
                ..ResponseRedactions::default()
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("cannot be combined"));
    }

    #[test]
    fn reveal_response_json_redacts_everything_except_value() {
        let recv = b"HTTP/1.1 200 OK\r\nContent-Length: 25\r\n\r\n{\"usd\":64000,\"base\":\"BTC\"}";
        let redactions = effective_recv_redact_ranges(
            recv,
            &[],
            &ResponseRedactions {
                reveal_json_pointers: vec!["/usd".to_string()],
                ..ResponseRedactions::default()
            },
        )
        .unwrap();
        let reveal_ranges = recv_reveal_ranges(recv.len(), &redactions).unwrap();
        let revealed = reveal_ranges
            .iter()
            .flat_map(|range| recv[range.clone()].to_vec())
            .collect::<Vec<_>>();

        assert_eq!(revealed, b"64000");
    }

    #[test]
    fn response_json_redaction_rejects_chunked_body() {
        let recv =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nD\r\n{\"usd\":64000}\r\n0\r\n\r\n";
        let error = resolve_response_json_ranges(recv, &["/usd".to_string()]).unwrap_err();

        assert!(error.to_string().contains("chunked"));
    }
}
