#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required to generate merge reports" >&2
  exit 1
fi

BASE_BRANCH="${1:-}"
OUTPUT_DIR="engine/_output"
OUTPUT_JSON="$OUTPUT_DIR/merge_branches_report.json"
OUTPUT_JSONL="$OUTPUT_JSON.tmp.jsonl"

mkdir -p "$OUTPUT_DIR"

# Capture the current HEAD description for informational messages.
current_head="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || git rev-parse --short HEAD)"

if [[ -z "$BASE_BRANCH" ]]; then
  BASE_BRANCH="$current_head"
fi

if ! git show-ref --verify --quiet "refs/heads/$BASE_BRANCH" && \
   ! git show-ref --verify --quiet "refs/remotes/$BASE_BRANCH"; then
  echo "error: base branch '$BASE_BRANCH' does not exist" >&2
  exit 1
fi

mapfile -t remote_branches < <(git for-each-ref --format='%(refname:short)' refs/remotes/ | grep -v '/HEAD$' || true)

if [[ ${#remote_branches[@]} -eq 0 ]]; then
  jq -n '[{branch:null,status:"skipped",note:"No remote branches were found. Configure remotes before regenerating merge status."}]' > "$OUTPUT_JSON"
  exit 0
fi

trap 'rm -f "$OUTPUT_JSONL"' EXIT
> "$OUTPUT_JSONL"

for remote in "${remote_branches[@]}"; do
  branch_display="${remote#*/}"
  status="diverged"
  note="Branches have diverged; manual merge required to reconcile histories."

  if git merge-base --is-ancestor "$remote" "$BASE_BRANCH"; then
    status="merged"
    note="Remote branch tip is already merged into $BASE_BRANCH."
  elif git merge-base --is-ancestor "$BASE_BRANCH" "$remote"; then
    status="fast_forward"
    note="Remote branch can fast-forward cleanly into $BASE_BRANCH."
  fi

  jq -n --arg branch "$branch_display" --arg status "$status" --arg note "$note" \
    '{branch:$branch,status:$status,note:$note}' >> "$OUTPUT_JSONL"
done

jq -s '.' "$OUTPUT_JSONL" > "$OUTPUT_JSON"
