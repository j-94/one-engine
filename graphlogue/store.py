"""Persistent storage helpers for the Graphlogue terminal tooling."""

from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import uuid


@dataclass
class NodeRecord:
    """A graph node captured from a streaming event."""

    id: str
    kind: str
    label: str
    run_id: str
    json_blob: str
    timestamp: str = ""


@dataclass
class EdgeRecord:
    """A parent/child edge in the run graph."""

    parent_id: str
    child_id: str
    run_id: str
    kind: str


@dataclass
class DeliverableRecord:
    """A surfaced artifact for stakeholders."""

    id: str
    kind: str
    label: str
    value: str
    run_id: str
    json_blob: str


class GraphlogueStore:
    """Stores nodes, edges, and deliverables in TSV files."""

    def __init__(self, root: Optional[Path] = None) -> None:
        self.root = Path(root or Path.cwd())
        self.data_dir = self.root / ".graphlogue"
        self.data_dir.mkdir(parents=True, exist_ok=True)

        self.nodes_path = self.data_dir / "nodes.tsv"
        self.edges_path = self.data_dir / "edges.tsv"
        self.deliverables_path = self.data_dir / "deliverables.tsv"

        self._nodes: Dict[str, NodeRecord] = {}
        self._edges: Dict[Tuple[str, str], EdgeRecord] = {}
        self._deliverables: Dict[str, DeliverableRecord] = {}
        self._counter = 0

        self._load()

    # ------------------------------------------------------------------
    # public API
    # ------------------------------------------------------------------
    def reset(self) -> None:
        """Remove cached data and clear the TSV files."""

        self._nodes.clear()
        self._edges.clear()
        self._deliverables.clear()
        for path in [self.nodes_path, self.edges_path, self.deliverables_path]:
            if path.exists():
                path.unlink()
        self._counter = 0

    def register_event(self, event: Dict[str, object]) -> NodeRecord:
        """Store a raw event as a graph node.

        Returns the :class:`NodeRecord` that was persisted. This method also
        collects deliverables and edges when the event payload contains the
        corresponding metadata.
        """

        run_id = _as_str(
            event.get("run_id")
            or event.get("run")
            or event.get("runId")
            or event.get("thread_id")
            or "unknown"
        )
        kind = _as_str(event.get("kind") or event.get("event") or event.get("type") or "event")
        node_id = _as_str(
            event.get("node_id")
            or event.get("id")
            or event.get("event_id")
            or event.get("uuid")
        )

        if not node_id:
            node_id = f"{run_id}:{kind}:{self._next_counter()}"

        label = _first_non_empty(
            event.get("label"),
            event.get("summary"),
            event.get("message"),
            event.get("title"),
            event.get("name"),
        ) or kind.replace("_", " ").title()

        timestamp = _as_str(event.get("ts") or event.get("timestamp") or event.get("time") or "")
        json_blob = json.dumps(event, sort_keys=True, ensure_ascii=False)

        record = NodeRecord(
            id=node_id,
            kind=kind,
            label=_as_str(label),
            run_id=_as_str(run_id),
            timestamp=timestamp,
            json_blob=json_blob,
        )

        self._nodes[node_id] = record
        self._write_nodes()

        parent_id = _as_str(
            event.get("parent_id")
            or event.get("parent")
            or event.get("in_reply_to")
            or event.get("preceding")
        )
        if parent_id:
            edge = EdgeRecord(
                parent_id=parent_id,
                child_id=node_id,
                run_id=record.run_id,
                kind=kind,
            )
            self._edges[(parent_id, node_id)] = edge
            self._write_edges()

        self._maybe_record_deliverable(event, record)

        return record

    def list_nodes(self) -> List[NodeRecord]:
        return sorted(self._nodes.values(), key=lambda node: node.timestamp)

    def list_edges(self) -> List[EdgeRecord]:
        return list(self._edges.values())

    def list_deliverables(self) -> List[DeliverableRecord]:
        return sorted(
            self._deliverables.values(),
            key=lambda deliverable: (deliverable.run_id, deliverable.label),
        )

    def export_markdown(self, destination: Path) -> None:
        """Write a stakeholder-friendly deliverables report."""

        deliverables = self.list_deliverables()
        lines: List[str] = []
        lines.append("# Run Deliverables")
        lines.append("")
        lines.append(
            f"Generated from {len(deliverables)} artifact(s)."
        )
        lines.append("")
        if not deliverables:
            lines.append("No deliverables captured yet.")
        else:
            lines.append("| Run | Type | Label | Target |")
            lines.append("| --- | --- | --- | --- |")
            for deliverable in deliverables:
                target = _render_target(deliverable)
                lines.append(
                    f"| {deliverable.run_id or '-'} | {deliverable.kind} | {deliverable.label} | {target} |"
                )
            lines.append("")
            lines.append("## Detailed Artifacts")
            lines.append("")
            for deliverable in deliverables:
                lines.extend(_render_detail(deliverable))
                lines.append("")

        destination.write_text("\n".join(lines) + "\n", encoding="utf-8")

    # ------------------------------------------------------------------
    # internal helpers
    # ------------------------------------------------------------------
    def _load(self) -> None:
        self._nodes = self._read_nodes()
        self._edges = self._read_edges()
        self._deliverables = self._read_deliverables()
        if self._nodes or self._deliverables:
            self._counter = max(len(self._nodes), len(self._deliverables))
        else:
            self._counter = 0

    def _read_nodes(self) -> Dict[str, NodeRecord]:
        rows: Dict[str, NodeRecord] = {}
        if not self.nodes_path.exists():
            return rows
        with self.nodes_path.open("r", encoding="utf-8") as handle:
            for raw_line in handle:
                line = raw_line.rstrip("\n")
                if not line:
                    continue
                parts = line.split("\t")
                if len(parts) < 5:
                    continue
                node = NodeRecord(
                    id=parts[0],
                    kind=parts[1],
                    label=parts[2],
                    run_id=parts[3],
                    json_blob=parts[4],
                    timestamp=parts[5] if len(parts) > 5 else "",
                )
                rows[node.id] = node
        return rows

    def _read_edges(self) -> Dict[Tuple[str, str], EdgeRecord]:
        rows: Dict[Tuple[str, str], EdgeRecord] = {}
        if not self.edges_path.exists():
            return rows
        with self.edges_path.open("r", encoding="utf-8") as handle:
            for raw_line in handle:
                line = raw_line.rstrip("\n")
                if not line:
                    continue
                parts = line.split("\t")
                if len(parts) < 4:
                    continue
                edge = EdgeRecord(
                    parent_id=parts[0],
                    child_id=parts[1],
                    run_id=parts[2],
                    kind=parts[3],
                )
                rows[(edge.parent_id, edge.child_id)] = edge
        return rows

    def _read_deliverables(self) -> Dict[str, DeliverableRecord]:
        rows: Dict[str, DeliverableRecord] = {}
        if not self.deliverables_path.exists():
            return rows
        with self.deliverables_path.open("r", encoding="utf-8") as handle:
            for raw_line in handle:
                line = raw_line.rstrip("\n")
                if not line:
                    continue
                parts = line.split("\t")
                if len(parts) < 5:
                    continue
                record = DeliverableRecord(
                    id=parts[0],
                    kind=parts[1],
                    label=parts[2],
                    value=parts[3],
                    run_id=parts[4],
                    json_blob=parts[5] if len(parts) > 5 else "{}",
                )
                rows[record.id] = record
        return rows

    def _write_nodes(self) -> None:
        with self.nodes_path.open("w", encoding="utf-8") as handle:
            for node in self._nodes.values():
                handle.write(
                    "\t".join(
                        [
                            node.id,
                            node.kind,
                            node.label,
                            node.run_id,
                            node.json_blob,
                            node.timestamp,
                        ]
                    )
                    + "\n"
                )

    def _write_edges(self) -> None:
        if not self._edges:
            if self.edges_path.exists():
                self.edges_path.unlink()
            return
        with self.edges_path.open("w", encoding="utf-8") as handle:
            for edge in self._edges.values():
                handle.write(
                    "\t".join([edge.parent_id, edge.child_id, edge.run_id, edge.kind]) + "\n"
                )

    def _write_deliverables(self) -> None:
        if not self._deliverables:
            if self.deliverables_path.exists():
                self.deliverables_path.unlink()
            return
        with self.deliverables_path.open("w", encoding="utf-8") as handle:
            for deliverable in self._deliverables.values():
                handle.write(
                    "\t".join(
                        [
                            deliverable.id,
                            deliverable.kind,
                            deliverable.label,
                            deliverable.value,
                            deliverable.run_id,
                            deliverable.json_blob,
                        ]
                    )
                    + "\n"
                )

    def _maybe_record_deliverable(self, event: Dict[str, object], node: NodeRecord) -> None:
        kind = node.kind
        run_id = node.run_id
        label = node.label
        event_id = node.id

        if kind == "deliverable":
            dtype = _as_str(event.get("type") or event.get("deliverable_type") or "note")
            value = _first_non_empty(
                event.get("url"),
                event.get("path"),
                event.get("md"),
                event.get("markdown"),
                event.get("value"),
                event.get("text"),
            ) or ""
            self._register_deliverable(event_id, dtype, label, _as_str(value), run_id, event)
        elif kind == "kpi":
            metric_name = _as_str(event.get("label") or event.get("name") or label or "KPI")
            value = event.get("value") or event.get("score") or event.get("measurement")
            value_str = _as_str(value)
            if event.get("unit"):
                value_str = f"{value_str} {event['unit']}"
            self._register_deliverable(event_id, "kpi", metric_name, value_str, run_id, event)
        elif kind in {"deeplink", "link"}:
            url = _first_non_empty(event.get("url"), event.get("href"), event.get("uri"))
            if url:
                self._register_deliverable(event_id, "link", label, _as_str(url), run_id, event)
        elif kind.startswith("receipt"):
            path = _first_non_empty(event.get("path"), event.get("location"))
            if path:
                self._register_deliverable(event_id, "receipt", label, _as_str(path), run_id, event)
        elif kind in {"file.write", "file_proposal"} and event.get("path"):
            self._register_deliverable(event_id, "file", label, _as_str(event.get("path")), run_id, event)

    def _register_deliverable(
        self,
        deliverable_id: Optional[str],
        kind: str,
        label: str,
        value: str,
        run_id: str,
        event: Dict[str, object],
    ) -> None:
        deliverable_id = _as_str(deliverable_id) or f"deliverable-{uuid.uuid4()}"
        record = DeliverableRecord(
            id=deliverable_id,
            kind=kind,
            label=_as_str(label) or kind.title(),
            value=_as_str(value),
            run_id=_as_str(run_id),
            json_blob=json.dumps(event, sort_keys=True, ensure_ascii=False),
        )
        self._deliverables[record.id] = record
        self._write_deliverables()

    def _next_counter(self) -> int:
        self._counter += 1
        return self._counter


def _render_target(deliverable: DeliverableRecord) -> str:
    value = deliverable.value
    if deliverable.kind == "link" and value:
        return f"[Open]({value})"
    if deliverable.kind in {"file", "receipt"}:
        return f"`{value}`"
    if deliverable.kind == "table":
        return "See below"
    compact = value.replace("\n", " ")
    if len(compact) > 96:
        compact = compact[:93] + "..."
    return compact or "-"


def _render_detail(deliverable: DeliverableRecord) -> List[str]:
    lines = [f"### {deliverable.label} ({deliverable.kind})", ""]
    if deliverable.kind == "link":
        lines.append(f"*Link:* {deliverable.value}")
    elif deliverable.kind in {"file", "receipt"}:
        lines.append(f"*Path:* `{deliverable.value}`")
    elif deliverable.kind == "table" and deliverable.value.strip().startswith("|"):
        lines.append(deliverable.value.strip())
    else:
        lines.append(deliverable.value)
    lines.append("")
    lines.append(f"*Run:* {deliverable.run_id or '-'}")
    return lines


def _as_str(value: Optional[object]) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value
    return str(value)


def _first_non_empty(*values: Optional[object]) -> Optional[str]:
    for value in values:
        text = _as_str(value)
        if text:
            return text
    return None
