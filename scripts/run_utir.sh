#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <utir-file> [output-json]" >&2
  exit 1
fi

UTIR_SOURCE=$1
OUTPUT_PATH=${2:-}
ENGINE_URL=${ENGINE_URL:-http://localhost:7777}
WORKSPACE_ABS=$(realpath "${ENGINE_WORKSPACE:-$(pwd)}")

if [[ ! -f $UTIR_SOURCE ]]; then
  echo "UTIR file not found: $UTIR_SOURCE" >&2
  exit 1
fi

if command -v jq >/dev/null 2>&1; then
  if jq -e '.utir' "$UTIR_SOURCE" >/dev/null 2>&1; then
    UTIR_PAYLOAD=$(jq -r '.utir' "$UTIR_SOURCE")
  else
    UTIR_PAYLOAD=$(cat "$UTIR_SOURCE")
  fi
else
  echo "jq is required to run this script" >&2
  exit 1
fi

SANITIZED_PAYLOAD=${UTIR_PAYLOAD//\{\{WORKSPACE\}\}/$WORKSPACE_ABS}

JSON_BODY=$(printf '%s' "$SANITIZED_PAYLOAD" | jq -Rs '{utir: .}')

CURL_ARGS=("-sS" "-X" "POST" "${ENGINE_URL%/}/compile_and_run" "-H" "Content-Type: application/json" "-d" "$JSON_BODY")

if [[ -n ${ENGINE_API_KEY:-} ]]; then
  CURL_ARGS+=("-H" "X-API-Key: $ENGINE_API_KEY")
fi

RESPONSE=$(curl "${CURL_ARGS[@]}")
STATUS=$?
if [[ $STATUS -ne 0 ]]; then
  echo "Failed to invoke engine" >&2
  exit $STATUS
fi

if [[ -n $OUTPUT_PATH ]]; then
  mkdir -p "$(dirname "$OUTPUT_PATH")"
  printf '%s' "$RESPONSE" >"$OUTPUT_PATH"
else
  printf '%s\n' "$RESPONSE"
fi
