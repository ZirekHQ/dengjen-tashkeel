# Java Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Java library (`io.github.zirekhq:dengjen-tashkeel-java`) that wraps dengjen-tashkeel's existing C ABI via JNA, and a Maven Central publish pipeline for it.

**Architecture:** A standalone Gradle module at `bindings/java/`, outside the Cargo workspace. A package-private JNA `Library` interface and `Structure` mirror `dengjen_tashkeel.h` exactly; a public `Tashkeel` class wraps the three C entry points with UTF-8-safe marshalling and maps `ExternError` into a sealed `TashkeelException.Reason` hierarchy. No new Rust code, no new native artifact — the module consumes the same `dengjen-tashkeel-capi` cdylib the GitHub Release, vcpkg, and Conan already distribute.

**Tech Stack:** Java 17, Gradle (Kotlin DSL), JNA (`net.java.dev.jna:jna`), JUnit 5, `com.vanniktech.maven.publish` Gradle plugin.

**Spec:** [docs/superpowers/specs/2026-09-02-java-bindings-design.md](../specs/2026-09-02-java-bindings-design.md)

## Global Constraints

- Java baseline: 17 (LTS) — public API may use records, sealed interfaces, `Optional`.
- No nulls in the public API — use `Optional`/checked exceptions, per this project's JVM standards.
- Maven groupId: `io.github.zirekhq`. Artifact ID: `dengjen-tashkeel-java` (matches the existing `dengjen-tashkeel-<lang>` naming convention: `dengjen-tashkeel-capi`, `dengjen-tashkeel-python`, `dengjen-tashkeel-cli`).
- Module lives at `bindings/java/`, outside the Cargo workspace (`Cargo.toml`'s `[workspace] members` is not touched).
- Binding technology is JNA only — no JNI, no new native build/release matrix. The module depends on the `dengjen-tashkeel-capi` cdylib that already ships in GitHub Releases (issue #23) and via vcpkg/Conan (issue #24).
- Native library delivery for this pass: the caller supplies the shared library on `jna.library.path` or `java.library.path` — no bundled per-OS classifier jars (documented as future work).
- Build tool is Gradle (Kotlin DSL); Maven Central publishing goes through the `com.vanniktech.maven.publish` plugin (handles Central Portal upload, GPG signing, and POM generation in one step).
- No `Co-Authored-By`, `Claude-Session`, "Generated with Claude Code", or any other AI-attribution trailer in any commit message, PR title, or PR body produced while executing this plan.
- Commit messages follow this repo's observed convention: `<type>[(<scope>)]: <lowercase imperative subject>` (e.g. `feat(java): add Tashkeel diacritization API`).

### Deviations from the spec's illustrative sketch (resolved during planning)

The spec sketched `TashkeelException` as a `sealed interface` directly implemented by the error records. That doesn't compile: a Java `record` implicitly extends `java.lang.Record`, so it cannot also extend `Exception` — and only a `Throwable` can be thrown. This plan instead makes `TashkeelException` a single concrete class extending `Exception`, carrying a sealed `Reason` value (see Task 2) that callers `switch` over exhaustively. The public throw/catch surface and the exhaustive-switch ergonomics the spec wanted are both preserved.

The spec's publish step read `./gradlew :bindings:java:publish`. Since `bindings/java/` is its own standalone Gradle build (its own `settings.gradle.kts`, not a subproject of a root multi-project build — this repo's root has no Gradle build at all), the correct invocation is `./gradlew publish` run with `bindings/java` as the working directory (see Task 5).

### Post-review revision: JReleaser, not `com.vanniktech.maven.publish`

After the final whole-branch review and its fix wave, this plan's Task 1 and
Task 5 were superseded on explicit direction: publishing now goes through
the `org.jreleaser` Gradle plugin instead of `com.vanniktech.maven.publish`,
mirroring the sibling `ZirekHQ/dengjen-tts` repo's Java module, whose
publish pipeline this project's maintainer had already built out and
proven. `build.gradle.kts` uses Gradle's own `maven-publish` plugin to
stage the jar/sources/javadoc/POM into a local `build/staging-deploy`
Maven repo, then JReleaser's `deploy.maven.mavenCentral` block signs and
uploads that staged repo to Central Portal — CI runs `./gradlew publish
jreleaserDeploy` (deploy-only, not `jreleaserFullRelease`, since GitHub
releases for this project are already handled by the cargo-dist
`release.yml` pipeline on the same tag). Required secrets are renamed
accordingly to JReleaser's own convention: `JRELEASER_MAVENCENTRAL_USERNAME`
/ `JRELEASER_MAVENCENTRAL_PASSWORD` / `JRELEASER_GPG_PASSPHRASE` /
`JRELEASER_GPG_SECRET_KEY` / `JRELEASER_GPG_PUBLIC_KEY`, superseding
`MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` / `GPG_PRIVATE_KEY` /
`GPG_PASSPHRASE`.

The same direction also mirrored `dengjen-tts`'s general Gradle tooling:
the wrapper moved to Gradle 9.7.1, `settings.gradle.kts` gained the
`org.gradle.toolchains.foojay-resolver-convention` plugin, and plugin/library
versions moved into `gradle/libs.versions.toml`. The Java 17 toolchain
baseline was kept as-is (an explicit decision) even though `dengjen-tts`
targets Java 25 — mirroring covered tooling, not the language-level
compatibility goal this plan's Java-17 decision protects. Separately, the
Artifact ID was renamed from `dengjen-tashkeel-java` to `dengjen-tashkeel`
(also explicit direction), set via an explicit `artifactId` override in the
`maven-publish` publication rather than by renaming the Gradle module
itself.

---

### Task 1: Scaffold the Gradle module

**Files:**
- Create: `bindings/java/settings.gradle.kts`
- Create: `bindings/java/build.gradle.kts`
- Create: `bindings/java/.gitignore`
- Create: `bindings/java/gradlew`, `bindings/java/gradlew.bat`, `bindings/java/gradle/wrapper/gradle-wrapper.properties`, `bindings/java/gradle/wrapper/gradle-wrapper.jar` (generated, not hand-written)

**Interfaces:**
- Produces: a Gradle build with `group = "io.github.zirekhq"`, Maven coordinates `io.github.zirekhq:dengjen-tashkeel-java`, Java 17 toolchain, `src/main/java/io/github/zirekhq/dengjentashkeel/` and `src/test/java/io/github/zirekhq/dengjentashkeel/` source roots, JNA and JUnit 5 on the classpath, and a `mavenPublishing { ... }` block configured for Central Portal + in-memory GPG signing. Later tasks add source files under those roots; nothing else in this task is consumed elsewhere.

This task has no application logic, so there's no red/green test cycle — the deliverable is verified by running Gradle itself.

- [ ] **Step 1: Create the module directory and `.gitignore`**

```bash
mkdir -p bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel
mkdir -p bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel
```

`bindings/java/.gitignore`:

```gitignore
.gradle/
build/
out/
*.iml
.idea/
!gradle/wrapper/gradle-wrapper.jar
```

- [ ] **Step 2: Write `settings.gradle.kts`**

```kotlin
rootProject.name = "dengjen-tashkeel-java"
```

- [ ] **Step 3: Write `build.gradle.kts`**

```kotlin
plugins {
    java
    id("com.vanniktech.maven.publish") version "0.30.0"
}

group = "io.github.zirekhq"
// Kept in sync by hand with [workspace.package].version in the repo root's
// Cargo.toml -- single source of truth is that file, this just mirrors it.
version = "1.5.2"

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
    withSourcesJar()
    withJavadocJar()
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.15.0")

    testImplementation(platform("org.junit:junit-bom:5.11.0"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    // Points JNA at the debug cdylib built by `cargo build -p
    // dengjen-tashkeel-capi` (repo root) so tests exercise the real FFI
    // boundary without needing a published release archive. Override with
    // -Pjna.library.path=/some/dir for a different build.
    val nativeDir = (project.findProperty("jna.library.path") as String?)
        ?: "${rootDir}/../../target/debug"
    systemProperty("jna.library.path", nativeDir)
}

