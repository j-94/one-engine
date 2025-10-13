#!/usr/bin/env bash
set -euo pipefail

# CI User Journey: spins up the engine, runs a conversational flow,
# validates success, and writes a status report. Exits non-zero on failure.
#
# This script is designed to run in CI (Ubuntu) and locally.
# It requires `jq` on PATH.
#
# Steps:
# 1) Start engine (background), wait for /healthz
# 2) Create conversation branch
# 3) Define persistent 'uppercase'
# 4) Approve pattern 'uppercase'
# 5) Call the API 'uppercase' with text='CI pass'
# 6) Validate autodoc and events
# 7) Write logs/ci_user_journey_report.md and exit 0 on success

BASE_URL="${ENGINE_BASE_URL:-http://127.0.0.1:7777}"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$PROJECT_ROOT/logs"
ENGINE_LOG="$(mktemp -t one_engine_ci_server)"
REPORT_MD="$LOG_DIR/ci_user_journey_report.md"
BRANCH_FILE="$PROJECT_ROOT/out_one_engine/branch_id.txt"

mkdir -p "$LOG_DIR" "$PROJECT_ROOT/out_one_engine"

cleanup() {
  if [[ -n "${ENGINE_PID:-}" ]]; then
    kill "$ENGINE_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

ensure_jq() {
  if ! command -v jq >/dev/null 2>&1; then
    echo "[error] jq is required on PATH" >&2
    exit 1
  fi
}

start_engine() {
  if curl -s "$BASE_URL/healthz" >/dev/null 2>&1; then
    echo "[meta2] engine already running at $BASE_URL"
    return
  fi
  echo "[meta2] starting engine (logs: $ENGINE_LOG)"
  (cd "$PROJECT_ROOT" && nohup cargo run >"$ENGINE_LOG" 2>&1 &)
  ENGINE_PID=$!
  for _ in {1..60}; do
    if curl -s "$BASE_URL/healthz" >/dev/null 2>&1; then
      echo "[meta2] engine is live"
      return
    fi
    sleep 0.5
  done
  echo "[error] engine did not respond at $BASE_URL; see $ENGINE_LOG" >&2
  exit 1
}

api() { # method path json_body(optional)
  local method="$1"; shift
  local path="$1"; shift
  local body="${1:-}"
  if [[ -n "$body" ]]; then
    curl -s -X "$method" "$BASE_URL$path" -H 'content-type: application/json' -d "$body"
  else
    curl -s -X "$method" "$BASE_URL$path"
  fi
}

main() {
  ensure_jq
  start_engine

  echo "[step] create conversation"
  convo=$(api POST "/conversation" '{"label":"ci"}') || { echo "[error] convo failed" >&2; exit 1; }
  branch_id=$(jq -r '.branch_id' <<<"$convo")
  if [[ -z "$branch_id" || "$branch_id" == "null" ]]; then
    echo "[error] no branch_id" >&2
    echo "$convo" >&2
    exit 1
  fi
  printf '%s\n' "$branch_id" > "$BRANCH_FILE"

  # 1) ephemeral define (to trigger generation)
  echo "[step] define ephemeral 'uppercase'"
  define_e="Define a simple API named 'uppercase' that accepts a single parameter 'text' and returns it in uppercase."
  define_e_resp=$(api POST "/conversation/$branch_id/prompt" "$(jq -n --arg p "$define_e" '{prompt:$p}')") || true
  sleep 0.2

  # 2) call ephemeral to ensure ApiCalled and validate runnable logic
  echo "[step] call ephemeral 'uppercase'"
  call_e="Call the API 'uppercase' with text='CI pass'"
  call_e_resp=$(api POST "/conversation/$branch_id/prompt" "$(jq -n --arg p "$call_e" '{prompt:$p}')") || true
  sleep 0.2

  # 3) persistent define (to persist capability)
  echo "[step] define persistent 'uppercase'"
  define_p="Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase."
  define_p_resp=$(api POST "/conversation/$branch_id/prompt" "$(jq -n --arg p "$define_p" '{prompt:$p}')") || true
  sleep 0.2

  # 4) approve pattern
  echo "[step] approve pattern 'uppercase'"
  approve_p="Approve pattern 'uppercase'"
  approve_resp=$(api POST "/conversation/$branch_id/prompt" "$(jq -n --arg p "$approve_p" '{prompt:$p}')") || true
  sleep 0.2

  # 5) call persisted
  echo "[step] call persisted 'uppercase'"
  call_p="Call the API 'uppercase' with text='CI pass'"
  call_resp=$(api POST "/conversation/$branch_id/prompt" "$(jq -n --arg p "$call_p" '{prompt:$p}')") || true
  sleep 0.2

  echo "[step] fetch events"
  events=$(api GET "/conversation/$branch_id/events")

  echo "[step] fetch autodoc names"
  names=$(api GET "/autodoc/$branch_id/names")

  # Validate criteria (autodoc may be unavailable in some builds)
  total=$(jq -r '.events | length' <<<"$events" 2>/dev/null || echo 0)
  gen_ct=$(jq -r '[.events[] | select(.ApiGenerated!=null)] | length' <<<"$events" 2>/dev/null || echo 0)
  call_ct=$(jq -r '[.events[] | select(.ApiCalled!=null)] | length' <<<"$events" 2>/dev/null || echo 0)
  appr_ct=$(jq -r '[.events[] | (.ParsedIntent.description // "") | select(test("ApprovePattern|approval:"))] | length' <<<"$events" 2>/dev/null || echo 0)
  # Verify final output
  final_ok=$(jq -r '.effect.ApiResponse.output // empty' <<<"$call_resp" 2>/dev/null | grep -c "CI PASS" || true)
  # Verify memory (persisted name in autodoc names)
  mem_ok=$(jq -r '.[]? // empty' <<<"$names" 2>/dev/null | grep -c "^uppercase$" || true)

  status="success"
  fail_reasons=()
  if [[ "$gen_ct" -lt 1 ]]; then status="failure"; fail_reasons+=("no ApiGenerated events"); fi
  if [[ "$call_ct" -lt 1 ]]; then status="failure"; fail_reasons+=("no ApiCalled events"); fi
  if [[ "$appr_ct" -lt 1 ]]; then status="failure"; fail_reasons+=("no approval evidence"); fi
  if [[ "$final_ok" -lt 1 ]]; then status="failure"; fail_reasons+=("final call output mismatch"); fi
  if [[ "$mem_ok" -lt 1 ]]; then status="failure"; fail_reasons+=("uppercase not in autodoc names"); fi

  {
    echo "# CI User Journey Report"
    echo
    echo "- Base URL: $BASE_URL"
    echo "- Branch ID: $branch_id"
    echo "- Engine log: $ENGINE_LOG"
    echo
    echo "## Steps"
    echo "1. Define persistent 'uppercase'"
    echo "2. Approve pattern 'uppercase'"
    echo "3. Call 'uppercase' with text='CI pass'"
    echo
    echo "## Events Summary"
    echo "- total: $total"
    echo "- generated: $gen_ct"
    echo "- approvals: $appr_ct"
    echo "- calls: $call_ct"
    echo
    echo "## Memory"
    echo "$names" | jq -c '.'
    echo
    echo "## Status"
    if [[ "$status" == "success" ]]; then
      echo "- result: ✅ success"
    else
      echo "- result: ❌ failure"
      echo "- reasons: ${fail_reasons[*]}"
    fi
    echo
    echo "## Raw Responses (truncated)"
    echo "### define ephemeral response"
    echo '```'
    echo "$define_e_resp" | head -c 4000
    echo
    echo '```'
    echo "### call ephemeral response"
    echo '```'
    echo "$call_e_resp" | head -c 4000
    echo
    echo '```'
    echo "### define persistent response"
    echo '```'
    echo "$define_p_resp" | head -c 4000
    echo
    echo '```'
    echo "### approve response"
    echo '```'
    echo "$approve_resp" | head -c 4000
    echo
    echo '```'
    echo "### call persisted response"
    echo '```'
    echo "$call_resp" | head -c 4000
    echo
    echo '```'
  } > "$REPORT_MD"

  echo "[meta2] report saved: $REPORT_MD"
  if [[ "$status" != "success" ]]; then
    echo "[error] CI user journey failed: ${fail_reasons[*]}" >&2
    exit 2
  fi
}

main "$@"
