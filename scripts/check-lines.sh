#!/usr/bin/env bash
# Source file size check.
#   - warns for files with more than WARN_LIMIT lines (default 300)
#   - fails for files with more than FAIL_LIMIT lines (default 400)
#     unless the path is listed in the allowlist
# Skipped directories: node_modules, target, out, .next, .git, dist, .engineering.
#
# Usage: check-lines.sh [options] [root-dir]
#   --warn N         warning threshold            (env WARN_LIMIT, default 300)
#   --fail N         failure threshold            (env FAIL_LIMIT, default 400)
#   --allowlist F    allowlist file, one path per line relative to the root, '#' = comment
#                    (env ALLOWLIST, default scripts/lines-allowlist.txt below the root)
#   --ext "a b c"    extensions to check, space or comma separated
#                    (env EXTENSIONS, default "rs ts tsx go js jsx py")
#   -h, --help       show this help
# root-dir defaults to the current directory. Options win over environment variables.
set -euo pipefail

WARN_LIMIT="${WARN_LIMIT:-300}"
FAIL_LIMIT="${FAIL_LIMIT:-400}"
ALLOWLIST="${ALLOWLIST:-}"
EXTENSIONS="${EXTENSIONS:-rs ts tsx go js jsx py}"
ROOT=""

# Prints the header comment (usage text).
usage() { sed -n '2,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; }

# Exits with an error message for an invalid call.
die() { echo "check-lines: $*" >&2; exit 2; }

while [ $# -gt 0 ]; do
  case "$1" in
    --warn) WARN_LIMIT="${2:?--warn needs a value}"; shift 2 ;;
    --fail) FAIL_LIMIT="${2:?--fail needs a value}"; shift 2 ;;
    --allowlist) ALLOWLIST="${2:?--allowlist needs a value}"; shift 2 ;;
    --ext) EXTENSIONS="${2:?--ext needs a value}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    -*) die "unknown option: $1" ;;
    *) [ -z "$ROOT" ] || die "only one root-dir allowed"; ROOT="$1"; shift ;;
  esac
done

ROOT="${ROOT:-.}"
[ -d "$ROOT" ] || die "root-dir not found: $ROOT"
ROOT="$(cd "$ROOT" && pwd)"
[[ "$WARN_LIMIT" =~ ^[0-9]+$ && "$FAIL_LIMIT" =~ ^[0-9]+$ ]] || die "thresholds must be integers"
ALLOWLIST="${ALLOWLIST:-scripts/lines-allowlist.txt}"
# A relative allowlist path is relative to the root.
case "$ALLOWLIST" in /*) ;; *) ALLOWLIST="$ROOT/$ALLOWLIST" ;; esac

cd "$ROOT"

# Reads the allowlist: one path per line (relative to the repo root), '#' starts a comment.
is_allowed() {
  [ -f "$ALLOWLIST" ] || return 1
  grep -v '^[[:space:]]*#' "$ALLOWLIST" | sed 's/[[:space:]]*$//' | grep -Fxq -- "$1"
}

# Builds the find expression for the configured extensions.
name_args=()
for ext in ${EXTENSIONS//,/ }; do
  ext="${ext#.}"
  [ "${#name_args[@]}" -eq 0 ] || name_args+=(-o)
  name_args+=(-name "*.${ext}")
done
[ "${#name_args[@]}" -gt 0 ] || die "no extensions given"

warnings=0
failures=0

while IFS= read -r -d '' file; do
  path="${file#./}"
  lines=$(wc -l < "$file")
  if [ "$lines" -gt "$FAIL_LIMIT" ]; then
    if is_allowed "$path"; then
      echo "ALLOWED  $path: $lines lines (> $FAIL_LIMIT, in allowlist)"
    else
      echo "ERROR    $path: $lines lines (> $FAIL_LIMIT) - split the file or justify it in the allowlist"
      echo "::error file=$path::$lines Zeilen (> $FAIL_LIMIT) - Datei aufteilen oder in der Allowlist begründen" 2>/dev/null || true
      failures=$((failures + 1))
    fi
  elif [ "$lines" -gt "$WARN_LIMIT" ]; then
    echo "WARNING  $path: $lines lines (> $WARN_LIMIT)"
    warnings=$((warnings + 1))
  fi
done < <(find . \
  \( -name node_modules -o -name target -o -name out -o -name .next -o -name .git -o -name dist -o -name .engineering \) -prune -o \
  -type f \( "${name_args[@]}" \) -print0)

echo "check-lines: $warnings warning(s), $failures error(s)"
[ "$failures" -eq 0 ]
