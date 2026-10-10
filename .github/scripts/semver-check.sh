#!/usr/bin/env bash
set -euo pipefail

# Runs cargo-semver-checks against the release type scripts/next-version.sh
# would cut for the commits since the last tag. A breaking change that the
# commit history only classifies as minor/patch fails here, before release.
package="${1:?usage: semver-check.sh <package>}"

set +e
next="$(bash scripts/next-version.sh)"
rc=$?
set -e
if [ "$rc" -eq 1 ]; then
  echo "::notice::No release-worthy commits since the last tag -- skipping semver check"
  exit 0
elif [ "$rc" -ne 0 ]; then
  exit "$rc"
fi

case "$next" in
  *.0.0) release_type="major" ;;
  *.0) release_type="minor" ;;
  *) release_type="patch" ;;
esac

echo "Checking ${package} for a ${release_type} release (${next})"
cargo semver-checks check-release -p "$package" --release-type "$release_type"
