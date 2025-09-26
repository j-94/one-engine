#!/usr/bin/env bash
set -euo pipefail

mkdir -p .graphlogue
nodes=.graphlogue/nodes.tsv   # id<TAB>kind<TAB>label
edges=.graphlogue/edges.tsv   # src<TAB>dst<TAB>type

touch "$nodes" "$edges"

upsert_node(){ # id kind label
  awk -v id="$1" -v k="$2" -v l="$3" -F'\t' 'BEGIN{found=0}
    $1==id{print id "\t" k "\t" l; found=1; next}1
    END{ if(!found) print id "\t" k "\t" l }' "$nodes" > "$nodes.tmp" && mv "$nodes.tmp" "$nodes"
}

add_edge(){ echo -e "$1\t$2\t$3" >> "$edges"; }

while IFS= read -r line; do
  kind=$(jq -r '.kind // empty' <<<"$line") || continue
  case "$kind" in
    run.start)
      rid=$(jq -r '.run_id' <<<"$line"); intent=$(jq -r '.intent' <<<"$line")
      iid="intent/$(printf "%s" "$intent" | sha1sum | awk '{print $1}')"
      upsert_node "run/$rid" "Run" "$rid"
      upsert_node "$iid" "Intent" "$intent"
      add_edge "run/$rid" "$iid" "RUN_INTENT"
      ;;
    plan.step)
      rid=$(jq -r '.run_id' <<<"$line"); n=$(jq -r '.n' <<<"$line"); purpose=$(jq -r '.purpose' <<<"$line")
      sid="step/$rid/$n"
      upsert_node "$sid" "PlanStep" "$purpose"
      ;;
    net.read)
      h=$(jq -r '.sha256' <<<"$line"); url=$(jq -r '.url' <<<"$line")
      upsert_node "read/$h" "Read" "$url"
      ;;
    bundle.proposed)
      rid=$(jq -r '.run_id' <<<"$line")
      jq -r '.files[] | [.path, .sha256] | @tsv' <<<"$line" | while IFS=$'\t' read -r p s; do
        upsert_node "filep/$s" "FileProposal" "$p"
      done
      ;;
    gate.eval)
      rid=$(jq -r '.run_id' <<<"$line"); st=$(jq -r '.step' <<<"$line"); dec=$(jq -r '.decision'<<<"$line")
      upsert_node "gate/$rid/$st" "GateDecision" "$dec"
      ;;
    file.write)
      p=$(jq -r '.path' <<<"$line"); s=$(jq -r '.sha256' <<<"$line")
      upsert_node "filew/$s" "FileWrite" "$p"
      ;;
    deeplink)
      url=$(jq -r '.url' <<<"$line"); lid="link/$(printf "%s" "$url" | sha1sum | awk '{print $1}')"
      upsert_node "$lid" "Deeplink" "$url"
      ;;
    kpi)
      rid=$(jq -r '.run_id' <<<"$line"); da=$(jq -r '.decision_agreement'<<<"$line")
      kid="kpi/$rid/$(date +%s%3N)"
      upsert_node "$kid" "KPI" "agreement=$da"
      ;;
    run.halt)
      rid=$(jq -r '.run_id' <<<"$line"); code=$(jq -r '.code'<<<"$line")
      upsert_node "halt/$rid/$code" "Halt" "$code"
      ;;
  esac
done
