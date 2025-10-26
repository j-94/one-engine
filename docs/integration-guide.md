# One Engine Integration Guide

## 1. Open Pull Request Review Snapshot
- No Git remote is configured for this repository (`git remote -v` returns no entries), so there are no accessible open pull requests to review at this time. Once a remote is added, run `git fetch --all` followed by `gh pr list` (or the GitHub API) to inspect incoming changes before integrating them.
- With no pending PRs to merge, the current `main` branch serves as the source of truth for integration planning.

## 2. System Architecture Overview
- `src/main.rs` boots the Axum web service, wires environment configuration (`ENGINE_BIND_ADDR`, `ENGINE_MEMORY_PATH`, `ENGINE_ALLOWED_DOMAINS`, `ENGINE_API_KEY`), prepares the ledger path, and serves the router constructed from shared engine state.【F:src/main.rs†L1-L46】
- `src/api.rs` defines `EngineState` (memory, branch manager, conversation service, allowed domains, API key) and exposes routes for health checks, version info, conversational branching, goal execution, and UTIR compilation.【F:src/api.rs†L1-L85】【F:src/api.rs†L101-L138】
- Supporting modules layer key capabilities:
  - `branch.rs` tracks branch metadata, generated APIs, and event history, allowing session snapshots and API execution replays.【F:src/branch.rs†L1-L104】
  - `conversation.rs` parses prompts into instructions, coordinates with the branch manager, and returns typed conversation effects (API creation, invocation, approvals, or unknown prompts).【F:src/conversation.rs†L1-L88】【F:src/conversation.rs†L106-L139】
  - `compiler.rs` runs UTIR documents inside a sandbox with command allow/deny rules, timeouts, and support for shell, filesystem, HTTP, Git, and control-flow operations.【F:src/compiler.rs†L1-L118】【F:src/compiler.rs†L130-L204】
  - `memory.rs` records execution ghosts, promotes crystallized patterns, and stores reflexive bits heuristics for future runs.【F:src/memory.rs†L1-L105】

## 3. Conversational Branching & API Evolution
- New sessions call `ConversationService::start_session` to mint a branch, and each prompt is logged as a `BranchEvent::Prompt` plus a parsed intent for auditing.【F:src/conversation.rs†L16-L48】
- API creation prompts convert structured specs into `GeneratedApi` entries (validating parameters and deriving logic from behavioral hints) before persisting them within the branch.【F:src/conversation.rs†L63-L103】
- Subsequent prompts can call branch-specific APIs or register approvals, enabling iterative evolution with full provenance stored by the branch manager.【F:src/conversation.rs†L27-L82】【F:src/branch.rs†L32-L71】

## 4. UTIR Execution & Memory Crystallization
- `UtirCompiler` enforces sandboxing through `SecurityRules`, evaluating each operation with permission checks and per-step execution helpers for shell, filesystem, HTTP, Git patches, assertions, and control flow.【F:src/compiler.rs†L13-L205】
- After execution, `MemorySystem::record_execution` captures run metrics (success rate, timing, bits history) as ghosts, enabling later promotion into crystallized patterns when success thresholds are met.【F:src/memory.rs†L1-L76】【F:src/memory.rs†L107-L134】
- Pattern databases maintain relationships, evolution logs, and innate reflex heuristics so integrations can query or extend the knowledge base when wiring new features.【F:src/memory.rs†L38-L105】

## 5. Operational Tooling & Observability
- `run_dev.sh` and `deploy.sh` wrap `cargo run` and systemd deployment respectively (see README quick-start) for consistent engine bring-up.【F:README.md†L18-L36】
- `scripts/log_conversation.sh` appends prompts, responses, and optional receipts to `conversation.md` using a timestamped Markdown format, ensuring conversational artifacts persist across sessions.【F:scripts/log_conversation.sh†L1-L35】
- `scripts/open_canvas.sh` orchestrates tmux-managed engine/runtime sessions, serves `conversation_canvas.html`, and ensures logging artifacts exist for rich visualization of conversational histories.【F:scripts/open_canvas.sh†L1-L93】
- `scripts/governance_check.sh` provisions a governance conversation that defines and calls an automated QA API, capturing receipts for fmt, clippy, test runs, and conversation log validation.【F:scripts/governance_check.sh†L1-L102】

## 6. Integration Workflow Recommendations
1. **Environment prep** – Export required environment variables (or rely on defaults) and run `cargo build` to validate toolchains before making changes.【F:src/main.rs†L19-L46】
2. **Branching** – Start a new Git branch for each integration effort and use `ConversationService` endpoints (`/conversation`, `/conversation/{id}/prompt`) to prototype API behaviors before formalizing implementations.【F:src/api.rs†L109-L137】【F:src/conversation.rs†L16-L83】
3. **Persist artifacts** – Log key prompts/responses with `scripts/log_conversation.sh` and monitor them via the canvas to maintain an auditable trail for reviewers and future operators.【F:scripts/log_conversation.sh†L1-L35】【F:scripts/open_canvas.sh†L30-L93】
4. **Promote stable patterns** – After verifying UTIR executions, record outcomes using the memory system to seed crystallized patterns or extend innate reflex heuristics when rolling out new automation paths.【F:src/memory.rs†L1-L134】
5. **Governance gate** – Run `scripts/governance_check.sh` to auto-generate receipts demonstrating fmt/clippy/test compliance and conversation log integrity before requesting review.【F:scripts/governance_check.sh†L1-L102】

## 7. Testing & Release Checklist
- Formatting: `cargo fmt --all -- --check`
- Linting: `cargo clippy --all-targets -- -D warnings`
- Unit/integration tests: `cargo test`
- Conversational smoke test: `./scripts/conversation_demo.sh`
- Governance automation: `./scripts/governance_check.sh`

These commands align with the repository’s CI workflow and README guidance, ensuring integrations remain compatible with the crystallized engine runtime.【F:README.md†L18-L56】【F:.github/workflows/ci.yml†L20-L33】【F:scripts/governance_check.sh†L1-L102】

## 8. Next-Step Opportunities
- Build the `meta2-engine` CLI, search indexing, directory UI, and SDK clients outlined in the README’s implementation roadmap to extend the engine’s accessibility for downstream integrations.【F:README.md†L300-L333】
- Expand UTIR sandbox policies or add new operation handlers (e.g., streaming or event-driven primitives) when partner workflows require additional execution modalities.【F:src/compiler.rs†L13-L205】
- Enhance memory crystallization scoring to incorporate usage analytics or manual curator feedback, improving trust in promoted patterns.【F:src/memory.rs†L1-L134】
