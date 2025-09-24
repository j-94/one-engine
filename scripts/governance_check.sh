#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

BASE_URL="${ENGINE_URL:-http://127.0.0.1:7777}"
LOG_DIR="logs"
mkdir -p "$LOG_DIR"

if ! command -v jq >/dev/null 2>&1; then
  echo "[error] jq is required on the PATH" >&2
  exit 1
fi

if ! curl -s "$BASE_URL/healthz" >/dev/null 2>&1; then
  echo "[error] engine not reachable at $BASE_URL" >&2
  echo "        make sure 'cargo run' is running in another terminal." >&2
  exit 1
fi

print_json() {
  local payload="$1"
  if jq '.' >/dev/null 2>&1 <<<"$payload"; then
    jq '.' <<<"$payload"
  else
    printf '%s\n' "$payload"
  fi
}

TIMESTAMP=$(date -u '+%Y%m%dT%H%M%SZ')
REPORT_FILE="$LOG_DIR/governance_report_${TIMESTAMP}.txt"

conversation_resp=$(curl -s -X POST "$BASE_URL/conversation" \
  -H 'content-type: application/json' \
  -d '{"label":"governance-check"}')
branch_id=$(jq -r '.branch_id' <<<"$conversation_resp")
if [[ -z "$branch_id" || "$branch_id" == "null" ]]; then
  echo "[error] failed to create conversation" >&2
  print_json "$conversation_resp" >&2
  exit 1
fi

echo "[meta2] governance branch: $branch_id" >&2

read -r -d '' GOVERNANCE_PROMPT <<'PROMPT'
Define an API named "governance_check" with no parameters. The API must:
1. Run the command `cargo fmt --all -- --check` in the repository root and capture stdout/stderr.
2. Run the command `cargo clippy --all-targets -- -D warnings`.
3. Run the command `cargo test`.
4. Verify that the file `conversation.md` exists and contains at least one logged conversation entry.
5. Collect the stdout/stderr of each command and the verification result, and write them to `logs/governance_receipt_<timestamp>.json` (create the directory if necessary). The JSON must contain keys `fmt`, `clippy`, `test`, `conversation_log`, and `receipt_path`, with boolean success flags and captured output strings.
Return the JSON object that was written to disk.
PROMPT

payload_define=$(jq -n --arg prompt "$GOVERNANCE_PROMPT" '{prompt:$prompt}')
response_define=$(curl -s -X POST "$BASE_URL/conversation/$branch_id/prompt" \
  -H 'content-type: application/json' \
  -d "$payload_define")

payload_call=$(jq -n --arg prompt "Call the API 'governance_check'" '{prompt:$prompt}')
response_call=$(curl -s -X POST "$BASE_URL/conversation/$branch_id/prompt" \
  -H 'content-type: application/json' \
  -d "$payload_call")

snapshot=$(curl -s "$BASE_URL/conversation/$branch_id/events")

receipt_path=$(echo "$response_call" | jq -r '.effect.ApiResponse.output | try (fromjson | .receipt_path) catch ""' 2>/dev/null)

{
  echo "# Governance Run $TIMESTAMP"
  echo "Branch ID: $branch_id"
  echo
  echo "## Definition Response"
  print_json "$response_define"
  echo
  echo "## Execution Response"
  print_json "$response_call"
  echo
  echo "## Conversation Snapshot"
  print_json "$snapshot"
  if [[ -n "$receipt_path" && -f "$receipt_path" ]]; then
    echo
    echo "## Receipt Contents ($receipt_path)"
    print_json "$(cat "$receipt_path")"
  fi
} > "$REPORT_FILE"

echo "[meta2] governance report saved to $REPORT_FILE" >&2

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

if [[ -x scripts/log_conversation.sh ]]; then
  printf '%s\n' "$GOVERNANCE_PROMPT" > "$TMPDIR/prompt_define.txt"
  print_json "$response_define" > "$TMPDIR/response_define.json"
  scripts/log_conversation.sh "governance-define" "$branch_id" \
    "$TMPDIR/prompt_define.txt" "$TMPDIR/response_define.json"

  printf '%s\n' "Call the API 'governance_check'" > "$TMPDIR/prompt_call.txt"
  print_json "$response_call" > "$TMPDIR/response_call.json"
  scripts/log_conversation.sh "governance-call" "$branch_id" \
    "$TMPDIR/prompt_call.txt" "$TMPDIR/response_call.json" "$REPORT_FILE"
fi

exit 0