mavenPublishing {
    publishToMavenCentral()
    signAllPublications()

    coordinates(group.toString(), "dengjen-tashkeel-java", version.toString())

    pom {
        name.set("dengjen-tashkeel-java")
        description.set("Java bindings for dengjen-tashkeel: Arabic-text diacritic restoration using neural networks")
        url.set("https://github.com/ZirekHQ/dengjen-tashkeel")
        licenses {
            license {
                name.set("MIT")
                url.set("https://opensource.org/licenses/MIT")
            }
            license {
                name.set("Apache-2.0")
                url.set("https://www.apache.org/licenses/LICENSE-2.0")
            }
        }
        developers {
            developer {
                name.set("Musharraf Omer")
                email.set("ibnomer2011@hotmail.com")
            }
        }
        scm {
            url.set("https://github.com/ZirekHQ/dengjen-tashkeel")
            connection.set("scm:git:https://github.com/ZirekHQ/dengjen-tashkeel.git")
        }
    }
}
```

- [ ] **Step 4: Generate the Gradle wrapper**

No system-wide `gradle` is assumed to be installed. Install one temporarily via SDKMAN to bootstrap the wrapper, then rely on the wrapper from here on:

```bash
sdk install gradle 8.11 || sdk use gradle 8.11
cd bindings/java
gradle wrapper --gradle-version 8.11 --distribution-type bin
cd -
```

This produces `gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.properties`, and `gradle/wrapper/gradle-wrapper.jar` under `bindings/java/`.

- [ ] **Step 5: Verify the build works**

```bash
cd bindings/java
chmod +x gradlew
./gradlew --version
./gradlew build
cd -
```

Expected: `./gradlew --version` prints a Gradle 8.11 banner; `./gradlew build` succeeds (there's no source yet, so this only proves the toolchain, dependency resolution, and plugin application all work).

- [ ] **Step 6: Commit**

```bash
git add bindings/java
git commit -m "feat(java): scaffold Gradle module for Java bindings"
```

---

### Task 2: `TashkeelException`

**Files:**
- Create: `bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/TashkeelException.java`
- Test: `bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel/TashkeelExceptionTest.java`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: `public final class TashkeelException extends Exception`, constructor `TashkeelException(Reason reason)`, method `Reason reason()`. Nested `public sealed interface Reason permits InputTooLong, InferenceError, ModelLoadError, Unknown { String message(); }` with records `InputTooLong(String message)`, `InferenceError(String message)`, `ModelLoadError(String message)`, `Unknown(int code, String message)`. Task 3 throws this type and constructs these records from `ExternError`.

- [ ] **Step 1: Write the failing test**

`bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel/TashkeelExceptionTest.java`:

```java
package io.github.zirekhq.dengjentashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import org.junit.jupiter.api.Test;

