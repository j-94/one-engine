#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="$(mktemp /tmp/one_engine_demo.XXXXXX)"
PORT="127.0.0.1:7777"

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

echo "[meta2] starting engine (logs: $LOG_FILE)"
cargo run >"$LOG_FILE" 2>&1 &
SERVER_PID=$!

for _ in {1..30}; do
  if curl -s "http://$PORT/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

if ! curl -s "http://$PORT/healthz" >/dev/null 2>&1; then
  echo "[error] engine failed to start; check $LOG_FILE" >&2
  exit 1
fi

echo "[meta2] engine is live"

BRANCH_JSON=$(curl -s -X POST "http://$PORT/conversation" \
  -H 'content-type: application/json' \
  -d '{"label":"demo"}')
BRANCH_ID=$(jq -r '.branch_id' <<<"$BRANCH_JSON")

echo "[meta2] branch id: $BRANCH_ID"

DEFINE_PROMPT='Define a simple API named "echo" that accepts a single parameter "text" and returns it unmodified.'
DEFINE_PAYLOAD=$(jq -n --arg prompt "$DEFINE_PROMPT" '{prompt:$prompt}')
DEFINE_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$DEFINE_PAYLOAD")

echo "[meta2] definition response:"
echo "$DEFINE_JSON" | jq

CALL_PROMPT="Call the API 'echo' with text='Hello, World'"
CALL_PAYLOAD=$(jq -n --arg prompt "$CALL_PROMPT" '{prompt:$prompt}')
CALL_JSON=$(curl -s -X POST "http://$PORT/conversation/$BRANCH_ID/prompt" \
  -H 'content-type: application/json' \
  -d "$CALL_PAYLOAD")

echo "[meta2] call response:"
echo "$CALL_JSON" | jq

popd >/dev/null
