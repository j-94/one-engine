#!/usr/bin/env bash
set -euo pipefail

nodes=.graphlogue/nodes.tsv
edges=.graphlogue/edges.tsv

preview() {
  id="$1"
  echo ":: $id"
  awk -F'\t' -v id="$id" '$1==id{print "Kind:",$2; print "Label:",$3}' "$nodes"
  echo
  echo "Neighbors:"
  awk -F'\t' -v id="$id" '$1==id{print "→",$2,"(" $3 ")"} $2==id{print "←",$1,"(" $3 ")"}' "$edges" \
    | sed 's/\t/ /g' | head -n 30
  echo
  echo "Recommendations:"
  # simple heuristics
  kind=$(awk -F'\t' -v id="$id" '$1==id{print $2}' "$nodes")
  label=$(awk -F'\t' -v id="$id" '$1==id{print $3}' "$nodes")
  case "$kind" in
    Deeplink) echo " ↪ open: xdg-open \"$label\" ;; mac: open \"$label\"" ;;
    FileProposal|FileWrite) echo " diff: git diff --no-index -- \"$label\" \"$label\" # (proposed vs written)";;
    PlanStep) echo " filter: grep \"$label\" .graphlogue/nodes.tsv" ;;
    Read) echo " fetch-cache: cat cache/$(basename "$id")  # if you keep cached bodies" ;;
    *) echo " inspect-run: grep \"$id\" receipts/*" ;;
  esac
}

fzf --prompt="graphlogue> " \
    --with-nth=2,3 \
    --preview-window=down,60% \
    --bind "change:reload(cat $nodes)" \
    --bind "enter:execute-silent(echo {1} > .graphlogue/last && echo {})" \
    --preview 'bash -lc "preview {1}"' \
    < "$nodes"
