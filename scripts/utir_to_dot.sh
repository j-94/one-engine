#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <utir-file> [output-svg]" >&2
  exit 1
fi

UTIR_FILE=$1
OUTPUT_PATH=${2:-docs/plan.svg}
DOT_FALLBACK=${OUTPUT_PATH%.*}.dot

if [[ ! -f $UTIR_FILE ]]; then
  echo "UTIR file not found: $UTIR_FILE" >&2
  exit 1
fi

TMP_DOT=$(mktemp)
trap 'rm -f "$TMP_DOT"' EXIT

cargo run --quiet --bin utir_to_dot -- "$UTIR_FILE" >"$TMP_DOT"

mkdir -p "$(dirname "$OUTPUT_PATH")"

if command -v dot >/dev/null 2>&1; then
  dot -Tsvg "$TMP_DOT" -o "$OUTPUT_PATH"
  if [[ -n ${KEEP_DOT:-} ]]; then
    cp "$TMP_DOT" "$DOT_FALLBACK"
  fi
else
  echo "Graphviz 'dot' not available; saving DOT to $DOT_FALLBACK" >&2
  cp "$TMP_DOT" "$DOT_FALLBACK"
fi
