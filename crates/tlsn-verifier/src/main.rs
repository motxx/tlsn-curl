use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;
use tlsn_attestation::presentation::{Presentation, PresentationOutput};

#[derive(Parser)]
#[command(name = "tlsn-verifier", about = "Verify TLSNotary presentation files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify a .presentation.tlsn file and output JSON result
    Verify {
        /// Path to the presentation file
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Verify { path } => match verify_presentation(&path) {
            Ok(output) => {
                println!("{}", serde_json::to_string(&output).unwrap());
            }
            Err(e) => {
                let output = json!({
                    "valid": false,
                    "server_name": null,
                    "time": null,
                    "revealed_body": null,
                    "revealed_body_segments": null,
                    "revealed_headers": null,
                    "revealed_sent": null,
                    "revealed_recv": null,
                    "revealed_recv_segments": null,
                    "error": format!("{:#}", e),
                });
                println!("{}", serde_json::to_string(&output).unwrap());
                std::process::exit(1);
            }
        },
    }
}

fn verify_presentation(path: &PathBuf) -> Result<serde_json::Value> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let presentation: Presentation = bincode::deserialize(&bytes)
        .context("Failed to deserialize presentation (expected bincode format)")?;

    let provider = tlsn_attestation::CryptoProvider::default();

    let PresentationOutput {
        server_name,
        connection_info,
        transcript,
        ..
    } = presentation
        .verify(&provider)
        .map_err(|e| anyhow::anyhow!("Verification failed: {}", e))?;

    let server_name_str: Option<String> = server_name.map(|s| format!("{}", s));

    let time = connection_info.time;

    let (revealed_sent, revealed_recv, revealed_recv_segments, revealed_body_segments) =
        match transcript {
            Some(mut partial) => {
                partial.set_unauthed(0u8);
                let sent_bytes = partial.sent_unsafe();
                let recv_bytes = partial.received_unsafe();
                let sent = render_with_redaction(sent_bytes);
                let recv = render_with_redaction(recv_bytes);
                let recv_segments = revealed_segments(recv_bytes);
                let body_segments = revealed_body_segments(recv_bytes);
                (Some(sent), Some(recv), Some(recv_segments), body_segments)
            }
            None => (None, None, None, None),
        };

    // Extract HTTP response body from revealed_recv (after \r\n\r\n)
    // Handle chunked Transfer-Encoding by stripping chunk framing
    let revealed_body = revealed_recv.as_ref().and_then(|recv| {
        recv.find("\r\n\r\n").map(|idx| {
            let raw_body = &recv[idx + 4..];
            decode_chunked_body(raw_body).unwrap_or_else(|| raw_body.to_string())
        })
    });

    // Extract HTTP response headers
    let revealed_headers = revealed_recv
        .as_ref()
        .and_then(|recv| recv.find("\r\n\r\n").map(|idx| recv[..idx].to_string()));

    Ok(json!({
        "valid": true,
        "server_name": server_name_str,
        "time": time,
        "revealed_body": revealed_body,
        "revealed_body_segments": revealed_body_segments,
        "revealed_headers": revealed_headers,
        "revealed_sent": revealed_sent,
        "revealed_recv": revealed_recv,
        "revealed_recv_segments": revealed_recv_segments,
        "error": null,
    }))
}

/// Render transcript bytes, replacing runs of \0 (unauthed/redacted bytes) with [REDACTED].
/// This preserves the cryptographic proof — redacted regions are still committed to
/// but their content is not revealed in the presentation.
fn render_with_redaction(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0 {
            // Find end of redacted run
            while i < bytes.len() && bytes[i] == 0 {
                i += 1;
            }
            result.push_str("[REDACTED]");
        } else {
            // Find end of non-redacted run
            let start_idx = i;
            while i < bytes.len() && bytes[i] != 0 {
                i += 1;
            }
            result.push_str(&String::from_utf8_lossy(&bytes[start_idx..i]));
        }
    }
    result
}

#[derive(Debug, PartialEq, Serialize)]
struct RevealedSegment {
    start: usize,
    end: usize,
    text: String,
}

fn revealed_segments(bytes: &[u8]) -> Vec<RevealedSegment> {
    let mut segments = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0 {
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() && bytes[i] != 0 {
            i += 1;
        }
        segments.push(RevealedSegment {
            start,
            end: i,
            text: String::from_utf8_lossy(&bytes[start..i]).to_string(),
        });
    }
    segments
}

