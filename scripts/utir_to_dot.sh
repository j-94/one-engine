#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <utir_json_path> [output_svg_path]" >&2
  exit 1
fi

UTIR_PATH="$1"
OUTPUT_PATH="${2:-docs/plan.svg}"

if [[ ! -f "$UTIR_PATH" ]]; then
  echo "UTIR file not found: $UTIR_PATH" >&2
  exit 1
fi

python3 - <<'PY' "$UTIR_PATH" "$OUTPUT_PATH"
import json
import math
import sys
from pathlib import Path

utir_path = Path(sys.argv[1])
output_path = Path(sys.argv[2])

utir_json = json.loads(utir_path.read_text(encoding="utf-8"))
raw = utir_json.get("utir", "")

operations = []
current_descriptor = None
for line in raw.splitlines():
    stripped = line.strip()
    if stripped.startswith("- type:"):
        op_type = stripped.split(":", 1)[1].strip().strip('"')
        operations.append({"type": op_type, "descriptor": ""})
        current_descriptor = operations[-1]
    elif current_descriptor and stripped.startswith("command:"):
        current_descriptor["descriptor"] = stripped.split(":", 1)[1].strip().strip('"')
    elif current_descriptor and stripped.startswith("url:"):
        current_descriptor["descriptor"] = stripped.split(":", 1)[1].strip().strip('"')

if not operations:
    raise SystemExit("No operations found in UTIR")

node_width = 300
node_height = 70
v_spacing = 40
margin = 40

height = margin * 2 + len(operations) * node_height + (len(operations) - 1) * v_spacing
width = node_width + margin * 2

svg_parts = [
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
    f"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">",
    "  <style>text { font-family: 'Helvetica', 'Arial', sans-serif; font-size: 14px; }</style>",
]

for idx, op in enumerate(operations):
    y = margin + idx * (node_height + v_spacing)
    x = margin
    svg_parts.append(
        f"  <rect x=\"{x}\" y=\"{y}\" rx=\"10\" ry=\"10\" width=\"{node_width}\" height=\"{node_height}\" fill=\"#eef2ff\" stroke=\"#312e81\" stroke-width=\"2\" />"
    )
    label = f"{idx + 1}. {op['type']}"
    svg_parts.append(
        f"  <text x=\"{x + 16}\" y=\"{y + 24}\" fill=\"#1e1b4b\">{label}</text>"
    )
    if op.get("descriptor"):
        descriptor = op["descriptor"]
        if len(descriptor) > 50:
            descriptor = descriptor[:47] + "…"
        svg_parts.append(
            f"  <text x=\"{x + 16}\" y=\"{y + 46}\" fill=\"#4338ca\">{descriptor}</text>"
        )
    if idx < len(operations) - 1:
        next_y = y + node_height
        svg_parts.append(
            f"  <line x1=\"{x + node_width / 2}\" y1=\"{next_y}\" x2=\"{x + node_width / 2}\" y2=\"{next_y + v_spacing}\" stroke=\"#312e81\" stroke-width=\"2\" marker-end=\"url(#arrow)\" />"
        )

svg_parts.insert(2, "  <defs><marker id=\"arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"5\" refY=\"5\" orient=\"auto-start-reverse\"><path d=\"M0,0 L10,5 L0,10 z\" fill=\"#312e81\"/></marker></defs>")
svg_parts.append("</svg>")

output_path.parent.mkdir(parents=True, exist_ok=True)
output_path.write_text("\n".join(svg_parts), encoding="utf-8")
PY
