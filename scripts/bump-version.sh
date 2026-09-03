#!/usr/bin/env bash
# Bumps every hand-synced copy of the workspace version to the given value.
#
# Cargo.toml's [workspace.package].version is the single source of truth;
# every other file here mirrors it because it can't consume it directly
# (bindings/java/build.gradle.kts per its own comment; README's Java
# coordinate examples mirror that file). vcpkg/Conan are NOT touched here --
# those can't be bumped until the release archives they hash actually exist,
# so they always land in a follow-up PR once the tag has been pushed (see
# packaging/README.md).
#
# Usage: scripts/bump-version.sh 1.5.3
set -euo pipefail

new_version="${1:?usage: scripts/bump-version.sh <new-version>}"
if ! echo "$new_version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "::error::'${new_version}' doesn't look like a semver version (X.Y.Z)" >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

old_version="$(awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f && /^version = /{gsub(/version = "|"/,""); print; exit}' Cargo.toml)"
if [ -z "$old_version" ]; then
  echo "::error::Couldn't find [workspace.package].version in Cargo.toml" >&2
  exit 1
fi

sed -i.bak "0,/^version = \"${old_version}\"\$/s//version = \"${new_version}\"/" Cargo.toml
rm -f Cargo.toml.bak

sed -i.bak "s/version = \"${old_version}\"/version = \"${new_version}\"/" bindings/java/build.gradle.kts
rm -f bindings/java/build.gradle.kts.bak

sed -i.bak "s/dengjen-tashkeel:${old_version}/dengjen-tashkeel:${new_version}/g" README.md
rm -f README.md.bak

echo "Bumped ${old_version} -> ${new_version}:"
git diff --stat -- Cargo.toml bindings/java/build.gradle.kts README.md