fn revealed_body_segments(bytes: &[u8]) -> Option<Vec<RevealedSegment>> {
    let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n")?;
    let body_start = header_end + 4;
    Some(revealed_segments(&bytes[body_start..]))
}

/// Decode HTTP chunked transfer encoding.
/// Input: "19\r\n{...json...}\r\n0\r\n\r\n"
/// Output: "{...json...}"
fn decode_chunked_body(raw: &str) -> Option<String> {
    let mut result = String::new();
    let mut remaining = raw;

    loop {
        // Find chunk size line
        let crlf_pos = remaining.find("\r\n")?;
        let size_str = remaining[..crlf_pos].trim();
        let chunk_size = usize::from_str_radix(size_str, 16).ok()?;

        if chunk_size == 0 {
            break; // final chunk
        }

        let data_start = crlf_pos + 2;
        let data_end = data_start + chunk_size;
        if data_end + 2 > remaining.len() {
            return None; // truncated
        }

        if &remaining[data_end..data_end + 2] != "\r\n" {
            return None;
        }

        result.push_str(&remaining[data_start..data_end]);
        remaining = &remaining[data_end + 2..];
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_chunked_body, render_with_redaction, revealed_body_segments, revealed_segments,
        RevealedSegment,
    };

    #[test]
    fn render_with_redaction_keeps_plain_text() {
        assert_eq!(render_with_redaction(b"GET / HTTP/1.1"), "GET / HTTP/1.1");
    }

    #[test]
    fn render_with_redaction_collapses_adjacent_redacted_runs() {
        assert_eq!(
            render_with_redaction(b"abc\0\0def\0ghi"),
            "abc[REDACTED]def[REDACTED]ghi"
        );
    }

    #[test]
    fn render_with_redaction_handles_leading_and_trailing_redaction() {
        assert_eq!(
            render_with_redaction(b"\0\0Host: example.com\0"),
            "[REDACTED]Host: example.com[REDACTED]"
        );
    }

    #[test]
    fn render_with_redaction_handles_empty_input() {
        assert_eq!(render_with_redaction(b""), "");
    }

    #[test]
    fn render_with_redaction_replaces_invalid_utf8_lossily() {
        assert_eq!(render_with_redaction(&[0xff, b'o', b'k']), "\u{fffd}ok");
    }

    #[test]
    fn revealed_segments_returns_unredacted_runs_without_markers() {
        assert_eq!(
            revealed_segments(b"\0\0\"123\"\0true\0"),
            vec![
                RevealedSegment {
                    start: 2,
                    end: 7,
                    text: "\"123\"".to_string(),
                },
                RevealedSegment {
                    start: 8,
                    end: 12,
                    text: "true".to_string(),
                },
            ]
        );
    }

    #[test]
    fn revealed_body_segments_returns_body_relative_runs() {
        assert_eq!(
            revealed_body_segments(b"HTTP/1.1 200 OK\r\n\r\n\0\0\"123\"\0true\0"),
            Some(vec![
                RevealedSegment {
                    start: 2,
                    end: 7,
                    text: "\"123\"".to_string(),
                },
                RevealedSegment {
                    start: 8,
                    end: 12,
                    text: "true".to_string(),
                },
            ])
        );
    }

    #[test]
    fn revealed_body_segments_returns_none_without_header_boundary() {
        assert_eq!(revealed_body_segments(b"\0\0\"123\"\0"), None);
    }

    #[test]
    fn decode_chunked_body_decodes_multiple_chunks() {
        assert_eq!(
            decode_chunked_body("5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn decode_chunked_body_accepts_final_zero_chunk() {
        assert_eq!(decode_chunked_body("0\r\n\r\n"), Some(String::new()));
    }

    #[test]
    fn decode_chunked_body_rejects_malformed_chunk_size() {
        assert_eq!(decode_chunked_body("z\r\nhello\r\n0\r\n\r\n"), None);
    }

    #[test]
    fn decode_chunked_body_rejects_truncated_chunk_data() {
        assert_eq!(decode_chunked_body("5\r\nhel"), None);
    }

    #[test]
    fn decode_chunked_body_rejects_missing_chunk_terminator() {
        assert_eq!(decode_chunked_body("5\r\nhello0\r\n\r\n"), None);
    }

    #[test]
    fn decode_chunked_body_rejects_missing_final_zero_chunk() {
        assert_eq!(decode_chunked_body("5\r\nhello\r\n"), None);
    }

    #[test]
    fn decode_chunked_body_returns_none_for_non_chunked_input() {
        assert_eq!(decode_chunked_body("plain response body"), None);
    }
}
