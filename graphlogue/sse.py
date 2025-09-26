"""Tiny Server-Sent Event helpers used by the CLI tools."""
from __future__ import annotations

import json
from typing import Dict, Iterable, Iterator, Optional
import urllib.error
import urllib.request


class SseError(RuntimeError):
    """Raised when an SSE stream cannot be consumed."""


def fetch(url: str, headers: Optional[Dict[str, str]] = None, timeout: int = 30):
    """Open an SSE stream with urllib.

    The caller is responsible for closing the returned response object.
    """

    request = urllib.request.Request(url, headers=headers or {})
    try:
        return urllib.request.urlopen(request, timeout=timeout)
    except urllib.error.URLError as exc:  # pragma: no cover - network failure
        raise SseError(str(exc)) from exc


def iter_sse_lines(source: Iterable[bytes]) -> Iterator[Dict[str, str]]:
    """Convert an iterable of raw HTTP lines into SSE dictionaries."""

    data: list[str] = []
    event_type = ""
    event_id = ""

    for raw_line in source:
        line = raw_line.decode("utf-8", errors="replace").rstrip("\r\n")
        if not line:
            if not data:
                continue
            payload = "\n".join(data)
            yield {
                "event": event_type,
                "id": event_id,
                "data": payload,
            }
            data = []
            event_type = ""
            event_id = ""
            continue
        if line.startswith(":"):
            continue
        if line.startswith("data:"):
            data.append(line[5:].lstrip())
        elif line.startswith("event:"):
            event_type = line[6:].lstrip()
        elif line.startswith("id:"):
            event_id = line[3:].lstrip()
        else:
            # Treat the full line as data when the prefix is unknown.
            data.append(line)

    if data:
        payload = "\n".join(data)
        yield {
            "event": event_type,
            "id": event_id,
            "data": payload,
        }


def iter_json_events(source: Iterable[bytes]) -> Iterator[Dict[str, object]]:
    """Parse JSON events from an SSE byte iterable."""

    for envelope in iter_sse_lines(source):
        raw = envelope.get("data", "")
        if not raw:
            continue
        try:
            event = json.loads(raw)
        except json.JSONDecodeError:
            event = {"kind": envelope.get("event") or "text", "data": raw}
        else:
            if isinstance(event, dict):
                event.setdefault("kind", envelope.get("event") or event.get("kind") or "event")
                if envelope.get("id") and "id" not in event:
                    event["id"] = envelope["id"]
        yield event
