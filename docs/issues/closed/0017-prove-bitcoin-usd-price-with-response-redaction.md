# Prove Bitcoin USD price with response redaction

Created: 2026-05-16
Model: GPT-5

## Priority

maintenance

## Dependencies

Depends on:
- 0016

Blocks:
- None

## Summary

After response body redaction is implemented, run a real proof-generation
exercise for the current Bitcoin USD price and verify that only the USD price
is revealed.

## Rationale

The response-redaction feature should be validated against a realistic public
market-data endpoint, not only unit tests. The proof artifact should demonstrate
that TLSNotary can prove a live HTTPS response while selectively disclosing the
price and withholding all other response fields or body content.

## Plan

- Pick a stable HTTPS endpoint that returns BTC/USD market data and document
  the exact URL used.
- Generate a proof envelope with response redaction configured so only the USD
  price bytes are revealed.
- Verify the proof with `tlsn-verify`.
- Inspect the verification output and assert that the revealed response
  contains the USD price but does not expose other response fields, metadata,
  timestamps, symbols, or unrelated body content.
- Record the command sequence and sanitized verification result in the issue
  resolution without committing private proof artifacts.

Completed: 2026-05-16

## Resolution

Implemented by running:

- `deno task build:tlsn`
- `deno task tlsn-fetch 'https://api.coinbase.com/v2/prices/BTC-USD/spot' --out /tmp/tlsn-fetch-btc-proof.json --reveal-response-json /data/amount --max-recv-data 8192`
- `deno task tlsn-verify /tmp/tlsn-fetch-btc-proof.json`

Verified with:

- `deno task lint:paths`

Harness update:

- None - this issue records a one-off live proof exercise using the structured
  redaction harness from issue 0016.

Review residuals:

- The generated proof artifact stayed under `/tmp` and was not committed.
- Sanitized verification result: `ok: true`, `serverName: api.coinbase.com`,
  `revealedRecv: [REDACTED]"78975.575"[REDACTED]`.
- The revealed response did not expose Coinbase response headers, cookies,
  `base`, `currency`, or unrelated body content.

Follow-up:

- None
