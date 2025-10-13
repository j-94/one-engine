# One Engine Research Draft

Title: From Ephemeral Thought to Immortal Tool: Conversational Learning, Safety, and Crystallization in One Engine

Abstract:
We demonstrate how a conversational agent (One Engine) learns from natural language, respects safety boundaries through a constrained UTIR (Universal Task IR), and produces a reusable cognitive asset that persists within a conversational branch. We show an observable workflow with HTTP endpoints, governance approvals, and a security model that blocks dangerous operations and enforces allowlisted domains.

1. Introduction
- Problem: Turning ephemeral agentic thoughts into durable, auditable tools.
- Contribution: A minimal but observable pipeline: Parse → Generate → Approve → Reuse, with UTIR guardrails and a branch-scoped memory.

2. System Overview
- Conversation Layer: /conversation (start, prompt, events); see src/api.rs (lines 121–140, 523–580).
- Parsing: Natural language → ParsedInstruction (CreateApi, CallApi, ApprovePattern); see src/parser.rs (lines 5–77, 79–109, 111–124).
- Branch State: Generated APIs and Events; see src/branch.rs (lines 21–58, 70–93, 94–141).
- UTIR Compiler: Safety rules and allowed domains; see src/compiler.rs (security: 21–60, command: 593–609, URL allowlist: 611–620).
- Memory: Ghosts → Crystallized Patterns (future cross-restart “immortalization”); see src/memory.rs (146–189, 190–244, 351–375).

3. Methods
- Conversational Learning:
  1) Define ‘echo’ (ephemeral)
  2) Call ‘echo’
  3) Define ‘echo’ as persistent
  4) Approve pattern ‘echo’ (governance)
  5) Reuse ‘echo’ within the branch
- Safety Boundary Evaluation (UTIR):
  - Blocked shell command (rm -rf /) → failed
  - Disallowed domain HTTP (example.com) → failed
  - Allowed domain HTTP (localhost healthz) → succeeded
- Additional Nudges: slugify, reverse, counter (approved and called; custom logic placeholder)

4. Results (Artifacts)
- Health & Version: out_one_engine/healthz.json, version.json
- Branch Evidence: out_one_engine/branch_id.txt
- Conversational Effects: out_one_engine/01_* through 06_* JSON files
- Safety UTIR Evidence: out_one_engine/utir_*.json (blocked, disallowed, allowed)
- Audit Trails: out_one_engine/events_tail.json, next_nudges_events_tail.json
- Git History Snapshot: out_one_engine/git_log.txt

5. Safety Architecture
- UTIR security rules block dangerous shell, enforce domain allowlists, cap execution time and response size.
- Bits (A,U,P,E,Δ,I,R,T) provide reflexive state for trust and permission.
- Approval step provides explicit governance record (ApprovalRecorded).

6. Discussion
- Current limitations: Custom behaviors return placeholders; branch-scoped persistence only.
- Next steps: Wire MemorySystem into conversation path to crystallize approved APIs into patterns.json for cross-restart reuse. Add typed behavior templates (e.g., string transforms) and unit tests.

7. Related Work
- Program synthesis with safety constraints
- Conversational schema evolution
- Agent safety and allowlists in execution sandboxes

8. Conclusion
We show a transparent, auditable loop from natural language to a reusable tool, bounded by safety and captured as artifacts. This establishes a foundation for research and production hardening.
