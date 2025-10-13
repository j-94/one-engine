#!/usr/bin/env bash
set -euo pipefail

# Programmatically seed the engine with multiple define/approve/call prompts
# to populate the Graph UI with more endpoints and event structure.
#
# Usage:
#   ./scripts/seed_graph_demo.sh
#
# Requirements:
#   - Engine running at $ENGINE_BASE_URL (default http://127.0.0.1:7777)
#   - jq installed

ENGINE_BASE_URL="${ENGINE_BASE_URL:-http://127.0.0.1:7777}"

if ! command -v jq >/dev/null 2>&1; then
  echo "[error] jq is required (e.g., brew install jq)" >&2
  exit 1
fi

if ! curl -s "$ENGINE_BASE_URL/healthz" >/dev/null 2>&1; then
  echo "[error] engine not reachable at $ENGINE_BASE_URL" >&2
  echo "        run: cargo run" >&2
  exit 1
fi

# Create a new conversation branch
BRANCH_JSON=$(curl -s -X POST "$ENGINE_BASE_URL/conversation" \
  -H 'content-type: application/json' \
  -d '{"label": "seed-graph"}')
BRANCH_ID=$(jq -r '.branch_id' <<<"$BRANCH_JSON")
if [[ -z "$BRANCH_ID" || "$BRANCH_ID" == "null" ]]; then
  echo "[error] failed to start conversation" >&2
  echo "$BRANCH_JSON" >&2
  exit 1
fi
mkdir -p out_one_engine
printf '%s\n' "$BRANCH_ID" > out_one_engine/branch_id.txt

echo "[seed] branch: $BRANCH_ID"

post_prompt() {
  local p="$1"
  curl -s -X POST "$ENGINE_BASE_URL/conversation/$BRANCH_ID/prompt" \
    -H 'content-type: application/json' \
    -d "$(jq -n --arg prompt "$p" '{prompt:$prompt}')" >/dev/null
  printf '. '
}

# Curated endpoints to create/approve/call
DEFINES=(
  "Define a persistent API named 'uppercase' that accepts 'text' and returns it in uppercase."
  "Define a persistent API named 'reverse' that accepts 'text' and returns the reversed string."
  "Define a persistent API named 'slugify' that accepts 'text' and returns a URL-safe slug."
  "Define a persistent API named 'replace' that accepts 'text', 'from', and 'to' and returns the text with occurrences of 'from' replaced by 'to'."
  "Define a persistent API named 'concat' that accepts 'a' and 'b' and returns their concatenation."
  "Define a persistent API named 'counter' that increments an internal counter and returns it each call."
)

APPROVES=(uppercase reverse slugify replace concat counter)

CALLS=(
  "Call the API 'uppercase' with text='Warp rocks'"
  "Call the API 'reverse' with text='ruliad'"
  "Call the API 'slugify' with text='One Engine: Graph UI'"
  "Call the API 'replace' with text='hello world' from='world' to='Ruliad'"
  "Call the API 'concat' with a='fractal' b='intelligence'"
  "Call the API 'counter'"
  "Call the API 'counter'"
)

echo "[seed] defining endpoints"
for d in "${DEFINES[@]}"; do post_prompt "$d"; done; echo

echo "[seed] approving endpoints"
for a in "${APPROVES[@]}"; do post_prompt "Approve pattern '$a'"; done; echo

echo "[seed] calling endpoints"
for c in "${CALLS[@]}"; do post_prompt "$c"; done; echo

# Optional: show a brief summary
SNAP=$(curl -s "$ENGINE_BASE_URL/conversation/$BRANCH_ID/events")
TOTAL=$(jq '.events | length' <<<"$SNAP")
CALLED=$(jq '[.events[] | select(.ApiCalled!=null)] | length' <<<"$SNAP")
GEN=$(jq '[.events[] | select(.ApiGenerated!=null)] | length' <<<"$SNAP")
APPR=$(jq '[.events[] | select(.ParsedIntent.description // "" | test("ApprovePattern|approval:"))] | length' <<<"$SNAP")

echo "[seed] events: total=$TOTAL generated=$GEN approvals=$APPR calls=$CALLED"
echo "[seed] done. Use Graph UI with ENGINE_BRANCH_ID=$BRANCH_ID and click Load/Reload."
