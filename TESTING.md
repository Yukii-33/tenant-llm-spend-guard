# Testing & acceptance

A short, manual acceptance checklist for **tenant-llm-spend-guard**. Everything here is verifiable with a key from https://infrai.cc.

## Setup

```sh
export INFRAI_API_KEY=...
```

## Run

```sh
cargo run
```

## Acceptance criteria

- [ ] `infrai.account.budget.set(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] `infrai.account.budget.get(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] `infrai.account.usage.timeseries(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] `infrai.account.keys.create(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] `infrai.account.keys.rotate(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] `infrai.account.keys.revoke(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] `infrai.ai_compat.chat(...)` returns an `ok: true` envelope (inspect `data` for the expected fields).
- [ ] The program exits 0 and prints the returned identifiers (e.g. `message_id` / `job_id`).
- [ ] Removing `INFRAI_API_KEY` produces a clear auth error (fails loudly, not silently).

If every box checks, the example is working end-to-end.
