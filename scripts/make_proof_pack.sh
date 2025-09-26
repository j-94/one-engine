#!/usr/bin/env bash
set -euo pipefail

PACK_ROOT=proofs/pack
mkdir -p "$PACK_ROOT"

RECEIPT=logs/proof.receipt.json
PLAN=docs/plan.svg
ARTEFACT=proofs/HELLO_ENGINE.txt

if [[ ! -f $RECEIPT ]]; then
  echo "Missing receipt: $RECEIPT" >&2
  exit 1
fi

cp "$RECEIPT" "$PACK_ROOT/"
if [[ -f $PLAN ]]; then
  cp "$PLAN" "$PACK_ROOT/"
fi
if [[ -f $ARTEFACT ]]; then
  cp "$ARTEFACT" "$PACK_ROOT/"
fi

git status --porcelain >"$PACK_ROOT/git_status.txt"
git rev-parse HEAD >"$PACK_ROOT/commit.txt"

jq -r '
  ( ["run_id", .run_id],
    ["status", .status],
    ["status_line", .status_line],
    ["execution_time_ms", (.execution_time_ms|tostring)],
    ["operations", ""],
    ["idx","op","success","tokens","cost_usd","duration_ms","note"] ),
  ( .data.operations // [] | to_entries[] | [
      (.key|tostring),
      (.value.metadata.operation_type // "-"),
      (.value.success|tostring),
      (.value.metadata.tokens // "-"),
      (.value.metadata.cost_usd // "-"),
      (.value.duration_ms|tostring),
      (.value.output | tostring | gsub("\\n"; " ") | .[0:120])
    ] )
  | @tsv
' "$RECEIPT" >"$PACK_ROOT/run_summary.tsv"

echo "sha256  proofs/pack/*:" >"$PACK_ROOT/SHA256SUMS.txt"
shasum -a 256 "$PACK_ROOT"/* >>"$PACK_ROOT/SHA256SUMS.txt"

echo "Proof pack ready in $PACK_ROOT/"
