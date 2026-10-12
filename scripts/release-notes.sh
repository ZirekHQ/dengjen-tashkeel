#!/usr/bin/env bash
set -euo pipefail

# Prints Markdown release notes for <tag>: the Conventional Commit subjects between the
# previous vX.Y.Z tag and <tag>, grouped by type. PR numbers in `(#N)` autolink on GitHub.
tag="${1:?usage: scripts/release-notes.sh <vX.Y.Z>}"

previous="$(git tag --list 'v*' --sort=-v:refname \
  | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' \
  | awk -v t="$tag" 'found {print; exit} $0 == t {found = 1}')"
range="${previous:+${previous}..}${tag}"

git log "$range" --no-merges --pretty='%s' | awk '
  /^chore: bump workspace version -> / { next }
  {
    subject = $0
    if (match(subject, /^[a-zA-Z]+(\([^)]*\))?!?: /)) {
      head = substr(subject, 1, RLENGTH - 2)
      text = substr(subject, RLENGTH + 1)
      breaking = (head ~ /!$/)
      sub(/!$/, "", head)
      type = head
      sub(/\(.*$/, "", type)
      scope = ""
      if (match(head, /\(.*\)/)) scope = substr(head, RSTART + 1, RLENGTH - 2)
      if (scope ~ /^deps/ && !breaking) { deps++; next }
      section = breaking ? "Breaking changes" : (type == "feat" ? "Features" : (type == "fix" ? "Fixes" : "Maintenance"))
      line = scope != "" ? "**" scope ":** " text : text
    } else {
      section = "Other changes"
      line = subject
    }
    items[section] = items[section] "- " line "\n"
  }
  END {
    if (deps > 0) items["Maintenance"] = items["Maintenance"] "- " deps " dependency update" (deps > 1 ? "s" : "") "\n"
    n = split("Breaking changes|Features|Fixes|Maintenance|Other changes", order, "|")
    printed = 0
    print "## What'"'"'s changed\n"
    for (i = 1; i <= n; i++) {
      if (items[order[i]] != "") {
        printf "### %s\n\n%s\n", order[i], items[order[i]]
        printed = 1
      }
    }
    if (!printed) print "No user-facing changes."
  }'
