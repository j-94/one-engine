#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required to build the production branch" >&2
  exit 1
fi

BASE_BRANCH_INPUT="${1:-}"
TARGET_BRANCH="${2:-production}"
REPORT_PATH="${3:-engine/_output/merge_branches_report.json}"

current_head_ref="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || git rev-parse --short HEAD)"

if [[ -z "$BASE_BRANCH_INPUT" ]]; then
  BASE_BRANCH_INPUT="$current_head_ref"
fi

if [[ ! -f "$REPORT_PATH" ]]; then
  echo "error: merge report '$REPORT_PATH' was not found" >&2
  exit 1
fi

resolve_ref() {
  local name="$1"
  local -a candidates=()

  if [[ "$name" == */* ]]; then
    candidates=("$name")
  else
    candidates=("origin/$name" "$name")
  fi

  local candidate refname
  for candidate in "${candidates[@]}"; do
    if [[ "$candidate" == */* ]]; then
      refname="refs/remotes/$candidate"
    else
      refname="refs/heads/$candidate"
    fi

    if git show-ref --verify --quiet "$refname"; then
      printf '%s' "$candidate"
      return 0
    fi
  done

  return 1
}

base_ref=""
if ! base_ref=$(resolve_ref "$BASE_BRANCH_INPUT"); then
  echo "error: base branch '$BASE_BRANCH_INPUT' does not exist" >&2
  exit 1
fi

if git show-ref --verify --quiet "refs/heads/$TARGET_BRANCH"; then
  echo "error: target branch '$TARGET_BRANCH' already exists" >&2
  exit 1
fi

mapfile -t report_entries < <(jq -c '.[] | select(.branch != null)' "$REPORT_PATH")

if [[ ${#report_entries[@]} -eq 0 ]]; then
  echo "error: merge report '$REPORT_PATH' did not contain any actionable branches" >&2
  exit 1
fi

original_head="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || git rev-parse HEAD)"
export GIT_MERGE_AUTOEDIT=no

if [[ "$base_ref" == */* ]]; then
  git fetch --quiet "${base_ref%%/*}" "${base_ref#*/}" >/dev/null 2>&1 || true
fi

git checkout --quiet -B "$TARGET_BRANCH" "$base_ref"

declare -a skipped

for entry in "${report_entries[@]}"; do
  branch_name="$(jq -r '.branch' <<<"$entry")"
  status="$(jq -r '.status' <<<"$entry")"

  case "$status" in
    merged)
      echo "info: '$branch_name' already merged into $base_ref; skipping" >&2
      skipped+=("$branch_name (already merged)")
      continue
      ;;
    fast_forward)
      ;;
    *)
      note="$(jq -r '.note // ""' <<<"$entry")"
      echo "warn: skipping '$branch_name' due to status '$status'${note:+ - $note}" >&2
      skipped+=("$branch_name ($status)")
      continue
      ;;
  esac

  if ! ref=$(resolve_ref "$branch_name"); then
    echo "warn: could not locate branch '$branch_name'; skipping" >&2
    skipped+=("$branch_name (missing)")
    continue
  fi

  echo "info: merging $ref into $TARGET_BRANCH" >&2
  if ! git merge --no-ff --no-edit "$ref"; then
    echo "error: merge of '$ref' into '$TARGET_BRANCH' failed" >&2
    git merge --abort >/dev/null 2>&1 || true
    if [[ "$original_head" != "$TARGET_BRANCH" ]]; then
      git checkout --quiet "$original_head"
    fi
    exit 1
  fi

done

if [[ "$original_head" != "$TARGET_BRANCH" ]]; then
  git checkout --quiet "$original_head"
fi

if [[ ${#skipped[@]} -gt 0 ]]; then
  printf 'note: skipped branches -> %s\n' "${skipped[*]}" >&2
fi

echo "Production branch '$TARGET_BRANCH' created from base '$base_ref'." >&2
