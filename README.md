# dengjen-tashkeel

Arabic-text diacritic (tashkeel) restoration using an ONNX neural model,
trained mainly on MSA data from [Hareef](https://github.com/mush42/hareef).

Available as a Rust crate, a C ABI, a Python package, a Java library, and a standalone CLI.

## Non-scope

Diacritization only — not a general-purpose Arabic NLP toolkit.

## Known limitation

Accuracy is bounded by the underlying Hareef model: it can mis-diacritize
proper nouns and other words whose correct case ending depends on context
it wasn't trained to resolve. For example, given `عن أمير المؤمنين أبي حفص
عمر بن الخطاب`, the name `حفص` comes out as `حِفَصِ` ("hifsi") instead of
the grammatically correct `حَفْصٍ` ("hafsin"). See
[issue #28](https://github.com/ZirekHQ/dengjen-tashkeel/issues/28) for
the full discrepancy. This is a model-accuracy limitation, not a bug in
this library's code.

## Install

**Rust:**

```bash
cargo add dengjen-tashkeel
```

The default features (`ort-static`, `rayon`) statically link a bundled
ONNX Runtime, so this works with no extra setup.

**Python:**

```bash
pip install dengjen-tashkeel-py
```

Published to PyPI as `dengjen-tashkeel-py` by the `python-publish.yml` CI
workflow whenever a version tag is pushed — availability at any given
moment depends on whether a tag has been pushed since this was set up, so
if the install fails, build the wheel yourself instead (see **Building**
below). The wheel depends on `onnxruntime` (installed automatically by
`pip`); don't have `onnxruntime-gpu` installed alongside it — they share
the same import name.

**C:** download the prebuilt `dengjen-tashkeel-capi-<target>` archive for
your platform from [GitHub
Releases](https://github.com/ZirekHQ/dengjen-tashkeel/releases). Each
archive bundles the built shared library (`.so`/`.dylib`/`.dll`) together
with [`dengjen_tashkeel.h`](./crates/capi/dengjen_tashkeel.h) — link
against the library and include the header, no need to build from source.

The library and header are also available through a self-hosted [vcpkg
custom registry](https://learn.microsoft.com/en-us/vcpkg/consuming/git-registries)
— this repository itself, referenced directly by git, no separate server
needed. Add it to your project's `vcpkg-configuration.json`:

```json
{
  "default-registry": {
    "kind": "builtin",
    "baseline": "<current vcpkg builtin-registry baseline>"
  },
  "registries": [
    {
      "kind": "git",
      "repository": "https://github.com/ZirekHQ/dengjen-tashkeel",
      "baseline": "<commit-sha of the dengjen-tashkeel commit to pin>",
      "packages": ["dengjen-tashkeel-capi"]
    }
  ]
}
```

then add `dengjen-tashkeel-capi` to your `vcpkg.json` dependencies and run
`vcpkg install`.

[ConanCenter](https://github.com/conan-io/conan-center-index) requires
recipes to build from source, which this project's C API can't do
without a Rust toolchain in Conan's build environment, so this isn't
published there. Instead, clone this repository and export the recipe
into your local Conan cache:

```bash
conan create packaging/conan --version=1.5.2
```

then add `dengjen-tashkeel-capi/1.5.2` to your `conanfile.txt`/`conanfile.py`
`requires`. See [packaging/README.md](./packaging/README.md) for how both
are maintained.

**Java:**

```kotlin
implementation("io.github.zirekhq:dengjen-tashkeel:1.5.3")
runtimeOnly("io.github.zirekhq:dengjen-tashkeel:1.5.3:linux-x86_64")
```

Published to Maven Central by the `java-publish.yml` CI workflow whenever a
version tag is pushed. The binding uses the JDK Foreign Function & Memory
API (`java.lang.foreign`), not JNA, to load the native
`dengjen_tashkeel_capi` shared library. The native library ships as
per-platform classifier jars — `linux-x86_64`, `windows-x64`, and
`macos-aarch64` — so add a `runtimeOnly` dependency on the one matching
your platform alongside the main dependency; it's picked up automatically
at runtime. To use a library you built or placed yourself instead, set
`-Ddengjen.tashkeel.native.library.path=/path/to/library` (a file path, not
a directory — `java.library.path` is not consulted). Either way, add
`--enable-native-access=ALL-UNNAMED` to your own JVM invocation to avoid a
native-access warning.

**CLI:** either build from source (see **Building**) or `cargo install
dengjen-tashkeel-cli` (published to crates.io).

## Quick start

**Rust:**

```rust
use dengjen_tashkeel::{create_inference_engine, do_tashkeel};

// None = use the bundled model.
let engine = create_inference_engine(None)?;
let diacritized = do_tashkeel(&engine, "بسم الله الرحمن الرحيم", None, false)?;
```

`do_tashkeel`'s third argument is an optional taskeen threshold (see
below) and the fourth is `preprocessed` — pass `true` only if `text` is
already sentence-segmented, otherwise the library segments it for you.
Input is capped at `CHAR_LIMIT` (12,000 characters); longer input returns
`Err(DengjenTashkeelError::InputTooLong(_))`.

**Python:**

```python
from dengjen_tashkeel_py import tashkeel

tashkeel("بسم الله الرحمن الرحيم")
```

The Rust side logs via the `log` facade but installs no backend; if you
want its diagnostics (e.g. a redundant-init warning) surfaced through
Python's own `logging` module, install [`pyo3-log`](https://github.com/vorner/pyo3-log)
in your embedding process.

**C:** the API is a single entry point for diacritizing a UTF-8 encoded
string — see
[`ffi_usage_example.py`](./crates/capi/ffi_usage_example.py) for sample
usage against the compiled library via `ctypes`.

**Java:**

```java
import io.github.zirekhq.dengjen.tashkeel.Tashkeel;
import java.util.Optional;

Tashkeel tashkeel = Tashkeel.loadDefault();
String diacritized = tashkeel.diacritize("بسم الله الرحمن الرحيم", Optional.empty(), false);
```

`diacritize`'s second argument is an optional taskeen threshold (see
below) and the third is `preprocessed` — pass `true` only if the text is
already sentence-segmented, otherwise the library segments it for you.
Errors surface as a checked `TashkeelException`, whose `reason()` is an
exhaustively switchable sealed type mirroring the `ErrorCode` values in
`dengjen_tashkeel.h`.

**CLI:**

```bash
echo "بسم الله الرحمن الرحيم" > input.txt
dengjen-tashkeel -f input.txt
```

```text
Usage: dengjen-tashkeel [OPTIONS]

Options:
  -f, --input-file <INPUT_FILE>    Input file (default `stdin`)
  -o, --output-file <OUTPUT_FILE>  Output file (default `stdout`)
  -i, --interactive                Use interactive mode (useful for testing)
  -t, --taskeen                    Use sukoon for case-ending diacritic if the model is uncertain
  -p, --prob <PROB>                Taskeen threshold probability [default: 0.95]
  -x, --onnx <ONNX_MODEL>          ONNX model (default: use bundled model if available)
  -h, --help                       Print help
  -V, --version                    Print version
```

With neither `--input-file` nor `--output-file`, the CLI runs in
interactive mode by default.

## The taskeen option

When enabled (`taskeen_threshold` in Rust/Python, `--taskeen`/`--prob` in
the CLI), the model substitutes a sukoon for a case-ending diacritic it
isn't confident about, instead of guessing.

## Building

Requires [Rust](https://www.rust-lang.org/tools/install).

```bash
cargo build --release
```

Builds the `dengjen_tashkeel_capi` library and the `dengjen-tashkeel` CLI
under `target/`.

To build the Python wheel, install
[maturin](https://github.com/pyo3/maturin):

```bash
cd crates/python
python3 -m venv .venv
source .venv/bin/activate
pip install maturin
maturin build --release --strip
```

The wheel is written to `target/wheels/`.

## Credits

Created by [mush42](https://github.com/mush42) (Musharraf Omer).

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE), at your option.
