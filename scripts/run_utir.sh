#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <utir_json_path> [--host http://localhost:7777]" >&2
  exit 1
fi

UTIR_PATH="$1"
shift || true
ENGINE_URL="http://localhost:7777"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      ENGINE_URL="${2%/}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ ! -f "$UTIR_PATH" ]]; then
  echo "UTIR file not found: $UTIR_PATH" >&2
  exit 1
fi

if ! command -v curl >/dev/null; then
  echo "curl is required" >&2
  exit 1
fi

if ! command -v jq >/dev/null; then
  echo "jq is required" >&2
  exit 1
fi

HEALTH_OK=false
for _ in {1..10}; do
  if curl -sS --fail "$ENGINE_URL/healthz" >/dev/null; then
    HEALTH_OK=true
    break
  fi
  sleep 1
done

if [[ "$HEALTH_OK" != true ]]; then
  echo "Engine at $ENGINE_URL is not reachable (health check failed)" >&2
  exit 2
fi

TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
RESPONSE_FILE="$(mktemp)"
trap 'rm -f "$RESPONSE_FILE"' EXIT

curl -sS -X POST "$ENGINE_URL/compile_and_run" \
  -H 'Content-Type: application/json' \
  --data @"$UTIR_PATH" >"$RESPONSE_FILE"

python3 - <<'PY' "$UTIR_PATH" "$RESPONSE_FILE" "$TIMESTAMP" "$ENGINE_URL"
import json
import hashlib
import sys
from pathlib import Path

utir_path = Path(sys.argv[1])
response_path = Path(sys.argv[2])
timestamp = sys.argv[3]
engine_url = sys.argv[4]

request = json.loads(utir_path.read_text(encoding="utf-8"))
response_raw = response_path.read_text(encoding="utf-8").strip()

if not response_raw:
    raise SystemExit("Empty response from engine")

try:
    response = json.loads(response_raw)
except json.JSONDecodeError as exc:
    raise SystemExit(f"Invalid JSON response: {exc}: {response_raw[:200]}")

utir_text = request.get("utir", "")
utir_hash = hashlib.sha256(utir_text.encode("utf-8")).hexdigest()

data = response.get("data") or {}
operations = data.get("operations") or []
steps = []
blocked = []
for op in operations:
    step = {
        "phase": op.get("phase", "execution"),
        "op": op.get("op", "step"),
        "operation_type": op.get("operation_type", "unknown"),
        "status": op.get("status", "unknown"),
        "success": bool(op.get("success", False)),
        "tokens": op.get("token_estimate", 0),
        "notes": op.get("descriptor", ""),
        "duration_ms": op.get("duration_ms", 0),
        "bits": op.get("bits", {}),
        "metadata": op.get("metadata", {}),
        "output": op.get("output", ""),
        "output_truncated": op.get("output_truncated", False),
    }
    steps.append(step)
    if not step["success"]:
        blocked.append(step["op"])

receipt = {
    "ts": timestamp,
    "engine_url": engine_url,
    "utir_path": str(utir_path),
    "utir_hash": utir_hash,
    "utir_bytes": len(utir_text.encode("utf-8")),
    "token_estimate_total": data.get("token_estimate_total", sum(s["tokens"] for s in steps)),
    "engine": {
        "run_id": response.get("run_id"),
        "status": response.get("status"),
        "status_line": response.get("status_line"),
        "execution_time_ms": response.get("execution_time_ms"),
        "bits": response.get("bits", {}),
    },
    "plan": {
        "task_id": data.get("task_id"),
        "operations_count": data.get("operations_count", 0),
        "total_duration_ms": data.get("total_duration_ms", 0),
        "pattern_signature": data.get("pattern_signature"),
        "crystallized": data.get("crystallized", False),
    },
    "steps": steps,
    "safety": {
        "status": "triggered" if blocked else "clear",
        "blocked_steps": blocked,
    },
    "request": request,
    "response": response,
}

json.dump(receipt, sys.stdout, indent=2, sort_keys=True)
PY
