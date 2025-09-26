#!/usr/bin/env bash
set -euo pipefail

SCRIPT_PATH=$(readlink -f "$0")
GRAPH_DIR="${GRAPH_DIR:-.graphlogue}"
NODES_FILE="$GRAPH_DIR/nodes.tsv"
EDGES_FILE="$GRAPH_DIR/edges.tsv"
EVENTS_FILE="$GRAPH_DIR/events.jsonl"
STREAM_LOG="$GRAPH_DIR/stream.log"

usage() {
  cat <<'USAGE'
Usage: graphlogue.sh [options]
  -r, --run-id ID        Filter SSE stream to a specific run_id
  -u, --engine-url URL   Override ENGINE_URL (default env or http://127.0.0.1:8080)
      --no-stream        Skip the SSE consumer; just launch the viewer
  -h, --help             Show this message

The script maintains .graphlogue/{nodes,edges}.tsv and launches an fzf viewer.
USAGE
}

ensure_prereqs() {
  local missing=()
  for bin in curl jq fzf awk sha1sum sed; do
    if ! command -v "$bin" >/dev/null 2>&1; then
      missing+=("$bin")
    fi
  done
  if ((${#missing[@]} > 0)); then
    printf 'graphlogue: missing required tools: %s\n' "${missing[*]}" >&2
    exit 1
  fi
}

sanitize() {
  local input="${1:-}"
  input=${input//$'\r'/ }
  input=${input//$'\n'/ }
  input=${input//$'\t'/ }
  printf '%s' "$input"
}

truncate_text() {
  local text="${1:-}" max=${2:-48}
  local len=${#text}
  if (( len > max )); then
    printf '%s' "${text:0:max-3}..."
  else
    printf '%s' "$text"
  fi
}

qs() {
  local filter="$1" json="$2" out
  if ! out=$(jq -r "$filter // empty" <<<"$json" 2>/dev/null); then
    out=""
  fi
  printf '%s' "$out"
}

timestamp_now() {
  date +%s%3N
}

init_store() {
  mkdir -p "$GRAPH_DIR"
  touch "$NODES_FILE" "$EDGES_FILE" "$EVENTS_FILE"
}

upsert_node() {
  local id="$1" kind="$(sanitize "$2")" label="$(sanitize "$3")" ts="${4:-$(timestamp_now)}"
  local tmp="$NODES_FILE.tmp"
  awk -F'\t' -v id="$id" -v kind="$kind" -v label="$label" -v ts="$ts" 'BEGIN{updated=0}
    $1==id {print id"\t"kind"\t"label"\t"ts; updated=1; next}
    {print}
    END{if(!updated) print id"\t"kind"\t"label"\t"ts}' "$NODES_FILE" > "$tmp"
  mv "$tmp" "$NODES_FILE"
}

add_edge() {
  local src="$1" dst="$2" type="$(sanitize "${3:-link}")" ts="${4:-$(timestamp_now)}"
  local tmp="$EDGES_FILE.tmp"
  awk -F'\t' -v src="$src" -v dst="$dst" -v type="$type" -v ts="$ts" 'BEGIN{added=0}
    $1==src && $2==dst && $3==type {if(added==0){print src"\t"dst"\t"type"\t"ts; added=1}; next}
    {print}
    END{if(!added) print src"\t"dst"\t"type"\t"ts}' "$EDGES_FILE" > "$tmp"
  mv "$tmp" "$EDGES_FILE"
}

append_event() {
  printf '%s\n' "$1" >> "$EVENTS_FILE"
}

process_event() {
  local payload="$1" kind rid ts
  kind="$(qs '.kind' "$payload")"
  [[ -z "$kind" ]] && return
  ts="$(timestamp_now)"
  rid="$(qs '.run_id' "$payload")"
  append_event "$payload"
  case "$kind" in
    run.start)
      local intent
      intent="$(qs '.intent' "$payload")"
      upsert_node "run/$rid" "Run" "${rid:-unknown}" "$ts"
      if [[ -n "$intent" ]]; then
        local iid="intent/$(printf '%s' "$intent" | sha1sum | awk '{print $1}')"
        upsert_node "$iid" "Intent" "$(truncate_text "$intent" 80)" "$ts"
        [[ -n "$rid" ]] && add_edge "run/$rid" "$iid" "intent" "$ts"
      fi
      ;;
    plan.step)
      local step purpose sid
      step="$(qs '.n' "$payload")"
      purpose="$(qs '.purpose' "$payload")"
      sid="step/$rid/$step"
      upsert_node "$sid" "PlanStep" "$(truncate_text "$purpose" 96)" "$ts"
      [[ -n "$rid" ]] && add_edge "run/$rid" "$sid" "step" "$ts"
      ;;
    net.read)
      local sha url nid
      sha="$(qs '.sha256' "$payload")"
      url="$(qs '.url' "$payload")"
      nid="read/${sha:-$(timestamp_now)}"
      upsert_node "$nid" "NetRead" "$(truncate_text "$url" 96)" "$ts"
      [[ -n "$rid" ]] && add_edge "$nid" "run/$rid" "feeds" "$ts"
      ;;
    bundle.proposed)
      local count idx=0
      count="$(qs '.files | length' "$payload")"
      if [[ -n "$count" && "$count" != 0 ]]; then
        while (( idx < count )); do
          local path sha node_id
          path="$(qs ".files[$idx].path" "$payload")"
          sha="$(qs ".files[$idx].sha256" "$payload")"
          node_id="filep/${sha:-${path//\//_}}"
          upsert_node "$node_id" "FileProposal" "$(truncate_text "$path" 96)" "$ts"
          [[ -n "$rid" ]] && add_edge "run/$rid" "$node_id" "proposed" "$ts"
          ((idx++))
        done
      fi
      ;;
    gate.eval)
      local step decision node_id
      step="$(qs '.step' "$payload")"
      decision="$(qs '.decision' "$payload")"
      node_id="gate/$rid/$step"
      upsert_node "$node_id" "GateDecision" "$(truncate_text "$decision" 72)" "$ts"
      [[ -n "$rid" ]] && add_edge "$node_id" "run/$rid" "decision" "$ts"
      ;;
    file.write)
      local path sha node_id
      path="$(qs '.path' "$payload")"
      sha="$(qs '.sha256' "$payload")"
      node_id="filew/${sha:-$(timestamp_now)}"
      upsert_node "$node_id" "FileWrite" "$(truncate_text "$path" 96)" "$ts"
      [[ -n "$rid" ]] && add_edge "run/$rid" "$node_id" "writes" "$ts"
      ;;
    deeplink)
      local url lid
      url="$(qs '.url' "$payload")"
      lid="link/$(printf '%s' "$url" | sha1sum | awk '{print $1}')"
      upsert_node "$lid" "Deeplink" "$(truncate_text "$url" 96)" "$ts"
      [[ -n "$rid" ]] && add_edge "run/$rid" "$lid" "deeplink" "$ts"
      ;;
    kpi)
      local agreement cost node_id
      agreement="$(qs '.decision_agreement' "$payload")"
      cost="$(qs '.cost_per_decision' "$payload")"
      node_id="kpi/$rid/$ts"
      upsert_node "$node_id" "KPI" "agreement=$agreement cost=$cost" "$ts"
      [[ -n "$rid" ]] && add_edge "run/$rid" "$node_id" "kpi" "$ts"
      ;;
    note)
      local message nid
      message="$(qs '.message' "$payload")"
      [[ -z "$message" ]] && message="$(qs '.text' "$payload")"
      nid="note/$rid/$ts"
      upsert_node "$nid" "Note" "$(truncate_text "$message" 96)" "$ts"
      [[ -n "$rid" ]] && add_edge "run/$rid" "$nid" "note" "$ts"
      ;;
    run.halt)
      local code hid
      code="$(qs '.code' "$payload")"
      hid="halt/$rid/$ts"
      upsert_node "$hid" "RunHalt" "${code:-halt}" "$ts"
      [[ -n "$rid" ]] && add_edge "run/$rid" "$hid" "halt" "$ts"
      ;;
    receipt.write)
      local path rid_local nid
      path="$(qs '.path' "$payload")"
      rid_local="${rid:-$(qs '.run_id' "$payload")}" 
      nid="receipt/${rid_local:-unknown}/$ts"
      upsert_node "$nid" "Receipt" "$(truncate_text "$path" 96)" "$ts"
      [[ -n "$rid_local" ]] && add_edge "run/$rid_local" "$nid" "receipt" "$ts"
      ;;
    *)
      local summary generic
      summary="$(qs '.summary' "$payload")"
      [[ -z "$summary" ]] && summary="$(qs '.message' "$payload")"
      generic="evt/$kind/$ts"
      if [[ -n "$summary" ]]; then
        upsert_node "$generic" "${kind^}" "$(truncate_text "$summary" 96)" "$ts"
        [[ -n "$rid" ]] && add_edge "run/$rid" "$generic" "$kind" "$ts"
      fi
      ;;
  esac
}

