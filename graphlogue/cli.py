"""Command line entrypoint for the Graphlogue terminal viewer."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import textwrap
from typing import Optional

from . import demo
from .store import DeliverableRecord, GraphlogueStore
from . import sse

DEFAULT_ENGINE_URL = "http://127.0.0.1:8080"


def main(argv: Optional[list[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    store = GraphlogueStore(Path(args.root).resolve())

    if args.command == "demo":
        return run_demo(store)
    if args.command == "run":
        return run_stream(store, args)
    if args.command == "deliver":
        return show_deliverables(store)
    if args.command == "export":
        return export_deliverables(store, Path(args.output))

    parser.print_help()
    return 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Graphlogue terminal toolkit")
    parser.add_argument(
        "--root",
        default=".",
        help="Project root containing the .graphlogue directory (default: current directory)",
    )

    sub = parser.add_subparsers(dest="command")

    sub.add_parser("demo", help="Load demo events into the deliverables store")

    run_parser = sub.add_parser("run", help="Follow a live progress.sse stream")
    run_parser.add_argument("run_id", help="Run identifier to follow")
    run_parser.add_argument(
        "--engine-url",
        help="Override ENGINE_URL",
    )
    run_parser.add_argument(
        "--api-key",
        default=os.environ.get("ENGINE_API_KEY"),
        help="API key to attach as X-API-Key header",
    )
    run_parser.add_argument(
        "--timeout",
        type=int,
        default=30,
        help="HTTP timeout in seconds for SSE connection (default: 30)",
    )

    sub.add_parser("deliver", help="Open the deliverables feed")

    export_parser = sub.add_parser("export", help="Write a DELIVERABLES.md summary")
    export_parser.add_argument(
        "--output",
        default="DELIVERABLES.md",
        help="Destination markdown file (default: DELIVERABLES.md)",
    )

    return parser


def run_demo(store: GraphlogueStore) -> int:
    store.reset()
    for event in demo.demo_events():
        store.register_event(event)
    print("Loaded demo run into .graphlogue")
    print("Try `graphlogue deliver` or `graphlogue export`.\n")
    return 0


def run_stream(store: GraphlogueStore, args: argparse.Namespace) -> int:
    engine_url = args.engine_url or os.environ.get("ENGINE_URL", DEFAULT_ENGINE_URL)
    url = f"{engine_url.rstrip('/')}/runs/{args.run_id}/progress.sse"
    headers = {"Accept": "text/event-stream"}
    if args.api_key:
        headers["X-API-Key"] = args.api_key

    print(f"[graphlogue] connecting to {url}")
    try:
        response = sse.fetch(url, headers=headers, timeout=args.timeout)
    except sse.SseError as exc:
        print(f"[graphlogue] failed to open SSE stream: {exc}", file=sys.stderr)
        return 1

    try:
        for event in sse.iter_json_events(response):
            node = store.register_event(event)
            print(render_event(node))
            sys.stdout.flush()
    except KeyboardInterrupt:
        print("\n[graphlogue] interrupted", file=sys.stderr)
    finally:
        response.close()
    return 0


def render_event(node) -> str:
    label = textwrap.shorten(node.label, width=80, placeholder="...")
    return f"[{node.run_id}] {node.kind}: {label}"


def show_deliverables(store: GraphlogueStore) -> int:
    deliverables = store.list_deliverables()
    if not deliverables:
        print("No deliverables recorded yet.")
        return 0

    fzf = shutil.which("fzf")
    if fzf and sys.stdin.isatty() and sys.stdout.isatty():
        selection = run_fzf(fzf, deliverables)
        if selection is None:
            return 0
        open_deliverable(selection)
    else:
        print("Deliverables:")
        for record in deliverables:
            summary = textwrap.shorten(record.value.replace("\n", " "), width=90)
            print(f"- [{record.kind}] {record.label} -> {summary}")
    return 0


def run_fzf(fzf_path: str, deliverables: list[DeliverableRecord]) -> Optional[DeliverableRecord]:
    lines = []
    lookup: dict[str, DeliverableRecord] = {}
    for record in deliverables:
        summary = textwrap.shorten(record.value.replace("\n", " "), width=72)
        lines.append(f"{record.id}\t[{record.kind}] {record.label}\t{summary}")
        lookup[record.id] = record

    proc = subprocess.run(
        [
            fzf_path,
            "--with-nth=2..",
            "--prompt",
            "deliverables> ",
            "--header",
            "enter to open; esc to quit",
        ],
        input="\n".join(lines),
        text=True,
        capture_output=True,
    )
    if proc.returncode != 0 or not proc.stdout.strip():
        return None
    selected = proc.stdout.strip().split("\t", 1)[0]
    return lookup.get(selected)


def open_deliverable(record: DeliverableRecord) -> None:
    print(f"Opening {record.kind} {record.label}")
    if record.kind == "link":
        opener = shutil.which("xdg-open") or shutil.which("open")
        if opener:
            subprocess.Popen([opener, record.value])
            return
        print(record.value)
    elif record.kind in {"file", "receipt"}:
        path = Path(record.value)
        if path.exists():
            pager = os.environ.get("PAGER") or shutil.which("less")
            if pager:
                subprocess.run([pager, str(path)])
            else:
                print(path.read_text(encoding="utf-8"))
        else:
            print(f"File not found: {path}", file=sys.stderr)
    else:
        print(record.value)


def export_deliverables(store: GraphlogueStore, destination: Path) -> int:
    store.export_markdown(destination)
    print(f"Wrote deliverable summary to {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
