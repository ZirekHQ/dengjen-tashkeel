# Java bindings for dengjen-tashkeel

Issue: [#25](https://github.com/ZirekHQ/dengjen-tashkeel/issues/25)

## Context

`dengjen-tashkeel` already exposes a stable C ABI (`crates/capi`,
`dengjen_tashkeel.h`): `dengjenTashkeelTashkeel`, `dengjen_tashkeel_init`,
`dengjen_tashkeel_free_string`. Since issue #23, `cargo-dist` packages the
built cdylib together with the header into a `dengjen-tashkeel-capi-<target>`
archive on every GitHub Release (not yet cut into a tagged release as of this
writing, but code-complete on `main`). Issue #24 set the precedent this
follows: vcpkg and Conan both ship binary-only packages that download that
same archive rather than building Rust from source. Java bindings take the
same shape — no new Rust logic, a thin consumer of the existing C ABI and
release artifact.

## Goals

- A Java library, publishable to Maven Central, that wraps the diacritization
  entry point with an idiomatic, type-safe API.
- Reuse the existing capi release artifact — no new native build/release
  matrix.
- A working Maven Central publish pipeline (signing, POM metadata, CI).

## Non-goals (this pass)

- Bundling native libraries inside the Java artifact (classifier jars per
  OS/arch). Documented as future work.
- A JNI alternative implementation.
- Kotlin/Android-specific packaging.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Binding technology | JNA (`com.sun.jna`) over JNI | Dynamically loads the existing cdylib; no new native shim to write, cross-compile, and ship per platform. JNI would duplicate capi's release matrix for a second native artifact. |
| Native library delivery (v1) | User supplies it on `java.library.path` / `jna.library.path` | Matches the issue's own framing and the vcpkg/Conan precedent: download the `dengjen-tashkeel-capi-<target>` archive (or install via vcpkg/Conan) and point JNA at it. No per-OS Java artifacts to build or publish. |
| Maven namespace | `io.github.zirekhq` | Central Portal auto-verifies `io.github.<org>` against a public GitHub org the publisher admins — no domain/DNS TXT record needed. |
| Artifact ID | `dengjen-tashkeel-java` | Matches the existing `dengjen-tashkeel-<lang>` convention (`dengjen-tashkeel-capi`, `dengjen-tashkeel-python`, `dengjen-tashkeel-cli`). |
| Java baseline | 17 (LTS) | Matches the workplace JVM standard of using records, sealed interfaces, `Optional`/`Stream` in the public API, without excluding most current users. |
| Build tool | Gradle (Kotlin DSL) | Issue names Gradle first; `com.vanniktech.maven.publish` plugin handles Central Portal upload, GPG signing, and POM generation in one step. |
| Location | `bindings/java/` at repo root, outside the Cargo workspace | Pure Java, no Rust — keeps it out of `cargo build --workspace`, Sonar Rust scans, and `deny.toml`. |

## Public API

```java
package io.github.zirekhq.dengjentashkeel;

public final class Tashkeel implements AutoCloseable {
    public static Tashkeel load(Path modelPath) throws TashkeelException;
    public static Tashkeel loadDefault() throws TashkeelException;

    public String diacritize(String text, OptionalDouble taskeenThreshold, boolean preprocessed)
        throws TashkeelException;

    @Override
    public void close(); // no-op: the C ABI has no per-instance teardown; documented as such
}

public sealed interface TashkeelException permits
    TashkeelException.InputTooLong,
    TashkeelException.InferenceError,
    TashkeelException.ModelLoadError,
    TashkeelException.Unknown {
    record InputTooLong(int maxLength) implements TashkeelException { }
    record InferenceError(String message) implements TashkeelException { }
    record ModelLoadError(String message) implements TashkeelException { }
    record Unknown(int code, String message) implements TashkeelException { }
}
```

`TashkeelException` variants map 1:1 to the `ErrorCode` constants in
`dengjen_tashkeel.h` (`INPUT_TOO_LONG` = 1, `INFERENCE_ERROR` = 2,
`MODEL_LOAD_ERROR` = 3, `UNKNOWN_ERROR` = 99), mirroring
`DengjenTashkeelError` on the Rust side.

`Tashkeel.load`/`loadDefault` call `dengjen_tashkeel_init` once; the JNA
interface method for the diacritization call frees the returned `char*` via
`dengjen_tashkeel_free_string` before returning the Java `String`, so callers
never see a native pointer.

## Native library resolution

JNA's default search order is used unmodified: `jna.library.path` system
property, then `java.library.path`, then OS-standard locations
(`LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH` on macOS, `PATH` on
Windows). No custom resource-extraction loader.

The module README documents:
1. Download `dengjen-tashkeel-capi-<target>` from the
   [Releases page](https://github.com/ZirekHQ/dengjen-tashkeel/releases), or
   install via vcpkg/Conan per the existing root README instructions.
2. Either place the shared library where the OS's default native-library
   search finds it, or pass `-Djna.library.path=/path/to/dir` on the JVM
   command line.

## Publishing pipeline

New workflow `.github/workflows/java-publish.yml`, triggered the same way as
`python-publish.yml` (on release tag push). Steps:

1. `./gradlew :bindings:java:publish` using the
   `com.vanniktech.maven.publish` Gradle plugin, which handles Central
   Portal upload, GPG signing, and POM generation.
2. POM metadata (license, repository URL, description) sourced from the same
   values as `[workspace.package]` in the root `Cargo.toml`, kept in sync by
   hand (no automated cross-sync — low churn, single source of truth noted
   in a comment).

Required repository secrets (added manually by a maintainer before this
pipeline can run for real — outside the scope of what a coding agent can
provision):
- `MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` — a Central Portal
  publishing token, not a personal login.
- `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE` — the signing key registered with
  Central Portal.

## Testing

JUnit 5. CI builds the capi cdylib first (`cargo build -p
dengjen-tashkeel-capi`) and points `jna.library.path` at `target/debug`
before running the Gradle test task — exercising the real FFI boundary from
the Java side, the same way `crates/capi`'s own tests exercise it from Rust.

## Open follow-ups (not this issue)

- Classifier jars bundling prebuilt natives per OS/arch, for a zero-setup
  experience.
- JNI implementation, if idiomatic native-method packaging is ever required.