consume_sse() {
  local engine_url="$1" run_id="$2" params=""
  [[ -n "$run_id" ]] && params="?run_id=$run_id"
  {
    curl -sS -N "$engine_url/progress.sse$params" || true
  } | {
    local buffer=""
    while IFS= read -r line; do
      case "$line" in
        data:*)
          buffer+="${line#data: }"
          buffer+=$'\n'
          ;;
        "")
          if [[ -n "$buffer" ]]; then
            local payload
            payload="$(printf '%s' "$buffer" | sed 's/[[:space:]]*$//')"
            if [[ -n "$payload" && "$payload" != "[DONE]" ]]; then
              process_event "$payload"
            fi
            buffer=""
          fi
          ;;
      esac
    done
  }
}

get_node_row() {
  local id="$1"
  awk -F'\t' -v id="$id" '$1==id{print; exit}' "$NODES_FILE"
}

render_star_layout() {
  local id="$1"
  mapfile -t neighbors < <(
    awk -F'\t' -v id="$id" 'NR==FNR { labels[$1]=$3; kinds[$1]=$2; next }
      $1==id { printf "%s\t%s\t%s\t%s\tout\n", $2, labels[$2], kinds[$2], $3 }
      $2==id { printf "%s\t%s\t%s\t%s\tin\n", $1, labels[$1], kinds[$1], $3 }
    ' "$NODES_FILE" "$EDGES_FILE" | sort -u
  )
  if ((${#neighbors[@]} == 0)); then
    echo "  (no neighboring nodes yet)"
    return
  fi
  local center_label
  center_label=$(awk -F'\t' -v id="$id" '$1==id{print $3; exit}' "$NODES_FILE")
  local north="" east="" south="" west=""
  local entry
  if [[ -n "${neighbors[0]:-}" ]]; then
    IFS=$'\t' read -r _nid label kind etype _dir <<<"${neighbors[0]}"
    north="$(truncate_text "$label" 40) [$kind|$etype]"
  fi
  if [[ -n "${neighbors[1]:-}" ]]; then
    IFS=$'\t' read -r _nid label kind etype _dir <<<"${neighbors[1]}"
    east="$(truncate_text "$label" 30) [$kind|$etype]"
  fi
  if [[ -n "${neighbors[2]:-}" ]]; then
    IFS=$'\t' read -r _nid label kind etype _dir <<<"${neighbors[2]}"
    south="$(truncate_text "$label" 40) [$kind|$etype]"
  fi
  if [[ -n "${neighbors[3]:-}" ]]; then
    IFS=$'\t' read -r _nid label kind etype _dir <<<"${neighbors[3]}"
    west="$(truncate_text "$label" 30) [$kind|$etype]"
  fi
  if [[ -n "$north" ]]; then
    printf '          %s\n' "$north"
    printf '             |\n'
  fi
  local west_fmt="" east_fmt="" center_fmt
  if [[ -n "$west" ]]; then
    west_fmt="$(printf '%-24s' "$west")<-- "
  fi
  if [[ -n "$east" ]]; then
    east_fmt=" -->$(printf '%-24s' "$east")"
  fi
  center_fmt="$(truncate_text "$center_label" 42)"
  printf '%s[%s]%s\n' "$west_fmt" "$center_fmt" "$east_fmt" | sed 's/[[:space:]]*$//'
  if [[ -n "$south" ]]; then
    printf '             |\n'
    printf '          %s\n' "$south"
  fi
  if ((${#neighbors[@]} > 4)); then
    echo
    echo "  more neighbors:"
    local idx=4
    while (( idx < ${#neighbors[@]} )); do
      IFS=$'\t' read -r _nid label kind etype _dir <<<"${neighbors[$idx]}"
      printf '   - %s [%s|%s]\n' "$(truncate_text "$label" 48)" "$kind" "$etype"
      ((idx++))
    done
  fi
}

render_recommendations() {
  local id="$1" kind="$2" label="$3"
  case "$kind" in
    Deeplink)
      printf '  open: xdg-open "%s"  # or open on mac\n' "$label"
      ;;
    FileProposal)
      printf '  review proposal: jq '\''select(.path=="%s")'\'' out/bundle.json\n' "$label"
      printf '  compare: git diff -- "%s"\n' "$label"
      ;;
    FileWrite)
      printf '  inspect: bat --style=plain "%s"\n' "$label"
      printf '  diff staged: git diff --cached -- "%s"\n' "$label"
      ;;
    PlanStep)
      printf '  filter steps: rg "%s" .graphlogue/nodes.tsv\n' "$label"
      printf '  show run: rg "%s" receipts\n' "$id"
      ;;
    Run)
      printf '  inspect receipt: ls receipts | grep "%s"\n' "${id#run/}"
      printf '  tail log: rg "%s" %s\n' "${id#run/}" "$EVENTS_FILE"
      ;;
    NetRead)
      printf '  view cache: cat cache/%s  # if cached\n' "${id#read/}"
      printf '  re-fetch: curl -s "%s" | head\n' "$label"
      ;;
    KPI)
      printf '  audit receipts: rg "decision_agreement" receipts\n'
      ;;
    Note)
      printf '  grep note: rg "%s" %s\n' "$id" "$EVENTS_FILE"
      ;;
    *)
      printf '  search events: rg "%s" %s\n' "$id" "$EVENTS_FILE"
      printf '  list neighbors: rg "%s" %s\n' "$id" "$EDGES_FILE"
      ;;
  esac
}

