# tlsn-curl

Create and verify TLSNotary proofs from the command line.

## Quick Start

Requires Rust and HTTPS access.

```sh
cargo build --release
./target/release/tlsn-curl "https://example.com" --out example-proof.json
./target/release/tlsn-verify example-proof.json
```

Look for `"ok": true` in the verification output.

## Install Commands

```sh
cargo install --path .
```

Installs `tlsn-curl`, `tlsn-verify`, and implementation sidecars.

## Examples

```sh
tlsn-curl "https://example.com" --out proof.json
tlsn-verify proof.json
```

Secret header from env, redacted in the proof:

```sh
export API_TOKEN='Bearer ...'
tlsn-curl "https://api.example.com/me" \
  --out me-proof.json \
  --header-env "Authorization: API_TOKEN"
```

Reveal one JSON value:

```sh
tlsn-curl "https://api.coinbase.com/v2/prices/BTC-USD/spot" \
  --out coinbase-proof.json \
  --reveal-response-json /data/amount
```

Pipe proof JSON:

```sh
tlsn-curl "https://example.com" --out - > proof.json
tlsn-verify - < proof.json
tlsn-curl "https://example.com" --out - | tlsn-verify -
```

Network-free envelope smoke test:

```sh
tlsn-curl "https://example.com" --out - --pending
```

## Command Shape

```sh
tlsn-curl <https-url> --out <file|-> [options]
tlsn-verify <proof.json|->
```

Useful options:

- `-H`, `--header`
- `--header-env "Name: ENV_VAR"`
- `--reveal-response-json /path`
- `--redact-recv-range start:end`
- `--verifier host:port`

## Local Verifier

```sh
./scripts/local-up.sh
tlsn-curl "https://example.com" --out local-proof.json --verifier localhost:7046
./scripts/local-down.sh
```

```sh
./scripts/e2e-tlsn.sh
```

## Development

```sh
./scripts/test-all.sh --local
./scripts/test-all.sh --docker
```

Details:

- [docs/proof-format.md](docs/proof-format.md): proof JSON, redaction, and
  verification behavior.
- [docs/development.md](docs/development.md): quality gates and implementation
  map.
