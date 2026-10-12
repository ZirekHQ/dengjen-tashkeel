#!/usr/bin/env bash
set -euo pipefail

port=dengjen-tashkeel-capi
manifest="ports/${port}/vcpkg.json"
versions="versions/d-/${port}.json"
baseline=versions/baseline.json

tree="$(git rev-parse "HEAD:ports/${port}")"
recorded="$(jq -r '.versions[0]["git-tree"]' "$versions")"
if [ "$tree" != "$recorded" ]; then
  echo "::error::${versions} records git-tree ${recorded}, but ports/${port} is ${tree}; follow packaging/README.md step 4" >&2
  exit 1
fi

port_version="$(jq -r '"\(.version) \(.["port-version"] // 0)"' "$manifest")"
entry_version="$(jq -r '.versions[0] | "\(.version) \(.["port-version"])"' "$versions")"
baseline_version="$(jq -r --arg p "$port" '.default[$p] | "\(.baseline) \(.["port-version"])"' "$baseline")"
if [ "$port_version" != "$entry_version" ] || [ "$port_version" != "$baseline_version" ]; then
  echo "::error::vcpkg.json (${port_version}), ${versions} (${entry_version}) and baseline.json (${baseline_version}) disagree" >&2
  exit 1
fi
echo "vcpkg registry entry matches ports/${port} at ${port_version} (${tree})"
