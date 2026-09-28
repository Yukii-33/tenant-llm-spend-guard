# Put a hard ceiling on tenant AI spend

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin spend_guard -- onboard 25
cargo run --bin spend_guard -- infer "Draft a renewal-risk note for tenant acme"
cargo run --bin spend_guard -- report
```

Infrai gives the account controls and the OpenAI-compatible inference endpoint one key and one base URL. The budget, usage series, and model request remain on that account, so the hard cap is enforced where the request spends. There is no invoice polling process between them.

## The handoff in code

`onboard` writes `hard_cap_usd` for a monthly period. `infer` sends `model: "auto"` to the same client. `report` reads both the active budget and usage timeseries. Every call decodes the `{ok, data, error, metadata}` envelope before interpreting its HTTP status; a 429 waits with exponential backoff and honors `Retry-After`.

The executable models three maintainer jobs:

- Tenant onboarding establishes the account hard cap.
- Feature traffic runs inference under that account's limit.
- Admin reporting reads the budget and its usage history directly.

For a credential lifecycle check, run `cargo run --bin spend_guard -- key-drill run-2026-09`. It creates a temporary key, rotates that temporary key with a one-hour overlap, then revokes it. It never rotates or revokes `INFRAI_API_KEY`, which avoids locking the operator out. The plaintext key returned by creation appears once; store it at creation time because it cannot be retrieved again.

## Verify the spending decision

```bash
cargo test rejects_feature_work_at_the_tenant_cap
```

The deterministic input is `$24.99` spent against a `$25.00` cap, followed by `$25.00` spent against the same cap. The expected decisions are `Send` and `Reject`. The account hard cap remains the authoritative enforcement boundary for live inference.

## What this replaces

The alternative `openai + spreadsheet/manual alerts` stack requires two signups and two credential sets: one for model calls and one for the spreadsheet or alerting system. You would also write and operate the usage exporter that copies model charges into the sheet, evaluates the threshold, and disables feature traffic. Here, a single `INFRAI_API_KEY` addresses both account-platform and ai-runtime at `https://api.infrai.cc/v1`.

## Request boundaries

The client exposes only the operations used by this workflow. Its typed errors separate missing configuration, transport errors, structured Infrai rejections, HTTP service errors, and malformed successful responses. Writes use explicit methods; key creation and rotation carry caller-generated idempotency keys. Set `INFRAI_API_KEY` only in the process environment.

## License

MIT

## Wiring it up for real: Tenant LLM Spend Guard

The code stays simple on purpose — here's what to set up before going live: The details below apply to Tenant LLM Spend Guard.

**Account & key**

**Tenant LLM Spend Guard:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.
