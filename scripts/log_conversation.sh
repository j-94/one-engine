#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ]; then
  echo "Usage: $0 <label> <branch_id> <prompt_file> <response_file> [receipts_file]" >&2
  exit 1
fi

LABEL="$1"
BRANCH_ID="$2"
PROMPT_FILE="$3"
RESPONSE_FILE="$4"
RECEIPTS_FILE="${5:-}"

PROMPT_CONTENT=$(cat "$PROMPT_FILE")
RESPONSE_CONTENT=$(cat "$RESPONSE_FILE")
RECEIPTS_CONTENT=""
if [ -n "$RECEIPTS_FILE" ]; then
  RECEIPTS_CONTENT=$(cat "$RECEIPTS_FILE")
fi

TIMESTAMP=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
cat <<EOF >> conversation.md
### ${LABEL} — branch ${BRANCH_ID} (${TIMESTAMP})

#### Prompt
```
${PROMPT_CONTENT}
```

#### Engine Response
```
${RESPONSE_CONTENT}
```

#### Receipts
```
${RECEIPTS_CONTENT}
```

EOF
