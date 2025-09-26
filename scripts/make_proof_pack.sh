#!/usr/bin/env bash
set -euo pipefail

PROOF_DIR="proofs/pack"
RECEIPT_PATH="logs/proof.receipt.json"
PLAN_PATH="docs/plan.svg"
HELLO_FILE="proofs/HELLO_ENGINE.txt"
HELLO_SUMMARY="proofs/HELLO_ENGINE_SUMMARY.txt"

mkdir -p "$PROOF_DIR"

if [[ -f "$RECEIPT_PATH" ]]; then
  cp -v "$RECEIPT_PATH" "$PROOF_DIR/" >&2 || true
else
  echo "Warning: receipt not found at $RECEIPT_PATH" >&2
fi

if [[ -f "$PLAN_PATH" ]]; then
  cp -v "$PLAN_PATH" "$PROOF_DIR/" >&2 || true
else
  echo "Warning: plan svg not found at $PLAN_PATH" >&2
fi

if [[ -f "$HELLO_FILE" ]]; then
  cp -v "$HELLO_FILE" "$PROOF_DIR/" >&2 || true
else
  echo "Warning: proof output file missing at $HELLO_FILE" >&2
fi

if [[ -f "$HELLO_SUMMARY" ]]; then
  cp -v "$HELLO_SUMMARY" "$PROOF_DIR/" >&2 || true
fi

# Capture repo state for reproducibility
git status --porcelain > "$PROOF_DIR/git_status.txt"
git rev-parse HEAD > "$PROOF_DIR/commit.txt"

# Build a TSV summary of the run if we have a receipt
if [[ -f "$RECEIPT_PATH" ]]; then
  jq -r '
    . as $root |
    ["ts","phase","op","status","tokens","notes"],
    ($root.steps[]? | [$root.ts, .phase, .op, .status, (.tokens // 0), (.notes // "")])
    | @tsv
  ' "$RECEIPT_PATH" > "$PROOF_DIR/run_summary.tsv"
fi

# Hash everything we collected
if command -v shasum >/dev/null; then
  shasum -a 256 "$PROOF_DIR"/* > "$PROOF_DIR/SHA256SUMS.txt"
else
  sha256sum "$PROOF_DIR"/* > "$PROOF_DIR/SHA256SUMS.txt"
fi

echo "Proof pack ready in $PROOF_DIR" >&2
