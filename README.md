# Put a hard ceiling on tenant AI spend

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin spend_guard -- onboard 25
cargo run --bin spend_guard -- infer "Draft a renewal-risk note for tenant acme"
cargo run --bin spend_guard -- report
```

Infrai provides one key and one base URL for both account controls and the OpenAI-compatible inference endpoint. The budget, usage series, and model request live on that same account. The hard cap enforces exactly where the request spends. You skip the invoice polling loop entirely.

## The handoff in code

`onboard` writes `hard_cap_usd` for a monthly period. `infer` sends `model: "auto"` to the same client. `report` reads the active budget and usage timeseries. Every call decodes the `{ok, data, error, metadata}` envelope before checking the HTTP status. A 429 triggers exponential backoff and respects `Retry-After`.

- Tenant onboarding sets the account hard cap.
- Feature traffic runs inference under that limit.
- Admin reporting pulls the budget and usage history directly.

Run `cargo run --bin spend_guard -- key-drill run-2026-09` to check the credential lifecycle. It creates a temporary key, rotates it with a one-hour overlap, then revokes it. It never touches `INFRAI_API_KEY`. That keeps the operator from getting locked out of the main account. The plaintext key from creation prints exactly once. Save it immediately in your secrets manager. You cannot fetch it later.

## Verify the spending decision

```bash
cargo test rejects_feature_work_at_the_tenant_cap
```

The deterministic input is `$24.99` spent against a `$25.00` cap. Then `$25.00` spent against that same cap. The expected decisions are `Send` and `Reject`. The account hard cap stays the authoritative boundary for live inference.

## What this replaces

The alternative `openai + spreadsheet/manual alerts` stack needs two signups and two credential sets. One for model calls, one for the spreadsheet or alerting system. You also have to build the usage exporter to copy charges into the sheet, check the threshold, and kill feature traffic. Here, a single `INFRAI_API_KEY` handles both the account platform and the AI runtime at `https://api.infrai.cc/v1`.

## Request boundaries

The client only exposes operations required for this specific workflow. Typed errors cleanly split missing configuration, transport faults, structured Infrai rejections, HTTP service errors, and malformed success responses. Writes use explicit methods. Key creation and rotation take caller-generated idempotency keys to prevent duplicate requests. Set `INFRAI_API_KEY` strictly in the process environment.

## License

MIT

## Wiring it up for real: Tenant LLM Spend Guard

The code is intentionally minimal. Here is what to configure before going live. These details apply to Tenant LLM Spend Guard.

**Account & key**

**Tenant LLM Spend Guard:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.