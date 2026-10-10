#!/usr/bin/env bash
set -euo pipefail

# usage: wait-for-checks.sh <sha> <check-name>...
# Blocks until every named check run on <sha> has completed, then fails unless
# all runs carrying each name concluded successfully. Needs GH_TOKEN and GITHUB_REPOSITORY.
sha="${1:?usage: wait-for-checks.sh <sha> <check-name>...}"
shift
[ "$#" -gt 0 ] || { echo "::error::no check names given" >&2; exit 2; }

max_attempts="${MAX_ATTEMPTS:-90}"
sleep_seconds="${SLEEP_SECONDS:-20}"

for attempt in $(seq 1 "$max_attempts"); do
  runs="$(gh api --paginate "repos/${GITHUB_REPOSITORY}/commits/${sha}/check-runs?per_page=100" \
    --jq '.check_runs[] | [.name, .status, (.conclusion // "")] | @tsv')"
  pending=()
  failed=()
  for name in "$@"; do
    matching="$(awk -F'\t' -v n="$name" '$1 == n' <<< "$runs")"
    if [ -z "$matching" ] || grep -qvP '\tcompleted\t' <<< "$matching"; then
      pending+=("$name")
    elif awk -F'\t' '$3 != "success" && $3 != "skipped"' <<< "$matching" | grep -q .; then
      failed+=("$name")
    fi
  done
  if [ "${#failed[@]}" -gt 0 ]; then
    echo "::error::Checks failed on ${sha}: ${failed[*]}" >&2
    exit 1
  fi
  if [ "${#pending[@]}" -eq 0 ]; then
    echo "All required checks passed on ${sha}: $*"
    exit 0
  fi
  echo "Waiting on ${pending[*]} (attempt ${attempt}/${max_attempts})..."
  sleep "$sleep_seconds"
done

echo "::error::Timed out waiting for checks on ${sha}" >&2
exit 1
