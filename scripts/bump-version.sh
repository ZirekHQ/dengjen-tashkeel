#!/usr/bin/env bash
set -euo pipefail

# Scope is intentionally workspace-only. vcpkg/Conan bump separately once
# release archives exist (checksums can't be computed before then) --
# see CONTRIBUTING.md step 3.
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

require_line() {
  local file="$1" line="$2"
  if ! grep -qxF -- "$line" "$file"; then
    echo "::error::Expected '${line}' in ${file} after bump -- format drift?" >&2
    exit 1
  fi
}

# awk instead of GNU-only `sed 0,/re/` so the script also runs on BSD/macOS.
awk -v old="$old_version" -v new="$new_version" '
  /^\[workspace\.package\]/ {f=1; print; next}
  /^\[/ {f=0}
  f && !done && $0 == "version = \"" old "\"" {print "version = \"" new "\""; done=1; next}
  {print}
' Cargo.toml > Cargo.toml.new
mv Cargo.toml.new Cargo.toml
require_line Cargo.toml "version = \"${new_version}\""

sed -i.bak "s/^version = \"${old_version}\"\$/version = \"${new_version}\"/" bindings/java/build.gradle.kts
rm -f bindings/java/build.gradle.kts.bak
require_line bindings/java/build.gradle.kts "version = \"${new_version}\""

sed -i.bak "s/dengjen-tashkeel:${old_version}/dengjen-tashkeel:${new_version}/g" README.md
rm -f README.md.bak
if grep -qE "dengjen-tashkeel:${old_version//./\\.}([^0-9A-Za-z.]|\$)" README.md; then
  echo "::error::README.md still references dengjen-tashkeel:${old_version}" >&2
  exit 1
fi
grep -qF "dengjen-tashkeel:${new_version}" README.md || {
  echo "::error::README.md has no dengjen-tashkeel:${new_version} coordinate" >&2
  exit 1
}

for sub_crate_toml in crates/capi/Cargo.toml crates/cli/Cargo.toml crates/python/Cargo.toml; do
  # Read each file's own current pin rather than reusing $old_version: the pin can already
  # lag behind [workspace.package].version, and matching against $old_version would silently
  # no-op on a file that's already stale instead of correcting it.
  sub_old_version="$(awk '/^\[dependencies\.dengjen-tashkeel\]/{f=1;next} /^\[/{f=0} f && /^version = /{gsub(/version = "|"/,""); print; exit}' "$sub_crate_toml")"
  if [ -z "$sub_old_version" ]; then
    echo "::error::Couldn't find [dependencies.dengjen-tashkeel].version in ${sub_crate_toml}" >&2
    exit 1
  fi
  sed -i.bak "/^\[dependencies\.dengjen-tashkeel\]/,/^\[/{s/^version = \"${sub_old_version}\"\$/version = \"${new_version}\"/}" "$sub_crate_toml"
  rm -f "${sub_crate_toml}.bak"
done

cargo check --quiet

echo "Bumped ${old_version} -> ${new_version}:"
git diff --stat -- Cargo.toml Cargo.lock bindings/java/build.gradle.kts README.md \
  crates/capi/Cargo.toml crates/cli/Cargo.toml crates/python/Cargo.toml
