#!/usr/bin/env bash
# Fail if tracked text files contain developer-local absolute paths.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [ "$#" -gt 0 ]; then
  FILES=("$@")
else
  mapfile -t FILES < <(
    git ls-files --cached --others --exclude-standard \
      '*.rs' '*.sh' '*.json' '*.md' '*.toml' '*.yml' '*.yaml' '*.html' '*.css' '*.txt'
  )
fi

FAILED=0
for file in "${FILES[@]}"; do
  case "$file" in
    scripts/check-no-local-paths.sh) continue ;;
  esac
  [ -f "$file" ] || continue
  while IFS= read -r hit; do
    if printf '%s\n' "$hit" | grep -q 'allow-local-path:'; then
      continue
    fi
    printf '%s\n' "$hit" >&2
    FAILED=1
  done < <(
    grep -En \
      -e '/Users/[A-Za-z0-9._-]+/' \
      -e '/home/[A-Za-z0-9._-]+/' \
      -e '[A-Za-z]:\\Users\\[A-Za-z0-9._-]+\\' \
      -e '/private/var/folders/' \
      -e '(^|[[:space:]"'\''`(=])~/[.[:alnum:]_][[:alnum:]_.-]*' \
      -e '\$HOME/[[:alnum:]_.-]+' \
      "$file" 2>/dev/null | sed "s#^#$file:#"
  )
done

if [ "$FAILED" != "0" ]; then
  echo "local path leak detected" >&2
  exit 1
fi

echo "OK no local paths detected"
