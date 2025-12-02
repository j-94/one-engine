# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Rust engine; `main.rs` boots the service while modules like `api.rs`, `compiler.rs`, and `conversation.rs` expose HTTP routes, UTIR execution, and chat logic.
- `memory/` holds the crystallized ledger (`ledger.jsonl` default); treat it as runtime state and omit bulky snapshots from commits.
- Operator tooling lives at the root: helper scripts (`run_dev.sh`, `deploy.sh`, `test_consciousness.sh`) plus `scripts/conversation_demo.sh` for quick demos.

## Build, Test, and Development Commands
- `cargo build` compiles the workspace; run it early to surface type or dependency issues.
- `cargo run` (or `./run_dev.sh`) starts the API with tracing hooks; keep it running while iterating on endpoints.
- `cargo fmt` and `cargo clippy --all-targets --all-features` are mandatory before push; `./test_consciousness.sh` exercises the public API against a live instance.

## Coding Style & Naming Conventions
- Follow idiomatic Rust: four-space indentation, `snake_case` for functions/modules, `CamelCase` for types, and exhaustive `match` statements where practical.
- Use `tracing` spans or events when touching request handlers so log output remains structured.
- Split oversized files into submodules under `src/` to keep responsibilities focused and reviewable.

## Testing Guidelines
- Place fast unit tests beside the code under `#[cfg(test)]`; async paths should rely on `#[tokio::test]`.
- `cargo test` must pass before review; add fixtures that avoid network calls unless explicitly mocked via tower layers.
- Extend `test_consciousness.sh` (or add sibling scripts) whenever API responses or CLI flows change, and note manual steps in the PR when smoke tests require a live server.

## Commit & Pull Request Guidelines
- Write imperative, Conventional Commit-style messages (`feat: enforce domain allowlist`) to keep the young history consistent.
- PRs should summarize intent, list functional changes, and capture test evidence; link issues or design notes when available.
- Request review only after formatting, linting, and targeted API scripts run cleanly; highlight migration or rollback considerations in the description.

## Security & Configuration Tips
- Configure via environment variables such as `ENGINE_API_KEY`, `ENGINE_ALLOWED_DOMAINS`, and `ENGINE_MEMORY_PATH`; never bake secrets into source.
- Validate new UTIR operations against the allowed-domain and shell guardrails before merging.
- For deployment, prefer `deploy.sh` with `one-engine.service` so systemd manages restart policy and log rotation; scrub `memory/ledger.jsonl` before sharing diagnostics.

## Telemetry Helpers
- Trigger the Codex + deterministic RLM suite via API:
  ```bash
  curl -s -X POST http://127.0.0.1:8080/telemetry/run \
    -H 'Content-Type: application/json' \
    -d '{"label_prefix":"daily"}' | jq
  ```
- Handy zsh alias (add to your shell profile):
  ```bash
  alias one-telemetry='curl -s -X POST http://127.0.0.1:8080/telemetry/run -H "Content-Type: application/json" -d "{\\"label_prefix\\":\\"daily\\"}" | jq'
  ```

## Meta Canon Workflow
- Canon definitions live under `meta/canons/*.yaml`; consult `meta/canons/schema.yaml` for required fields and provenance requirements.
- Inspect available canons via `GET /meta/catalog`, or fetch a single canon with `GET /meta/canons/:name` for raw JSON plus action previews.
- `POST /meta/run/:name` currently emits a planning stub (no side effects) describing the actions and noting that UTIR + receipt wiring must be implemented before enabling execution.
- Supporting artefacts:
  - UTIR templates: `utir/meta2_boot.utir`, `utir/ci_loop.utir`
  - Payload templates: `meta/payloads/bootstrap_branch.json`
- Canon postconditions assume receipts are mirrored into `Meta3Vault/Receipts/*`; keep vault artefacts aligned when extending the catalogue.
- For the narrative of how these decisions landed, review `docs/meta_journey.md`.
