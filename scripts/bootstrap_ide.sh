#!/usr/bin/env bash
set -euo pipefail

# Bootstrap the lightweight "Consciousness IDE" static UI and serve it.
# Usage:
#   bash ./scripts/bootstrap_ide.sh
#
# It will:
# - Ensure one-engine-ide/ exists with index.html
# - Start a static server on port 8010 to serve the IDE
# - Print the IDE URL
#
# ENV:
#   ENGINE_BASE_URL (default http://127.0.0.1:7777)
#   IDE_PORT (default 8010)

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IDE_DIR="$PROJECT_ROOT/one-engine-ide"
ENGINE_BASE_URL="${ENGINE_BASE_URL:-http://127.0.0.1:7777}"
IDE_PORT="${IDE_PORT:-8010}"

mkdir -p "$IDE_DIR"

# If index.html is missing, create a tiny placeholder and instruct the user to rebuild if needed.
if [[ ! -f "$IDE_DIR/index.html" ]]; then
  cat >"$IDE_DIR/index.html" <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>One Engine – Consciousness IDE</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <style>
    :root { --bg:#0b0d13; --panel:#0f172a; --muted:#94a3b8; --fg:#e5e7eb; --cyan:#22d3ee; --emerald:#10b981; --rose:#f43f5e; }
    body { margin:0; background:var(--bg); color:var(--fg); font:14px/1.45 -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Inter, sans-serif; }
    header { padding:12px 16px; border-bottom:1px solid #1f2937; background:#111827; display:flex; gap:12px; align-items:center; }
    header h1 { margin:0; font-size:14px; letter-spacing:.1em; color:var(--cyan); text-transform:uppercase; }
    main { display:grid; grid-template-columns: 2fr 1fr; gap:12px; padding:12px; }
    .panel { background:var(--panel); border:1px solid rgba(148,163,184,.15); border-radius:10px; overflow:hidden; }
    .panel header { background:rgba(2,6,23,.6); border-bottom:1px solid rgba(148,163,184,.12); }
    .panel header h2 { margin:0; font-size:12px; letter-spacing:.12em; text-transform:uppercase; color:#93c5fd; }
    .content { padding:12px; }
    .row { display:flex; gap:8px; align-items:center; flex-wrap:wrap; }
    input[type=text] { background:#0b1220; color:var(--fg); border:1px solid #1f2a3a; border-radius:8px; padding:8px 10px; outline:none; min-width: 200px; }
    textarea { width:100%; min-height:72px; background:#0b1220; color:var(--fg); border:1px solid #1f2a3a; border-radius:8px; padding:8px 10px; outline:none; resize:vertical; }
    button { background:#162033; color:var(--fg); border:1px solid #2a3b55; border-radius:8px; padding:7px 10px; cursor:pointer; }
    button.primary { background:#1f2f46; border-color:#35527a; }
    button.ghost { background:transparent; border-color:#2a3b55; }
    .muted { color:var(--muted); }
    .log { height: 260px; overflow:auto; background:#0b1220; border:1px solid #1f2a3a; border-radius:8px; padding:8px; }
    .log .line { margin:0 0 6px; white-space:pre-wrap; word-break:break-word; }
    .pill { padding:2px 8px; border:1px solid #2a3b55; border-radius:999px; font-size:12px; }
    .ok { color:var(--emerald); border-color:#14532d; }
    .warn { color:#f59e0b; border-color:#713f12; }
    .err { color:var(--rose); border-color:#7f1d1d; }
    .list { display:flex; flex-direction:column; gap:8px; }
    .item { border:1px solid #1f2a3a; padding:8px; border-radius:8px; background:#0b1220; }
    .item h4 { margin:0 0 4px; font-size:13px; }
    .grid { display:grid; grid-template-columns: 1fr 1fr; gap:8px; }
  </style>
</head>
<body>
  <header>
    <h1>Consciousness IDE</h1>
    <span class="muted">Lightweight UI that talks directly to the engine</span>
  </header>
  <main>
    <section class="panel" style="grid-column: span 2;">
      <header><h2>Connection</h2></header>
      <div class="content row">
        <label>Engine URL</label>
        <input id="engineUrl" type="text" placeholder="http://127.0.0.1:7777" />
        <button id="btnConnect" class="primary">Connect / Ensure Branch</button>
        <span id="status" class="muted"></span>
      </div>
    </section>

    <section class="panel">
      <header><h2>Chat</h2></header>
      <div class="content">
        <div class="row muted" style="margin-bottom:8px;">
          <span>Branch:</span> <span id="branchId" class="pill"></span>
        </div>
        <textarea id="prompt" placeholder="Teach a workflow or request an action...\nExample: Define a tool named 'github_issue_summarizer' ..."></textarea>
        <div class="row" style="margin-top:8px;">
          <button id="btnSend" class="primary">Send</button>
          <button id="btnApprove" class="ghost">Approve Last Pattern</button>
          <button id="btnReload" class="ghost">Reload Events</button>
        </div>
        <div class="log" id="log"></div>
      </div>
    </section>

    <section class="panel">
      <header><h2>Pattern Library</h2></header>
      <div class="content">
        <div class="row" style="margin-bottom:8px;">
          <button id="btnRefreshAutodoc" class="ghost">Refresh</button>
          <span class="muted">Persisted endpoints appear here once approved</span>
        </div>
        <div class="list" id="patterns"></div>
      </div>
    </section>

    <section class="panel" style="grid-column: span 2;">
      <header><h2>High-level Feedback</h2></header>
      <div class="content grid">
        <div>
          <div class="row" style="margin-bottom:8px;">
            <span class="pill">Generated: <span id="genCt">0</span></span>
            <span class="pill">Approvals: <span id="apprCt">0</span></span>
            <span class="pill">Calls: <span id="callCt">0</span></span>
          </div>
          <div class="muted" id="bits">A=?, U=?, P=?, E=?, Δ=?, I=?, R=?, T=?</div>
        </div>
        <div>
          <div class="muted">Timeline (ordinal K/N)</div>
          <div class="row">
            <input id="kInput" type="text" placeholder="K" style="width:60px;" />
            <span id="nLabel" class="muted"></span>
            <button id="btnApplyK" class="ghost">Apply</button>
          </div>
        </div>
      </div>
    </section>
  </main>

  <script>
    const $ = (id) => document.getElementById(id);
    const st = (el, msg) => el.textContent = msg;

    let BASE = localStorage.getItem('ENGINE_BASE_URL') || 'http://127.0.0.1:7777';
    let BRANCH = localStorage.getItem('ENGINE_BRANCH_ID') || '';
    let LAST_PATTERN = '';
    let K = 0, N = 0; // ordinal window

    $('engineUrl').value = BASE;
    st($('branchId'), BRANCH || '');

    async function json(url, opts={}) {
      const r = await fetch(url, opts);
      if (!r.ok) throw new Error('HTTP ' + r.status);
      return r.json();
    }

    function logLine(text, cls=''){
      const div = document.createElement('div');
      div.className = 'line ' + cls;
      div.textContent = text;
      $('log').appendChild(div);
      $('log').scrollTop = $('log').scrollHeight;
    }

    async function ensureBranch(){
      BASE = $('engineUrl').value.trim() || BASE;
      localStorage.setItem('ENGINE_BASE_URL', BASE);
      // Try to reuse existing branch
      if (!BRANCH) {
        const data = await json(`${BASE}/conversation`, {
          method:'POST', headers:{'content-type':'application/json'}, body: JSON.stringify({label:'ide'})
        });
        BRANCH = data.branch_id || '';
        localStorage.setItem('ENGINE_BRANCH_ID', BRANCH);
        st($('branchId'), BRANCH);
        logLine(`[Ready] Branch created: ${BRANCH}`);
      } else {
        logLine(`[Ready] Using existing branch: ${BRANCH}`);
      }
      await reloadAll();
    }

    async function sendPrompt(p){
      if (!BRANCH) await ensureBranch();
      const payload = { prompt: p };
      const res = await json(`${BASE}/conversation/${BRANCH}/prompt`, {
        method:'POST', headers:{'content-type':'application/json'}, body: JSON.stringify(payload)
      });
      logLine(`> ${p}`);
      const eff = JSON.stringify(res.effect || res, null, 0);
      logLine(eff, 'muted');
      await reloadEvents();
    }

    function summarizeBits(evts){
      // Minimal derivation: counts and heuristic gates
      let gen=0, appr=0, call=0;
      for(const e of evts){
        if (e.ApiGenerated) gen++;
        if (e.ApiCalled) call++;
        const d = e.ParsedIntent?.description || '';
        if (d.includes('ApprovePattern') || d.includes('approval:')) appr++;
      }
      return { gen, appr, call };
    }

    async function reloadEvents(){
      const snap = await json(`${BASE}/conversation/${BRANCH}/events`);
      const evts = Array.isArray(snap.events) ? snap.events : [];
      N = evts.length; if (!K || K> N) K=N; // default K=N
      const windowed = evts.slice(0, K);
      const {gen, appr, call} = summarizeBits(windowed);
      st($('genCt'), gen); st($('apprCt'), appr); st($('callCt'), call); st($('nLabel'), `/ N=${N}`);
      // naive bits text
      st($('bits'), `A=?, U=?, P=${appr>0?1:0}, E=?, Δ=?, I=?, R=?, T=?  (window K=${K})`);
    }

    async function reloadAutodoc(){
      const doc = await json(`${BASE}/autodoc/${BRANCH}`);
      const list = $('patterns');
      list.innerHTML = '';
      const eps = Array.isArray(doc.endpoints)? doc.endpoints: [];
      for (const ep of eps){
        const item = document.createElement('div');
        item.className = 'item';
        const h = document.createElement('h4'); h.textContent = ep.name + (ep.persisted? ' (persisted)':'');
        item.appendChild(h);
        const d = document.createElement('div'); d.className='muted'; d.textContent = ep.description || '';
        item.appendChild(d);
        const row = document.createElement('div'); row.className = 'row'; row.style.marginTop='6px';
        const btnApprove = document.createElement('button'); btnApprove.textContent='Approve';
        btnApprove.onclick = async ()=>{ LAST_PATTERN = ep.name; await sendPrompt(`Approve pattern '${ep.name}'`); };
        const btnCall = document.createElement('button'); btnCall.textContent='Call';
        btnCall.onclick = async ()=>{
          if ((ep.parameters||[]).length===0){ await sendPrompt(`Call the API '${ep.name}'`); }
          else {
            const first = ep.parameters[0] || 'text';
            await sendPrompt(`Call the API '${ep.name}' with ${first}='demo'`);
          }
        };
        row.appendChild(btnApprove); row.appendChild(btnCall);
        // Crystallize = same as Approve for MVS
        const btnC = document.createElement('button'); btnC.textContent='Crystallize';
        btnC.onclick = async ()=>{ LAST_PATTERN = ep.name; await sendPrompt(`Approve pattern '${ep.name}'`); };
        row.appendChild(btnC);
        item.appendChild(row);
        list.appendChild(item);
      }
    }

    async function reloadAll(){ await reloadEvents(); await reloadAutodoc(); }

    $('btnConnect').onclick = ensureBranch;
    $('btnSend').onclick = async ()=>{
      const p = $('prompt').value.trim(); if (!p) return;
      await sendPrompt(p); $('prompt').value='';
    };
    $('btnApprove').onclick = async ()=>{
      if (!LAST_PATTERN) { logLine('[info] Nothing to approve yet', 'muted'); return; }
      await sendPrompt(`Approve pattern '${LAST_PATTERN}'`);
    };
    $('btnReload').onclick = reloadAll;
    $('btnRefreshAutodoc').onclick = reloadAutodoc;
    $('btnApplyK').onclick = async ()=>{
      const v = parseInt(($('kInput').value||'').trim(), 10);
      if (!isNaN(v)) { K=Math.max(0, Math.min(v, N)); await reloadEvents(); }
    };

    // Auto-wire defaults
    (async ()=>{
      try { await ensureBranch(); } catch(e){ st($('status'), 'Connect failed – set Engine URL and retry'); }
    })();
  </script>
</body>
</html>
HTML
fi

# Start a simple static server for the IDE
if lsof -ti ":${IDE_PORT}" >/dev/null 2>&1; then
  echo "[meta2] IDE already served on :${IDE_PORT}"
else
  echo "[meta2] serving IDE on :${IDE_PORT}"
  ( cd "$IDE_DIR" && nohup python3 -m http.server ${IDE_PORT} > /tmp/one_engine_ide_http.log 2>&1 & )
fi

URL="http://127.0.0.1:${IDE_PORT}/index.html"
echo "[meta2] open ${URL}"
if command -v open >/dev/null 2>&1; then open "$URL"; fi
