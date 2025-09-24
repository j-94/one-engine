#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE_URL="${ENGINE_URL:-http://127.0.0.1:7777}"
CANVAS_PORT="${CANVAS_PORT:-8000}"
ENGINE_SESSION="one_engine"
SERVER_SESSION="one_engine_canvas_server"
LOG_FILE="/tmp/one_engine_canvas.log"

cd "$PROJECT_ROOT"

if ! command -v tmux >/dev/null 2>&1; then
  echo "[error] tmux is required. Install it (e.g., brew install tmux) and rerun." >&2
  exit 1
fi

ensure_engine_running() {
  if tmux has-session -t "$ENGINE_SESSION" 2>/dev/null; then
    return
  fi
  echo "[meta2] starting engine in tmux session '$ENGINE_SESSION'"
  tmux new-session -d -s "$ENGINE_SESSION" "cd '$PROJECT_ROOT' && cargo run > '$LOG_FILE' 2>&1"
}

wait_for_health() {
  for _ in {1..50}; do
    if curl -s "$ENGINE_URL/healthz" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done
  echo "[error] engine did not respond at $ENGINE_URL" >&2
  echo "Check tmux session '$ENGINE_SESSION' (tmux attach -t $ENGINE_SESSION) for logs." >&2
  exit 1
}

ensure_server_running() {
  if lsof -ti :"$CANVAS_PORT" >/dev/null 2>&1; then
    return
  fi
  echo "[meta2] starting canvas server on port $CANVAS_PORT"
  tmux new-session -d -s "$SERVER_SESSION" "cd '$PROJECT_ROOT' && python -m http.server $CANVAS_PORT"
}

ensure_conversation_files() {
  if [ ! -f conversation.md ]; then
    cat <<'EOF' > conversation.md
# Fractal Intelligence Conversations

*Start capturing prompts, engine responses, and receipts here.*
EOF
  fi
  if [ ! -f conversation_canvas.html ]; then
    cat <<'EOF' > conversation_canvas.html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>Fractal Conversation Canvas</title>
  <style>
    body { margin: 0; font-family: "SF Pro", Helvetica, sans-serif; background: #0b0d13; color: #f1f5f9; }
    header { background: linear-gradient(90deg, #111827, #1f2937); padding: 1.2rem 2rem; border-bottom: 1px solid #1f2937; }
    header h1 { margin: 0; font-size: 1.4rem; letter-spacing: 0.08em; text-transform: uppercase; color: #38bdf8; }
    .container { display: flex; gap: 1rem; padding: 1rem 2rem 2rem; height: calc(100vh - 72px); box-sizing: border-box; }
    .column { flex: 1; display: flex; flex-direction: column; background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(148, 163, 184, 0.15); border-radius: 12px; backdrop-filter: blur(18px); overflow: hidden; }
    .column h2 { margin: 0; padding: 0.85rem 1.1rem; font-size: 0.85rem; letter-spacing: 0.1em; text-transform: uppercase; border-bottom: 1px solid rgba(148, 163, 184, 0.12); background: rgba(15, 23, 42, 0.95); }
    .column .content { flex: 1; margin: 0; padding: 1rem; overflow-y: auto; line-height: 1.5; }
    pre { font-family: 'JetBrains Mono', 'Fira Code', monospace; font-size: 0.85rem; background: rgba(15, 23, 42, 0.35); border-radius: 8px; padding: 0.75rem; border: 1px solid rgba(148, 163, 184, 0.1); white-space: pre-wrap; word-wrap: break-word; }
  </style>
</head>
<body>
  <header>
    <h1>Fractal Intelligence — Conversation Canvas</h1>
  </header>
  <div class="container">
    <div class="column" id="prompts">
      <h2>Prompts</h2>
      <div class="content"></div>
    </div>
    <div class="column" id="responses">
      <h2>Engine Effects</h2>
      <div class="content"></div>
    </div>
    <div class="column" id="receipts">
      <h2>Receipts</h2>
      <div class="content"></div>
    </div>
  </div>
  <script>
    async function loadConversation() {
      const response = await fetch('conversation.md');
      const text = await response.text();
      const sections = text.split(/(^###\s.*$)/m).slice(1);
      const prompts = document.querySelector('#prompts .content');
      const responses = document.querySelector('#responses .content');
      const receipts = document.querySelector('#receipts .content');
      prompts.innerHTML = '';
      responses.innerHTML = '';
      receipts.innerHTML = '';
      for (let i = 0; i < sections.length; i += 2) {
        const title = sections[i];
        const body = sections[i + 1];
        if (!body) continue;
        const prompt = body.match(/#### Prompt\n```([\s\S]*?)```/);
        const response = body.match(/#### Engine Response\n```([\s\S]*?)```/);
        const receipt = body.match(/#### Receipts\n```([\s\S]*?)```/);
        prompts.innerHTML += `<h3>${title.trim()}</h3><pre>${prompt ? prompt[1].trim() : ''}</pre>`;
        responses.innerHTML += `<h3>${title.trim()}</h3><pre>${response ? response[1].trim() : ''}</pre>`;
        receipts.innerHTML += `<h3>${title.trim()}</h3><pre>${receipt ? receipt[1].trim() : ''}</pre>`;
      }
    }
    loadConversation();
  </script>
</body>
</html>
EOF
  fi
}

ensure_engine_running
wait_for_health
ensure_server_running
ensure_conversation_files

CANVAS_URL="http://127.0.0.1:${CANVAS_PORT}/conversation_canvas.html"

if command -v open >/dev/null 2>&1; then
  echo "[meta2] opening $CANVAS_URL"
  open "$CANVAS_URL"
else
  echo "[meta2] visit $CANVAS_URL"
fi

printf "\nSessions:\n  Engine:   tmux attach -t %s\n  WebSrv:   tmux attach -t %s\nLogs:\n  Engine log: %s\n\n" "$ENGINE_SESSION" "$SERVER_SESSION" "$LOG_FILE"
