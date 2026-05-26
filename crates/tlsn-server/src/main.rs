use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_util::compat::TokioAsyncReadCompatExt;

use tlsn::{
    attestation::{
        request::Request as AttestationRequest, signing::Secp256k1Signer, Attestation,
        AttestationConfig, CryptoProvider,
    },
    config::verifier::VerifierConfig,
    connection::{ConnectionInfo, TranscriptLength},
    transcript::{ContentType, Record},
    verifier::VerifierOutput,
    Session,
};

#[derive(Parser)]
#[command(name = "tlsn-server", about = "Local TLSNotary verifier server")]
struct Cli {
    /// TCP verifier listen address.
    #[arg(long, default_value = "0.0.0.0:7047")]
    listen: SocketAddr,
}

#[derive(Clone)]
struct VerifierState {
    connection_info: ConnectionInfo,
    server_ephemeral_key: Vec<u8>,
    transcript_commitments: Vec<u8>,
}

type Sessions = Arc<Mutex<HashMap<[u8; 16], VerifierState>>>;

#[derive(Debug)]
enum ConnectionMode {
    Mpc,
    Attestation,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let listener = TcpListener::bind(cli.listen).await?;
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));

    eprintln!("[tlsn-server] listening on {}", cli.listen);

    loop {
        let (stream, peer) = listener.accept().await?;
        let sessions = sessions.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, sessions).await {
                eprintln!("[tlsn-server] {}: {error:#}", peer);
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream, sessions: Sessions) -> Result<()> {
    let mut mode = [0u8; 1];
    let mut session_id = [0u8; 16];
    stream.read_exact(&mut mode).await?;
    stream.read_exact(&mut session_id).await?;

    match parse_connection_mode(mode[0])? {
        ConnectionMode::Mpc => handle_mpc(stream, session_id, sessions).await,
        ConnectionMode::Attestation => handle_attestation(stream, session_id, sessions).await,
    }
}

fn parse_connection_mode(mode: u8) -> Result<ConnectionMode> {
    match mode {
        b'M' => Ok(ConnectionMode::Mpc),
        b'A' => Ok(ConnectionMode::Attestation),
        other => Err(anyhow!("unknown connection mode byte: {other}")),
    }
}

async fn handle_mpc(stream: TcpStream, session_id: [u8; 16], sessions: Sessions) -> Result<()> {
    eprintln!(
        "[tlsn-server] MPC session {} connected",
        hex::encode(&session_id[..8])
    );

    let session = Session::new(stream.compat());
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

    let connection_info = ConnectionInfo {
        time: tls_transcript.time(),
        version: *tls_transcript.version(),
        transcript_length: TranscriptLength {
            sent: application_data_len(tls_transcript.sent()) as u32,
            received: application_data_len(tls_transcript.recv()) as u32,
        },
    };
    let state = VerifierState {
        connection_info,
        server_ephemeral_key: bincode::serialize(tls_transcript.server_ephemeral_key())?,
        transcript_commitments: bincode::serialize(&transcript_commitments)?,
    };

    sessions.lock().await.insert(session_id, state);
    handle.close();
    driver_task.await??;

    eprintln!(
        "[tlsn-server] MPC session {} stored verifier data",
        hex::encode(&session_id[..8])
    );

    Ok(())
}

async fn handle_attestation(
    mut stream: TcpStream,
    session_id: [u8; 16],
    sessions: Sessions,
) -> Result<()> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let req_len = u32::from_be_bytes(len_buf) as usize;
    let mut req_buf = vec![0u8; req_len];
    stream.read_exact(&mut req_buf).await?;
    let request: AttestationRequest = bincode::deserialize(&req_buf)?;

    let state = wait_for_session(session_id, sessions).await?;
    let attestation = build_attestation(request, state)?;
    let attestation_bytes = bincode::serialize(&attestation)?;

    stream
        .write_all(&(attestation_bytes.len() as u32).to_be_bytes())
        .await?;
    stream.write_all(&attestation_bytes).await?;
    stream.flush().await?;

    eprintln!(
        "[tlsn-server] attestation session {} completed",
        hex::encode(&session_id[..8])
    );

    Ok(())
}

async fn wait_for_session(session_id: [u8; 16], sessions: Sessions) -> Result<VerifierState> {
    for _ in 0..300 {
        if let Some(state) = sessions.lock().await.remove(&session_id) {
            return Ok(state);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow!(
        "timed out waiting for MPC verifier data for session {}",
        hex::encode(&session_id[..8])
    ))
}

fn build_attestation(request: AttestationRequest, state: VerifierState) -> Result<Attestation> {
    let signing_key = k256::ecdsa::SigningKey::random(&mut rand::thread_rng());
    let signer = Box::new(Secp256k1Signer::new(&signing_key.to_bytes())?);
    let mut provider = CryptoProvider::default();
    provider.signer.set_signer(signer);

    let att_config = AttestationConfig::builder()
        .supported_signature_algs(Vec::from_iter(provider.signer.supported_algs()))
        .build()?;

    let mut builder = Attestation::builder(&att_config).accept_request(request)?;
    builder
        .connection_info(state.connection_info)
        .server_ephemeral_key(
            bincode::deserialize(&state.server_ephemeral_key)
                .context("failed to deserialize server ephemeral key")?,
        )
        .transcript_commitments(
            bincode::deserialize(&state.transcript_commitments)
                .context("failed to deserialize transcript commitments")?,
        );

    Ok(builder.build(&provider)?)
}

fn application_data_len(records: &[Record]) -> usize {
    records
        .iter()
        .filter_map(|record| match record.typ {
            ContentType::ApplicationData => Some(record.ciphertext.len()),
            _ => None,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{application_data_len, parse_connection_mode, ConnectionMode};
    use tlsn::transcript::{ContentType, Record};

    fn record(typ: ContentType, ciphertext_len: usize) -> Record {
        Record {
            seq: 0,
            typ,
            plaintext: None,
            explicit_nonce: Vec::new(),
            ciphertext: vec![0; ciphertext_len],
            tag: None,
        }
    }

    #[test]
    fn application_data_len_returns_zero_for_empty_records() {
        assert_eq!(application_data_len(&[]), 0);
    }

    #[test]
    fn application_data_len_excludes_non_application_records() {
        let records = vec![
            record(ContentType::Handshake, 10),
            record(ContentType::Alert, 20),
            record(ContentType::ChangeCipherSpec, 30),
        ];

        assert_eq!(application_data_len(&records), 0);
    }

    #[test]
    fn application_data_len_sums_multiple_application_records() {
        let records = vec![
            record(ContentType::Handshake, 100),
            record(ContentType::ApplicationData, 12),
            record(ContentType::ApplicationData, 34),
        ];

        assert_eq!(application_data_len(&records), 46);
    }

    #[test]
    fn parse_connection_mode_accepts_known_modes() {
        assert!(matches!(
            parse_connection_mode(b'M').unwrap(),
            ConnectionMode::Mpc
        ));
        assert!(matches!(
            parse_connection_mode(b'A').unwrap(),
            ConnectionMode::Attestation
        ));
    }

    #[test]
    fn parse_connection_mode_rejects_unknown_mode() {
        let error = parse_connection_mode(b'?').unwrap_err().to_string();
        assert!(error.contains("unknown connection mode byte: 63"));
    }
}