class TashkeelExceptionTest {

    @Test
    void messageComesFromTheReason() {
        var exception = new TashkeelException(new TashkeelException.InferenceError("boom"));

        assertEquals("boom", exception.getMessage());
        assertEquals("boom", exception.reason().message());
    }

    @Test
    void reasonIsExhaustivelySwitchable() {
        TashkeelException.Reason reason = new TashkeelException.Unknown(-1, "panic");

        String described = switch (reason) {
            case TashkeelException.InputTooLong r -> "too long: " + r.message();
            case TashkeelException.InferenceError r -> "inference: " + r.message();
            case TashkeelException.ModelLoadError r -> "model: " + r.message();
            case TashkeelException.Unknown r -> "unknown(" + r.code() + "): " + r.message();
        };

        assertEquals("unknown(-1): panic", described);
        assertInstanceOf(TashkeelException.Unknown.class, reason);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cd bindings/java && ./gradlew test --tests "*.TashkeelExceptionTest" ; cd -
```

Expected: FAIL — `TashkeelException` does not exist (compile error).

- [ ] **Step 3: Write the implementation**

`bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/TashkeelException.java`:

```java
package io.github.zirekhq.dengjentashkeel;

/**
 * Thrown when the native dengjen-tashkeel engine reports an error.
 * {@link #reason()} carries the specific failure as a sealed,
 * exhaustively switchable value mirroring the Rust
 * {@code DengjenTashkeelError} enum and the {@code ErrorCode} constants
 * in {@code dengjen_tashkeel.h}.
 */
public final class TashkeelException extends Exception {

    private final Reason reason;

    public TashkeelException(Reason reason) {
        super(reason.message());
        this.reason = reason;
    }

    public Reason reason() {
        return reason;
    }

    public sealed interface Reason permits InputTooLong, InferenceError, ModelLoadError, Unknown {
        String message();
    }

    /** ErrorCode 1: input exceeds {@code dengjen_tashkeel::CHAR_LIMIT} characters. */
    public record InputTooLong(String message) implements Reason {
    }

    /** ErrorCode 2: the engine failed during inference (includes a bad or malformed model path/file). */
    public record InferenceError(String message) implements Reason {
    }

    /** ErrorCode 3: the model file failed to load. */
    public record ModelLoadError(String message) implements Reason {
    }

    /** Any other ErrorCode, including PANIC (-1) and INVALID_HANDLE (-1000). */
    public record Unknown(int code, String message) implements Reason {
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cd bindings/java && ./gradlew test --tests "*.TashkeelExceptionTest" ; cd -
```

Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/TashkeelException.java \
        bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel/TashkeelExceptionTest.java
git commit -m "feat(java): add TashkeelException error hierarchy"
```

---

### Task 3: JNA bindings and the `Tashkeel` API

**Files:**
- Create: `bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/ExternError.java`
- Create: `bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/NativeLibrary.java`
- Create: `bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/Tashkeel.java`
- Test: `bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel/TashkeelTest.java`

**Interfaces:**
- Consumes: `TashkeelException` and its `Reason` records from Task 2 (exact signatures above).
- Produces: `public final class Tashkeel implements AutoCloseable` with `static Tashkeel load(Path modelPath) throws TashkeelException`, `static Tashkeel loadDefault() throws TashkeelException`, `String diacritize(String text, Optional<Float> taskeenThreshold, boolean preprocessed) throws TashkeelException`, `void close()`, and a package-private no-arg constructor (used by tests to get a handle without consuming the one-shot native init slot — see the class Javadoc below). `ExternError` and `NativeLibrary` are package-private implementation details; no other task references them directly.

This task requires the native `dengjen_tashkeel_capi` shared library to exist before its tests can run.

- [ ] **Step 1: Build the native library the tests will link against**

```bash
cargo build -p dengjen-tashkeel-capi
```

Run from the repo root. This produces `target/debug/libdengjen_tashkeel_capi.so` (or `.dylib`/`.dll`), which `build.gradle.kts`'s `tasks.test` block (Task 1, Step 3) already points `jna.library.path` at by default.

- [ ] **Step 2: Write the failing tests**

`bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel/TashkeelTest.java`:

```java
package io.github.zirekhq.dengjentashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Optional;
import org.junit.jupiter.api.Test;

/**
 * The native engine lives in a single process-wide slot (see {@link
 * Tashkeel}'s class Javadoc), so -- like dengjen-tashkeel's own capi tests
 * -- these are written to be correct regardless of execution order: each
 * either avoids touching the global engine, or warms it up itself via the
 * idempotent lazy-init path before asserting on "already initialized".
 */
class TashkeelTest {

    // Must match dengjen_tashkeel::CHAR_LIMIT (crates/core/src/lib.rs).
    private static final int CHAR_LIMIT = 12000;

    @Test
    void diacritizeLazilyInitializesTheDefaultEngineAndChangesTheText() throws Exception {
        Tashkeel tashkeel = new Tashkeel();

        String result = tashkeel.diacritize("بسم الله الرحمن الرحيم", Optional.empty(), true);

        assertNotEquals("بسم الله الرحمن الرحيم", result);
    }

    @Test
    void diacritizeOverCharLimitThrowsInputTooLong() {
        Tashkeel tashkeel = new Tashkeel();
        String tooLong = "ا".repeat(CHAR_LIMIT + 1);

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> tashkeel.diacritize(tooLong, Optional.empty(), true));

        assertInstanceOf(TashkeelException.InputTooLong.class, exception.reason());
    }

    @Test
    void loadWithNonexistentModelPathThrowsInferenceError() {
        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.load(Path.of("/nonexistent/path/to/model.onnx")));

        assertInstanceOf(TashkeelException.InferenceError.class, exception.reason());
    }

    @Test
    void loadWithMalformedModelFileThrowsInferenceError() throws IOException {
        Path malformed = Files.createTempFile("dengjen-tashkeel-test", ".onnx");
        Files.writeString(malformed, "this is not a valid onnx model");

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.load(malformed));

        assertInstanceOf(TashkeelException.InferenceError.class, exception.reason());
    }

    @Test
    void loadAfterTheEngineIsAlreadyInitializedThrowsUnknown() throws Exception {
        // Warm-up: idempotent regardless of whether an earlier test already
        // initialized the global engine.
        new Tashkeel().diacritize("بسم الله", Optional.empty(), true);

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.load(Path.of("/nonexistent/path/to/model.onnx")));

        assertInstanceOf(TashkeelException.Unknown.class, exception.reason());
        assertEquals(99, ((TashkeelException.Unknown) exception.reason()).code());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

```bash
cd bindings/java && ./gradlew test --tests "*.TashkeelTest" ; cd -
```

Expected: FAIL — `Tashkeel` does not exist (compile error).

- [ ] **Step 4: Write `ExternError.java`**

```java
package io.github.zirekhq.dengjentashkeel;

import com.sun.jna.Pointer;
import com.sun.jna.Structure;

/**
 * Mirrors {@code struct ExternError { ErrorCode code; char *message; }}
 * from {@code dengjen_tashkeel.h}. Field order matters -- it must match
 * the C layout exactly (code first, then message).
 */
@Structure.FieldOrder({"code", "message"})
class ExternError extends Structure {
    public int code;
    public Pointer message;

    static class ByReference extends ExternError implements Structure.ByReference {
    }
}
```

- [ ] **Step 5: Write `NativeLibrary.java`**

```java
package io.github.zirekhq.dengjentashkeel;

import com.sun.jna.Library;
import com.sun.jna.Pointer;

/**
 * Raw JNA mapping of {@code dengjen_tashkeel.h}. Package-private --
 * callers use {@link Tashkeel}, not this interface, directly.
 *
 * <p>Strings cross this boundary as manually UTF-8-encoded {@link
 * Pointer}s (see {@link Tashkeel}), not JNA's default String marshalling
 * -- JNA's default native string encoding follows the JVM's platform
 * charset, which corrupts Arabic text on platforms that don't default to
 * UTF-8.
 */
interface NativeLibrary extends Library {

    Pointer dengjenTashkeelTashkeel(
            Pointer textPtr, Pointer taskeenThresholdPtr, boolean preprocessed, ExternError.ByReference outError);

    void dengjen_tashkeel_init(Pointer modelPathPtr, ExternError.ByReference outError);

    void dengjen_tashkeel_free_string(Pointer s);
}
```

- [ ] **Step 6: Write `Tashkeel.java`**

```java
package io.github.zirekhq.dengjentashkeel;

import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.Optional;

/**
 * Diacritizes Arabic text via dengjen-tashkeel's native inference engine.
 *
 * <p>The underlying native library holds its inference engine in a single
 * process-wide slot: the first successful {@link #load} or {@link
 * #loadDefault} call -- or the first {@link #diacritize} call on an
 * uninitialized instance, which lazily loads the default model -- wins
 * for the life of the JVM process. A later {@code load}/{@code
 * loadDefault} call throws a {@link TashkeelException.Unknown} rather
 * than replacing it. Every {@code Tashkeel} instance is a stateless
 * handle onto that same global engine; constructing more than one has no
 * effect beyond the first successful initialization.
 *
 * <p>Requires the native {@code dengjen_tashkeel_capi} shared library on
 * {@code jna.library.path} or {@code java.library.path}. See the
 * repository README for how to obtain it.
 */
public final class Tashkeel implements AutoCloseable {

    private static final int SUCCESS = 0;
    private static final int INPUT_TOO_LONG = 1;
    private static final int INFERENCE_ERROR = 2;
    private static final int MODEL_LOAD_ERROR = 3;

    private static final NativeLibrary LIB = Native.load("dengjen_tashkeel_capi", NativeLibrary.class);

    // Package-private: tests construct directly to avoid consuming the
    // one-shot global-init slot that load()/loadDefault() would. External
    // callers only ever see the static factories.
    Tashkeel() {
    }

    /** Initializes the native engine with a specific ONNX model file. */
    public static Tashkeel load(Path modelPath) throws TashkeelException {
        ExternError.ByReference outError = new ExternError.ByReference();
        LIB.dengjen_tashkeel_init(toNativeUtf8(modelPath.toString()), outError);
        checkError(outError);
        return new Tashkeel();
    }

    /** Initializes the native engine with its bundled default model. */
    public static Tashkeel loadDefault() throws TashkeelException {
        ExternError.ByReference outError = new ExternError.ByReference();
        LIB.dengjen_tashkeel_init(null, outError);
        checkError(outError);
        return new Tashkeel();
    }

    /**
     * Diacritizes {@code text}. Lazily initializes the default engine if
     * nothing has initialized it yet.
     *
     * @param taskeenThreshold confidence threshold for the taskeen
     *     (sukoon) diacritic; {@link Optional#empty()} uses the engine's
     *     default
     * @param preprocessed whether {@code text} has already been run
     *     through the library's Arabic text normalization
     */
    public String diacritize(String text, Optional<Float> taskeenThreshold, boolean preprocessed)
            throws TashkeelException {
        ExternError.ByReference outError = new ExternError.ByReference();
        Pointer textPtr = toNativeUtf8(text);
        Pointer thresholdPtr = taskeenThreshold.map(Tashkeel::toNativeFloat).orElse(null);

        Pointer resultPtr = LIB.dengjenTashkeelTashkeel(textPtr, thresholdPtr, preprocessed, outError);
        checkError(outError);
        try {
            return resultPtr.getString(0, "UTF-8");
        } finally {
            LIB.dengjen_tashkeel_free_string(resultPtr);
        }
    }

    @Override
    public void close() {
        // The C ABI has no per-call teardown -- the native engine lives
        // for the process's lifetime. Present for API symmetry and
        // try-with-resources.
    }

    private static void checkError(ExternError outError) throws TashkeelException {
        if (outError.code == SUCCESS) {
            return;
        }
        String message = outError.message == null ? "" : outError.message.getString(0, "UTF-8");
        LIB.dengjen_tashkeel_free_string(outError.message);
        throw new TashkeelException(switch (outError.code) {
            case INPUT_TOO_LONG -> new TashkeelException.InputTooLong(message);
            case INFERENCE_ERROR -> new TashkeelException.InferenceError(message);
            case MODEL_LOAD_ERROR -> new TashkeelException.ModelLoadError(message);
            default -> new TashkeelException.Unknown(outError.code, message);
        });
    }

    private static Pointer toNativeUtf8(String s) {
        byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        Memory memory = new Memory(bytes.length + 1L);
        memory.write(0, bytes, 0, bytes.length);
        memory.setByte(bytes.length, (byte) 0);
        return memory;
    }

    private static Pointer toNativeFloat(float value) {
        Memory memory = new Memory(Float.BYTES);
        memory.setFloat(0, value);
        return memory;
    }
}
```

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cd bindings/java && ./gradlew test --tests "*.TashkeelTest" ; cd -
```

Expected: PASS (5 tests). If `UnsatisfiedLinkError` is thrown, re-check Step 1 (`cargo build -p dengjen-tashkeel-capi` from the repo root) and that `target/debug/` contains the built shared library.

- [ ] **Step 8: Run the full test suite**

```bash
cd bindings/java && ./gradlew test ; cd -
```

Expected: PASS (7 tests total: 2 from `TashkeelExceptionTest`, 5 from `TashkeelTest`).

- [ ] **Step 9: Commit**

```bash
git add bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/ExternError.java \
        bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/NativeLibrary.java \
        bindings/java/src/main/java/io/github/zirekhq/dengjentashkeel/Tashkeel.java \
        bindings/java/src/test/java/io/github/zirekhq/dengjentashkeel/TashkeelTest.java
git commit -m "feat(java): add JNA bindings and the Tashkeel diacritization API"
```

---

### Task 4: Document the Java bindings in the root README

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: `Tashkeel.diacritize`/`load`/`loadDefault` signatures from Task 3 (for the Quick Start snippet) and the Maven coordinates from Task 1 (`io.github.zirekhq:dengjen-tashkeel-java`).
- Produces: nothing consumed by later tasks.

No automated test — this is a documentation-only deliverable, verified by reading the rendered section against the existing C/Python sections' style.

- [ ] **Step 1: Update the summary line**

`README.md:6`, change:

```markdown
Available as a Rust crate, a C ABI, a Python package, and a standalone CLI.
```

to:

```markdown
Available as a Rust crate, a C ABI, a Python package, a Java library, and a standalone CLI.
```

- [ ] **Step 2: Add a Java block to the `## Install` section**

Insert after the **C:** block (which ends with the `conan create packaging/conan --version=1.5.2` paragraph) and before the **CLI:** block:

```markdown
**Java:**

```kotlin
implementation("io.github.zirekhq:dengjen-tashkeel-java:1.5.2")
```

Published to Maven Central by the `java-publish.yml` CI workflow whenever a
version tag is pushed. The binding loads the native `dengjen_tashkeel_capi`
shared library via [JNA](https://github.com/java-native-access/jna) at
runtime rather than bundling it — download the `dengjen-tashkeel-capi-<target>`
archive for your platform (see the **C** section above), then either place
the shared library where your OS's default native-library search finds it,
or pass `-Djna.library.path=/path/to/dir` on the JVM command line.
```

- [ ] **Step 3: Add a Java block to the `## Quick start` section**

Insert after the **C:** block and before the **CLI:** block:

```markdown
**Java:**

```java
import io.github.zirekhq.dengjentashkeel.Tashkeel;
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
```

- [ ] **Step 4: Verify the section renders sensibly**

```bash
grep -n "Java" README.md
```

Expected: the summary line and both new blocks show up, each fenced code block closed correctly (no stray unclosed ``` from the nested kotlin/java fences inside the markdown insert — check the rendered file, not just the diff, since the snippets above are themselves fenced and easy to mis-nest when pasted into the surrounding ```markdown fence).

- [ ] **Step 5: Commit**

```bash
git add README.md
git commit -m "docs: document the Java bindings in the root README"
```

---

### Task 5: Maven Central publish workflow

**Files:**
- Create: `.github/workflows/java-publish.yml`

**Interfaces:**
- Consumes: `bindings/java/build.gradle.kts`'s `version` field and `mavenPublishing` block from Task 1 (this workflow just invokes `./gradlew publish`; it doesn't duplicate that configuration).
- Produces: nothing consumed by later tasks.

No automated test in the usual sense — the deliverable is verified by YAML validity and by mirroring the already-working `python-publish.yml` pattern. Actually publishing requires repository secrets (`MAVEN_CENTRAL_USERNAME`, `MAVEN_CENTRAL_PASSWORD`, `GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`) that only a maintainer can provision via a Sonatype Central Portal account and a registered GPG key — outside what this task can do; the workflow is correct and ready the moment those secrets exist.

- [ ] **Step 1: Resolve pinned commit SHAs for the two new third-party actions**

This repo pins every action to a commit SHA with the human-readable version as a trailing comment (see `.github/workflows/python-publish.yml`). Resolve the current latest tags for `actions/setup-java` and `gradle/actions` and their SHAs:

```bash
setup_java_tag="$(gh api repos/actions/setup-java/tags --jq '.[0].name')"
setup_java_sha="$(gh api repos/actions/setup-java/git/refs/tags/${setup_java_tag} --jq '.object.sha')"
echo "setup-java: ${setup_java_tag} ${setup_java_sha}"

setup_gradle_tag="$(gh api repos/gradle/actions/tags --jq '[.[] | select(.name | test("^v[0-9]+$"))][0].name')"
setup_gradle_sha="$(gh api repos/gradle/actions/git/refs/tags/${setup_gradle_tag} --jq '.object.sha')"
echo "setup-gradle: ${setup_gradle_tag} ${setup_gradle_sha}"
```

- [ ] **Step 2: Write the workflow**

`.github/workflows/java-publish.yml`, substituting the four `${...}` placeholders below with the values Step 1 printed (they are shell-style only to mark what to substitute — the committed file must contain literal values, not `${...}` syntax):

```yaml
name: Publish Java bindings

# Publishes the bindings/java Gradle module to Maven Central on the same
# version-tag push that triggers release.yml (Rust) and python-publish.yml
# (Python), so a Maven Central release always lines up with the others.
on:
  push:
    tags:
      - '**[0-9]+.[0-9]+.[0-9]+*'

permissions: {}

jobs:
  check-version:
    name: Verify tag matches Gradle module version
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          persist-credentials: false

      - name: Compare tag to bindings/java/build.gradle.kts version
        run: |
          set -euo pipefail
          tag_version="$(echo "${GITHUB_REF_NAME}" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+.*$')"
          gradle_version="$(grep -m1 '^version = ' bindings/java/build.gradle.kts | sed -E 's/version = "(.*)"/\1/')"
          if [ "${tag_version}" != "${gradle_version}" ]; then
            echo "::error::Tag ${GITHUB_REF_NAME} (version ${tag_version}) does not match bindings/java/build.gradle.kts version ${gradle_version}"
            exit 1
          fi

  publish:
    name: Publish to Maven Central
    needs: check-version
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          persist-credentials: false

      - uses: actions/setup-java@${setup_java_sha} # ${setup_java_tag}
        with:
          distribution: temurin
          java-version: "17"

      - uses: gradle/actions/setup-gradle@${setup_gradle_sha} # ${setup_gradle_tag}

      - name: Publish
        working-directory: bindings/java
        env:
          ORG_GRADLE_PROJECT_mavenCentralUsername: ${{ secrets.MAVEN_CENTRAL_USERNAME }}
          ORG_GRADLE_PROJECT_mavenCentralPassword: ${{ secrets.MAVEN_CENTRAL_PASSWORD }}
          ORG_GRADLE_PROJECT_signingInMemoryKey: ${{ secrets.GPG_PRIVATE_KEY }}
          ORG_GRADLE_PROJECT_signingInMemoryKeyPassword: ${{ secrets.GPG_PASSPHRASE }}
        run: ./gradlew publish
```

- [ ] **Step 3: Validate the YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/java-publish.yml'))" && echo OK
```

Expected: `OK`. If `actionlint` is available, also run `actionlint .github/workflows/java-publish.yml`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/java-publish.yml
git commit -m "ci: publish Java bindings to Maven Central on release tags"
```

---

## After this plan

Repository secrets `MAVEN_CENTRAL_USERNAME`, `MAVEN_CENTRAL_PASSWORD`, `GPG_PRIVATE_KEY`, and `GPG_PASSPHRASE` must be added by a maintainer (Sonatype Central Portal account + registered GPG key — see the spec's Publishing pipeline section) before `java-publish.yml` can complete a real publish. Until then, tags will fail at the `publish` job with an authentication error, which is the correct, loud failure mode rather than a silent no-op.
