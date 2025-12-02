#!/usr/bin/env bash
set -euo pipefail

ENDPOINT="${ONE_ENGINE_URL:-http://127.0.0.1:8080}"
DATASETS=()
CODEX_FLAG=""
DETERMINISTIC_FLAG=""
LABEL="daily"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dataset)
      DATASETS+=("$2")
      shift 2
      ;;
    --codex-only)
      CODEX_FLAG='"codex":true,"deterministic":false,'
      shift
      ;;
    --deterministic-only)
      CODEX_FLAG='"codex":false,'
      DETERMINISTIC_FLAG='"deterministic":true,'
      shift
      ;;
    --label)
      LABEL="$2"
      shift 2
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
end

if [[ ${#DATASETS[@]} -gt 0 ]]; then
  DATASET_JSON=$(printf '"%s",' "${DATASETS[@]}")
  DATASET_JSON="\"datasets\":[${DATASET_JSON%,}],"
else
  DATASET_JSON=""
fi

PAYLOAD="{${DATASET_JSON}${CODEX_FLAG}${DETERMINISTIC_FLAG}\"label_prefix\":\"$LABEL\"}"

curl -s -X POST "$ENDPOINT/telemetry/run" \
  -H 'Content-Type: application/json' \
  -d "$PAYLOAD" | jq
