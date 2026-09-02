# Packaging

`dengjen-tashkeel-capi`'s vcpkg port (`ports/dengjen-tashkeel-capi/`, registered
via `versions/`) and Conan recipe (`packaging/conan/conanfile.py`) both
download the prebuilt cdylib + header from a GitHub Release instead of
building from source — neither vcpkg nor Conan has a Rust toolchain available
in its build environment. See [issue #24](https://github.com/ZirekHQ/dengjen-tashkeel/issues/24).

Both were scaffolded before the capi artifacts they reference existed in any
release, so every checksum in them is a **placeholder** (`0` repeated to the
right length). Once a release tagged `v<version>` actually ships the
`dengjen-tashkeel-capi-<target>` archives, a maintainer must fill them in:

1. Download each release asset and compute its checksum:
   ```bash
   version=1.5.2
   for target in aarch64-apple-darwin x86_64-unknown-linux-gnu; do
     asset="dengjen-tashkeel-capi-${target}.tar.xz"
     curl -sLO "https://github.com/ZirekHQ/dengjen-tashkeel/releases/download/v${version}/${asset}"
     echo "$asset sha512: $(sha512sum "$asset" | cut -d' ' -f1)"
     echo "$asset sha256: $(sha256sum "$asset" | cut -d' ' -f1)"
   done
   asset="dengjen-tashkeel-capi-x86_64-pc-windows-msvc.zip"
   curl -sLO "https://github.com/ZirekHQ/dengjen-tashkeel/releases/download/v${version}/${asset}"
   echo "$asset sha512: $(sha512sum "$asset" | cut -d' ' -f1)"
   echo "$asset sha256: $(sha256sum "$asset" | cut -d' ' -f1)"
   ```
2. Paste the `sha512` values into the matching `CAPI_SHA512` entries in
   `ports/dengjen-tashkeel-capi/portfile.cmake`, and the `sha256` values into
   `_RELEASE_ASSETS` in `packaging/conan/conanfile.py`.
3. If the version being released differs from what's already recorded, bump
   it in `ports/dengjen-tashkeel-capi/vcpkg.json`, `portfile.cmake`'s
   `CAPI_VERSION`, and `conanfile.py`'s `version`.
4. Recompute the port's git-tree hash (vcpkg's version file must match the
   *exact* contents of `ports/dengjen-tashkeel-capi/` after the above edits)
   and update `versions/d-/dengjen-tashkeel-capi.json` and
   `versions/baseline.json`:
   ```bash
   git add ports/dengjen-tashkeel-capi
   git write-tree --prefix=ports/dengjen-tashkeel-capi/
   ```
5. Verify end-to-end before merging:
   ```bash
   # vcpkg, from a vcpkg checkout with this repo added as a registry
   vcpkg install dengjen-tashkeel-capi

   # Conan
   conan create packaging/conan --version=1.5.2
   ```

## Why not submit to the central registries?

Both [the vcpkg curated registry](https://github.com/microsoft/vcpkg) and
[ConanCenter](https://github.com/conan-io/conan-center-index) expect ports to
build from source; submitting a binary-only port there would need a
Rust-in-vcpkg / Rust-in-Conan build story this project doesn't have yet.
Instead:

- **vcpkg** consumers add this repository directly as a
  [custom git registry](https://learn.microsoft.com/en-us/vcpkg/consuming/git-registries)
  (see the README's "From vcpkg" section) — no server or approval needed,
  because vcpkg's git-registry protocol works against a bare git repo.
- **Conan** has no equivalent zero-infrastructure distribution path, so
  consumers run `conan create packaging/conan --version=<version>` locally to
  populate their own cache (see the README's "From Conan" section).
