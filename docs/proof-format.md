# Proof Format

Successful proof generation writes a JSON envelope containing the claim and a
base64 TLSNotary presentation. Use `--out <file>` to write it to a file, or
`--out -` to write it to stdout for shell pipelines:

```json
{
  "version": "tlsn-curl/v0",
  "kind": "tlsnotary-fetch-proof",
  "createdAt": "2026-05-16T00:00:00.000Z",
  "claim": {
    "url": "https://example.com/",
    "method": "GET",
    "requestHeaders": {}
  },
  "tlsn": {
    "status": "complete",
    "proof": {
      "format": "presentation.tlsn",
      "encoding": "base64",
      "data": "<base64 presentation>",
      "maxSentData": 4096,
      "maxRecvData": 4096
    }
  }
}
```

## Verification

`tlsn-verify <proof.json>` reads the envelope from a file. `tlsn-verify -`
reads the same JSON envelope from stdin.

Verification fails closed when the envelope is pending, the presentation is
mutated, the verifier binary is unavailable, or the verified TLS server name
does not match the claimed URL host.

Successful verification output includes both compatibility-oriented transcript
renderings and marker-free extracted segments:

- `revealedRecv` and `revealedBody` render hidden byte runs as `[REDACTED]`.
- `revealedRecvSegments` contains the revealed received transcript byte runs
  without `[REDACTED]` markers.
- `revealedBodySegments` contains the revealed HTTP response body byte runs
  without `[REDACTED]` markers, using body-relative byte offsets.
- `revealedJsonValues` contains JSON-parsed values when every revealed body
  segment, or every revealed received segment if the body boundary is hidden, is
  a complete JSON value.

Proof and verification JSON are written to stdout. Human-readable status and
errors are written to stderr so callers can pipe stdout into tools such as
`jq` without filtering diagnostics.

## Header Redaction

Use `--header` only for non-sensitive values because process arguments can be
visible to local process inspectors. The CLI rejects common sensitive names
such as `Authorization`, `Cookie`, and `X-Api-Key` unless they are passed as
`--header-env "Name: ENV_VAR"`.

Headers passed with `--header` are treated as non-sensitive proof conditions:
they are recorded in the proof envelope and must be revealed in the sent HTTP
request transcript for verification to succeed. Headers passed with
`--header-env` are recorded as `[REDACTED]` in the proof envelope and passed to
the prover as TLSNotary sent-header redactions. `[REDACTED]` is verifier-side
rendering of bytes that were intentionally not revealed by the transcript
proof.

## Response Byte Redaction

Use `--redact-recv-range start:end` to hide received TLS transcript bytes from
the presentation. Offsets are byte positions in the raw received TLS
application transcript, including HTTP response headers and body framing, not a
parsed JSON document or decoded DOM view.

Ranges must satisfy `0 <= start < end` and must fit within the received
transcript length. Multiple ranges are allowed; the prover reveals the
remaining received bytes and the verifier renders hidden runs as `[REDACTED]`.

For JSON responses, use `--redact-response-json /path` to hide a JSON Pointer
value, or `--reveal-response-json /path` to reveal only that JSON value and hide
the rest of the received transcript. Structured JSON redaction is conservative:
the response body must be unchunked UTF-8 JSON, the pointer must exist, and the
raw JSON value must appear exactly once in the body.

## Verifier Modes

If `--verifier` is omitted, the prover uses its in-process verifier path. Pass
`--verifier localhost:7047` for a TCP verifier or `--verifier ws://host:port`
for a compatible WebSocket verifier.
