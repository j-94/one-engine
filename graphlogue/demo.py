"""Sample events for the `graphlogue demo` command."""
from __future__ import annotations

from datetime import datetime, timezone
from typing import Dict, List
import uuid


def demo_events() -> List[Dict[str, object]]:
    """Return a deterministic demo timeline."""

    run_id = "demo-run"
    timestamp = datetime.now(timezone.utc).isoformat()
    run_node_id = f"run-{uuid.uuid4().hex[:8]}"
    plan_id = f"plan-{uuid.uuid4().hex[:8]}"
    write_id = f"write-{uuid.uuid4().hex[:8]}"
    receipt_id = f"receipt-{uuid.uuid4().hex[:8]}"
    link_id = f"deliverable-{uuid.uuid4().hex[:8]}"
    kpi_id = f"kpi-{uuid.uuid4().hex[:8]}"
    deeplink_id = f"deeplink-{uuid.uuid4().hex[:8]}"
    table_id = f"table-{uuid.uuid4().hex[:8]}"

    events: List[Dict[str, object]] = [
        {
            "kind": "run.start",
            "label": "Demo run started",
            "run_id": run_id,
            "id": run_node_id,
            "ts": timestamp,
        },
        {
            "kind": "plan",
            "label": "Plan the work",
            "run_id": run_id,
            "parent_id": run_node_id,
            "id": plan_id,
            "ts": timestamp,
        },
        {
            "kind": "file.write",
            "label": "Write README badge",
            "run_id": run_id,
            "parent_id": plan_id,
            "path": "README.md",
            "id": write_id,
            "ts": timestamp,
        },
        {
            "kind": "receipt.write",
            "label": "Recorded file receipt",
            "run_id": run_id,
            "path": "receipts/demo.json",
            "parent_id": write_id,
            "id": receipt_id,
            "ts": timestamp,
        },
        {
            "kind": "deliverable",
            "type": "link",
            "label": "Demo pull request",
            "url": "https://example.com/demo-pr",
            "run_id": run_id,
            "parent_id": plan_id,
            "id": link_id,
            "ts": timestamp,
        },
        {
            "kind": "kpi",
            "label": "decision_agreement",
            "value": 0.93,
            "run_id": run_id,
            "parent_id": run_node_id,
            "id": kpi_id,
            "ts": timestamp,
        },
        {
            "kind": "deeplink",
            "label": "Open run logs",
            "url": "https://example.com/runs/demo-run",
            "run_id": run_id,
            "parent_id": run_node_id,
            "id": deeplink_id,
            "ts": timestamp,
        },
        {
            "kind": "deliverable",
            "type": "table",
            "label": "KPI Summary",
            "md": "| metric | value |\n| --- | --- |\n| decision_agreement | 0.93 |",
            "run_id": run_id,
            "parent_id": kpi_id,
            "id": table_id,
            "ts": timestamp,
        },
    ]
    return events
