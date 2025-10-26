"""Minimal async SWE-style chat client for the terminal."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys
import urllib.error
import urllib.request
from typing import Dict, Optional

from . import sse

DEFAULT_ENGINE_URL = "http://127.0.0.1:8080"
SESSION_FILE = "swechat_session.json"


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Stream chat tokens from the engine")
    parser.add_argument("message", nargs="*", help="Prompt to send to the engine")
    parser.add_argument("--engine-url", help="Override ENGINE_URL environment variable")
    parser.add_argument("--api-key", default=os.environ.get("ENGINE_API_KEY"))
    parser.add_argument("--timeout", type=int, default=45)
    parser.add_argument("--no-stream", action="store_true", help="Disable SSE mode and use the fallback conversation flow")
    parser.add_argument(
        "--root",
        default=".",
        help="Project root used to persist session metadata (default: current directory)",
    )
    args = parser.parse_args(argv)

    message = " ".join(args.message).strip()
    if not message:
        message = sys.stdin.read().strip()
    if not message:
        print("No message provided.", file=sys.stderr)
        return 1

    engine_url = args.engine_url or os.environ.get("ENGINE_URL", DEFAULT_ENGINE_URL)
    if not args.no_stream and try_stream(engine_url, message, args.api_key, args.timeout):
        return 0

    return run_conversation(engine_url, message, args.api_key, Path(args.root).resolve(), args.timeout)


def try_stream(engine_url: str, prompt: str, api_key: Optional[str], timeout: int) -> bool:
    url = f"{engine_url.rstrip('/')}/chat"
    payload = json.dumps({"messages": [{"role": "user", "content": prompt}]})
    headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}
    if api_key:
        headers["X-API-Key"] = api_key

    request = urllib.request.Request(url, data=payload.encode("utf-8"), headers=headers, method="POST")
    try:
        response = urllib.request.urlopen(request, timeout=timeout)
    except urllib.error.HTTPError as exc:
        if exc.code in {404, 405}:
            return False
        print(f"/chat error {exc.code}: {exc.reason}", file=sys.stderr)
        return True
    except urllib.error.URLError as exc:  # pragma: no cover - network failure
        print(f"Failed to reach /chat: {exc}", file=sys.stderr)
        return True

    try:
        for event in sse.iter_sse_lines(response):
            data = event.get("data")
            if data == "[DONE]":
                break
            if not data:
                continue
            chunk = _extract_text_chunk(data)
            if chunk:
                print(chunk, end="", flush=True)
    finally:
        response.close()
    print()
    return True


def _extract_text_chunk(payload: str) -> str:
    try:
        body = json.loads(payload)
    except json.JSONDecodeError:
        return payload
    if isinstance(body, dict):
        delta = body.get("delta")
        if isinstance(delta, dict):
            return delta.get("content", "")
        message = body.get("message")
        if isinstance(message, dict):
            return message.get("content", "")
        return body.get("content") or body.get("text") or ""
    return str(body)


def run_conversation(
    engine_url: str,
    prompt: str,
    api_key: Optional[str],
    root: Path,
    timeout: int,
) -> int:
    session_path = root / ".graphlogue" / SESSION_FILE
    session_path.parent.mkdir(parents=True, exist_ok=True)
    session = _load_session(session_path)
    branches = session.setdefault("branches", {})
    branch_id = branches.get(engine_url)
    if not branch_id:
        branch_id = _create_branch(engine_url, api_key, timeout)
        if not branch_id:
            print("Failed to initialise conversation branch.", file=sys.stderr)
            return 1
        branches[engine_url] = branch_id
        _save_session(session_path, session)

    response = _send_prompt(engine_url, branch_id, prompt, api_key, timeout)
    if response is None:
        print("Conversation API returned an error; falling back to /execute_goal.", file=sys.stderr)
        return run_execute_goal(engine_url, prompt, api_key, timeout)

    print(format_conversation_response(response))
    return 0


def _create_branch(engine_url: str, api_key: Optional[str], timeout: int) -> Optional[str]:
    payload = json.dumps({"label": "swechat"}).encode("utf-8")
    url = f"{engine_url.rstrip('/')}/conversation"
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["X-API-Key"] = api_key
    request = urllib.request.Request(url, data=payload, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            data = json.loads(response.read().decode("utf-8"))
            branch_id = data.get("branch_id")
            if isinstance(branch_id, str) and branch_id:
                return branch_id
    except urllib.error.URLError as exc:  # pragma: no cover - network failure
        print(f"Failed to create conversation branch: {exc}", file=sys.stderr)
    except json.JSONDecodeError:
        print("Invalid JSON returned from /conversation", file=sys.stderr)
    return None


def _send_prompt(
    engine_url: str,
    branch_id: str,
    prompt: str,
    api_key: Optional[str],
    timeout: int,
) -> Optional[Dict[str, object]]:
    payload = json.dumps({"prompt": prompt}).encode("utf-8")
    url = f"{engine_url.rstrip('/')}/conversation/{branch_id}/prompt"
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["X-API-Key"] = api_key
    request = urllib.request.Request(url, data=payload, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode("utf-8")
            return json.loads(body)
    except urllib.error.HTTPError as exc:
        print(f"Prompt request failed: {exc}", file=sys.stderr)
    except urllib.error.URLError as exc:  # pragma: no cover - network failure
        print(f"Prompt request failed: {exc}", file=sys.stderr)
    except json.JSONDecodeError:
        print("Prompt response was not valid JSON", file=sys.stderr)
    return None


def run_execute_goal(engine_url: str, prompt: str, api_key: Optional[str], timeout: int) -> int:
    payload = json.dumps({"goal": prompt}).encode("utf-8")
    url = f"{engine_url.rstrip('/')}/execute_goal"
    headers = {"Content-Type": "application/json"}
    if api_key:
        headers["X-API-Key"] = api_key
    request = urllib.request.Request(url, data=payload, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode("utf-8")
            print(body)
            return 0
    except urllib.error.URLError as exc:
        print(f"Failed to call /execute_goal: {exc}", file=sys.stderr)
        return 1


def format_conversation_response(response: Dict[str, object]) -> str:
    effect = response.get("effect")
    if isinstance(effect, dict) and effect:
        key, value = next(iter(effect.items()))
        pretty = json.dumps({key: value}, indent=2)
        return f"Effect {key}:\n{pretty}"
    if response.get("events"):
        return json.dumps(response["events"], indent=2)
    return json.dumps(response, indent=2)


def _load_session(path: Path) -> Dict[str, object]:
    if path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            pass
    return {}


def _save_session(path: Path, session: Dict[str, object]) -> None:
    path.write_text(json.dumps(session, indent=2), encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
