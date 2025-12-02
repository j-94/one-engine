#!/usr/bin/env bash
set -euo pipefail

# Emergence demo for one-engine
# - Starts the engine
# - Runs a conversational flow: Define -> Call -> Persist -> Approve -> Call
# - Captures event traces and logs prompts/responses into conversation.md
#
# Requirements:
#   - jq installed (brew install jq)
#   - Port 7777 available (or adjust PORT below)

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="127.0.0.1:7777"
LOG_FILE="${ENGINE_LOG_PATH:-$(mktemp -t one_engine_emergence)}"
SERVER_PID=""

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

pushd "$PROJECT_ROOT" >/dev/null

if ! command -v jq >/dev/null 2>&1; then
  echo "[error] jq is required (e.g., brew install jq)" >&2
  exit 1
fi

if lsof -ti :7777 >/dev/null 2>&1; then
  echo "[error] something is already listening on $PORT" >&2
  exit 1
fi

echo "[emergence] starting engine (logs: $LOG_FILE)"
cargo run >"$LOG_FILE" 2>&1 &
SERVER_PID=$!

# Wait for server
for _ in {1..40}; do
  if curl -s "http://$PORT/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

if ! curl -s "http://$PORT/healthz" >/dev/null 2>&1; then
  echo "[error] engine failed to start; check $LOG_FILE" >&2
  exit 1
fi

echo "[emergence] engine is live"

# 1) Start conversation (agentic feedback loop begins)
BRANCH_JSON=$(curl -s -X POST "http://$PORT/conversation" -H 'content-type: application/json' -d '{"label":"emergence"}')
BRANCH_ID=$(jq -r '.branch_id' <<<"$BRANCH_JSON")
echo "[emergence] branch id: $BRANCH_ID"

# 2) Define API (CreateApi -> ApiGenerated)
DEFINE_PROMPT="Define a simple API named 'echo' that accepts a single parameter 'text' and returns it unmodified."
DEFINE_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$(jq -n --arg prompt "$DEFINE_PROMPT" '{prompt:$prompt}')")

echo "[emergence] definition response:"
echo "$DEFINE_JSON" | jq '.effect'

# Log to conversation.md
printf "%s\n" "$DEFINE_PROMPT" > /tmp/p.define.txt
printf "%s\n" "$DEFINE_JSON" > /tmp/r.define.json
"$PROJECT_ROOT/scripts/log_conversation.sh" "emergence-define" "$BRANCH_ID" /tmp/p.define.txt /tmp/r.define.json

# 3) Call API (CallApi -> ApiCalled -> ApiResponse)
CALL_PROMPT="Call the API 'echo' with text='Hello, World'"
CALL_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$(jq -n --arg prompt "$CALL_PROMPT" '{prompt:$prompt}')")

echo "[emergence] call response:"
echo "$CALL_JSON" | jq '.effect'

# Log
printf "%s\n" "$CALL_PROMPT" > /tmp/p.call1.txt
printf "%s\n" "$CALL_JSON" > /tmp/r.call1.json
"$PROJECT_ROOT/scripts/log_conversation.sh" "emergence-call1" "$BRANCH_ID" /tmp/p.call1.txt /tmp/r.call1.json

# 4) Evolve goal to persistence (parser detects persistence)
PERSIST_PROMPT="Define a persistent API named 'echo' that accepts 'text' and returns it unmodified."
PERSIST_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$(jq -n --arg prompt "$PERSIST_PROMPT" '{prompt:$prompt}')")

echo "[emergence] persist definition response:"
echo "$PERSIST_JSON" | jq '.effect'

# Log
printf "%s\n" "$PERSIST_PROMPT" > /tmp/p.persist.txt
printf "%s\n" "$PERSIST_JSON" > /tmp/r.persist.json
"$PROJECT_ROOT/scripts/log_conversation.sh" "emergence-persist" "$BRANCH_ID" /tmp/p.persist.txt /tmp/r.persist.json

# 5) HITL governance: approve pattern (ApprovePattern)
APPROVE_PROMPT="Approve pattern 'echo'"
APPROVE_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$(jq -n --arg prompt "$APPROVE_PROMPT" '{prompt:$prompt}')")

echo "[emergence] approval response:"
echo "$APPROVE_JSON" | jq '.effect'

# Log
printf "%s\n" "$APPROVE_PROMPT" > /tmp/p.approve.txt
printf "%s\n" "$APPROVE_JSON" > /tmp/r.approve.json
"$PROJECT_ROOT/scripts/log_conversation.sh" "emergence-approve" "$BRANCH_ID" /tmp/p.approve.txt /tmp/r.approve.json

# 6) Call again with variant text
CALL2_PROMPT="Call the API 'echo' with text='Calibrated output'"
CALL2_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$(jq -n --arg prompt "$CALL2_PROMPT" '{prompt:$prompt}')")

echo "[emergence] call2 response:"
echo "$CALL2_JSON" | jq '.effect'

# Log
printf "%s\n" "$CALL2_PROMPT" > /tmp/p.call2.txt
printf "%s\n" "$CALL2_JSON" > /tmp/r.call2.json
"$PROJECT_ROOT/scripts/log_conversation.sh" "emergence-call2" "$BRANCH_ID" /tmp/p.call2.txt /tmp/r.call2.json

# 7) Event snapshot (evidence of agentic feedback loop)
echo "[emergence] branch events:"
curl -s "http://$PORT/conversation/$BRANCH_ID/events" | jq '{branch_id, label, events}'

echo "[emergence] demo complete (engine log: $LOG_FILE)"

popd >/dev/null
