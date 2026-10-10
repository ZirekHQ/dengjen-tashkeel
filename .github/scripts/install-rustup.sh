#!/usr/bin/env bash
set -euo pipefail

# Pinned rustup-init binary from the versioned archive path, which is
# immutable (unlike the sh.rustup.rs redirect). Bump version and both hashes
# together from https://static.rust-lang.org/rustup/archive/<version>/<triple>/rustup-init.sha256.
version="1.29.1"
case "$(uname -m)" in
  x86_64)
    triple="x86_64-unknown-linux-gnu"
    sha256="dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71"
    ;;
  aarch64 | arm64)
    triple="aarch64-unknown-linux-gnu"
    sha256="15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433"
    ;;
  *)
    echo "::error::No pinned rustup-init for $(uname -m)" >&2
    exit 1
    ;;
esac

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl --proto '=https' --tlsv1.2 -sSfL \
  "https://static.rust-lang.org/rustup/archive/${version}/${triple}/rustup-init" \
  -o "${tmp}/rustup-init"
echo "${sha256}  ${tmp}/rustup-init" | sha256sum -c -
chmod +x "${tmp}/rustup-init"
"${tmp}/rustup-init" -y "$@"
