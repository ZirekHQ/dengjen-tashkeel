# Security Policy

## Reporting a vulnerability

Report suspected vulnerabilities privately via
[GitHub Private Vulnerability Reporting](https://github.com/ZirekHQ/dengjen-tashkeel/security/advisories/new)
(Security tab → Report a vulnerability). Do not open a public issue for a suspected
vulnerability.

Include, where possible: the affected crate/version, a minimal reproduction, and the impact
(memory safety, DoS, information disclosure, etc.).

## Scope

Libtashkeel embeds onnxruntime via FFI. The public C ABI (`crates/capi`) is in scope, as is any
`unsafe` code in `crates/capi` and `crates/core`. Denial-of-service reports against malformed
ONNX model files are in scope; resource-exhaustion reports against large-but-well-formed inputs
are lower priority.

## Supported versions

This project does not yet maintain parallel release branches — security fixes land on `main`.