preview_node() {
  local id="$1" row
  if [[ -z "$id" ]]; then
    echo "(no selection)"
    return
  fi
  row="$(get_node_row "$id")"
  if [[ -z "$row" ]]; then
    printf 'Node %s not indexed yet.\n' "$id"
    return
  fi
  local kind label ts human_ts
  IFS=$'\t' read -r _ kind label ts <<<"$row"
  human_ts=$(date -d @${ts:0:10} 2>/dev/null || printf '%s' "$ts")
  printf ':: %s\n' "$id"
  printf 'Kind: %s\n' "$kind"
  printf 'Label: %s\n' "$label"
  printf 'Seen: %s\n' "$human_ts"
  echo
  echo 'Neighbors (ASCII star):'
  render_star_layout "$id"
  echo
  echo 'Recent events:'
  tail -n 8 "$EVENTS_FILE" 2>/dev/null | jq -C -r 'try .kind + " " + (.run_id // "-") + " :: " + (.summary // .message // .intent // .purpose // "") catch ""' | sed '/^$/d' | tail -n 5 || true
  echo
  echo 'Recommendations:'
  render_recommendations "$id" "$kind" "$label"
}

launch_viewer() {
  local script="$1"
  FZF_DEFAULT_COMMAND="cat \"$NODES_FILE\"" \
  fzf \
    --prompt="graphlogue> " \
    --with-nth=2,3 \
    --layout=reverse \
    --preview-window=down,70% \
    --bind="change:reload(cat $NODES_FILE 2>/dev/null)" \
    --bind="ctrl-r:reload(cat $NODES_FILE 2>/dev/null)" \
    --preview "bash -lc '$script preview {1}'" \
    < "$NODES_FILE"
}

main() {
  if [[ ${1-} == preview ]]; then
    shift
    init_store
    preview_node "${1:-}"
    exit 0
  fi

  local run_id="" engine_url="${ENGINE_URL:-http://127.0.0.1:8080}" start_stream=1
  while (($#)); do
    case "$1" in
      -r|--run-id)
        run_id="$2"; shift 2;
        ;;
      -u|--engine-url)
        engine_url="$2"; shift 2;
        ;;
      --no-stream)
        start_stream=0; shift;
        ;;
      -h|--help)
        usage; exit 0;
        ;;
      *)
        printf 'graphlogue: unknown option %s\n' "$1" >&2
        usage
        exit 1
        ;;
    esac
  done

  ensure_prereqs
  init_store

  local stream_pid=""
  if (( start_stream )); then
    consume_sse "$engine_url" "$run_id" >>"$STREAM_LOG" 2>&1 &
    stream_pid=$!
  fi

  trap '[[ -n "$stream_pid" ]] && kill "$stream_pid" 2>/dev/null || true' EXIT INT TERM
  launch_viewer "$SCRIPT_PATH"
}

main "$@"
