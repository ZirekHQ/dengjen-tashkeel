#!/usr/bin/env bash
# Fingerprints everything that determines the Java native libraries: the core and capi sources
# (workspace version lines stripped, so a version bump alone is not a native change) and the
# third-party crates the published build links. The model ships as its own artifact, so it is
# left out.
#
#   natives-fingerprint.sh            print the fingerprint
#   natives-fingerprint.sh --update   rewrite bindings/java/natives.fingerprint
#   natives-fingerprint.sh --bump     --update, then bump the nativesVersion patch if the fingerprint moved
#   natives-fingerprint.sh --check    fail when the committed fingerprint is stale
#   natives-fingerprint.sh --check-bump <base-ref>
#                                     fail when the fingerprint changed but nativesVersion did not
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

record=bindings/java/natives.fingerprint

fingerprint() {
  {
    git ls-files Cargo.toml rust-toolchain.toml .cargo crates/core crates/capi \
      | grep -E '(\.(rs|toml|json)|/config)$' \
      | LC_ALL=C sort \
      | while read -r file; do
          printf '== %s\n' "$file"
          case "$file" in
            *.toml) sed -E 's/version = "[^"]*"//' "$file" ;;
            *) cat "$file" ;;
          esac
        done
    printf '== dependencies\n'
    cargo tree --color never --locked -p dengjen-tashkeel-capi --no-default-features --features ort-dylib \
      --target all -e normal,build,features --prefix none --no-dedupe \
      | grep -v '(/' | LC_ALL=C sort -u
  } | sha256sum | cut -d' ' -f1
}

case "${1:-}" in
  "") fingerprint ;;
  --update) fingerprint > "$record"; echo "updated $record" ;;
  --bump)
    gradle=bindings/java/build.gradle.kts
    before="$(tr -d '[:space:]' < "$record")"
    fingerprint > "$record"
    [ "$before" = "$(tr -d '[:space:]' < "$record")" ] && exit 0
    read -r major minor patch < <(grep -m1 '^val nativesVersion = ' "$gradle" \
      | sed -E 's/.*"([0-9]+)\.([0-9]+)\.([0-9]+)".*/\1 \2 \3/')
    next="${major}.${minor}.$((patch + 1))"
    sed -E "s/^(val nativesVersion = \")[^\"]*(\")/\1${next}\2/" "$gradle" > "$gradle.tmp"
    mv "$gradle.tmp" "$gradle"
    echo "natives changed: nativesVersion -> ${next}"
    ;;
  --check)
    current="$(fingerprint)"
    if [ "$current" != "$(tr -d '[:space:]' < "$record")" ]; then
      echo "::error::the native inputs changed: run scripts/natives-fingerprint.sh --update and bump nativesVersion in bindings/java/build.gradle.kts" >&2
      exit 1
    fi
    ;;
  --check-bump)
    base="${2:?usage: $0 --check-bump <base-ref>}"
    nativesVersion() { grep -m1 '^val nativesVersion = ' ; }
    if [ "$(git show "${base}:${record}" | tr -d '[:space:]')" != "$(tr -d '[:space:]' < "$record")" ] \
      && [ "$(git show "${base}:bindings/java/build.gradle.kts" | nativesVersion)" = "$(nativesVersion < bindings/java/build.gradle.kts)" ]; then
      echo "::error::natives.fingerprint changed but nativesVersion did not; bump nativesVersion in bindings/java/build.gradle.kts" >&2
      exit 1
    fi
    ;;
  *) echo "usage: $0 [--update|--bump|--check|--check-bump <base-ref>]" >&2; exit 2 ;;
esac
