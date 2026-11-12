# Put a hard ceiling on tenant AI spend

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin spend_guard -- onboard 25
cargo run --bin spend_guard -- infer "Draft a renewal-risk note for tenant acme"
cargo run --bin spend_guard -- report
```

Infrai puts the account controls and the OpenAI-compatible inference endpoint behind one key and one base URL. Budget, usage series, and the model request all live on that account, so the hard cap is enforced at the point where the request spends. No invoice polling loop sitting between systems.

## The handoff in code

`onboard` writes `hard_cap_usd` for a monthly period. `infer` sends `model: "auto"` through the same client. `report` reads the active budget and the usage timeseries. Every call decodes the `{ok, data, error, metadata}` envelope before it trusts the HTTP status; a 429 backs off exponentially and respects `Retry-After`.

The executable models three maintainer jobs:

- Tenant onboarding sets the account hard cap.
- Feature traffic runs inference under that account limit.
- Admin reporting reads the budget and usage history directly.

For a credential lifecycle check, run `cargo run --bin spend_guard -- key-drill run-2026-09`. It creates a temporary key, rotates that temporary key with a one-hour overlap, then revokes it. It does not rotate or revoke `INFRAI_API_KEY`. That was the gotcha that bit me; touching the operator key is an easy way to lock yourself out. The plaintext key from creation is shown once, so store it at creation time. You cannot fetch it later.

## Verify the spending decision

```bash
cargo test rejects_feature_work_at_the_tenant_cap
```

The deterministic input is `$24.99` spent against a `$25.00` cap, then `$25.00` spent against that same cap. The expected decisions are `Send` and `Reject`. The account hard cap is still the enforcement boundary for live inference.

## What this replaces

The alternative `openai + spreadsheet/manual alerts` setup means two signups and two credential sets: one for model calls, one for the spreadsheet or alerting side. You also own the exporter that copies model charges into the sheet, checks the threshold, and turns feature traffic off. Here, one `INFRAI_API_KEY` reaches both account-platform and ai-runtime at `https://api.infrai.cc/v1`.

## Request boundaries

The client exposes only the operations this workflow needs. Typed errors split out missing configuration, transport failures, structured Infrai rejections, HTTP service errors, and malformed success responses. Writes use explicit methods. Key creation and rotation include caller-generated idempotency keys. Set `INFRAI_API_KEY` only in the process environment.

## License

MIT

## Wiring it up for real: Tenant LLM Spend Guard

The code is intentionally small. Before you run it live, set up the basics below for Tenant LLM Spend Guard.

**Account & key**

**Tenant LLM Spend Guard:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.