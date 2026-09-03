package io.github.zirekhq.dengjen.tashkeel;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.UncheckedIOException;
import java.lang.foreign.Arena;
import java.lang.foreign.SymbolLookup;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.nio.file.attribute.FileAttribute;
import java.nio.file.attribute.PosixFilePermissions;

/**
 * Resolves the native {@code dengjen_tashkeel_capi} shared library for
 * {@link TashkeelLib}. Two paths:
 *
 * <ul>
 *   <li>{@code -Ddengjen.tashkeel.native.library.path=<file>} -- an
 *       explicit override, used by this module's own test suites (see
 *       {@code build.gradle.kts}) and available to any consumer who wants
 *       to point at a native library they built or placed themselves.
 *   <li>Otherwise: detect the running platform (see {@link
 *       NativePlatform}) and look for that platform's native library as a
 *       classpath resource under {@code natives/<classifier>/}, which is
 *       exactly what this project's per-platform classifier jars contain.
 *       The resource is copied to a temp file -- {@code
 *       SymbolLookup.libraryLookup} needs a real filesystem path, not an
 *       in-jar one.
 * </ul>
 *
 * <p>Plain {@code java.library.path} is not consulted here: one of the
 * two paths above always either resolves or throws. JNA transparently
 * fell back to {@code java.library.path} before this migration, and
 * FFM's {@code SymbolLookup.libraryLookup(String, Arena)} overload offers
 * the same fallback -- but this class always has the more specific
 * classpath-resource path available instead, so that overload is never
 * used here.
 */
final class NativeLibraryLoader {

    private static final String OVERRIDE_PROPERTY = "dengjen.tashkeel.native.library.path";

    // Owner-only permissions for the extracted native library: the system
    // temp directory is world-writable on multi-user hosts, and this file
    // gets loaded and executed. POSIX permissions aren't supported on
    // Windows, so fall back to the platform default there.
    private static final FileAttribute<?>[] OWNER_ONLY_PERMISSIONS =
            FileSystems.getDefault().supportedFileAttributeViews().contains("posix")
                    ? new FileAttribute<?>[] {
                        PosixFilePermissions.asFileAttribute(PosixFilePermissions.fromString("rw-------"))
                    }
                    : new FileAttribute<?>[0];

    private NativeLibraryLoader() {
    }

    static SymbolLookup load() {
        String override = System.getProperty(OVERRIDE_PROPERTY);
        if (override != null) {
            return SymbolLookup.libraryLookup(Path.of(override), Arena.global());
        }
        String classifier =
                NativePlatform.classifier(System.getProperty("os.name"), System.getProperty("os.arch"));
        String libraryName = System.mapLibraryName("dengjen_tashkeel_capi");
        // NOSONAR(java:S1075): this is a classpath/JAR-entry path, not a
        // filesystem path -- it must always use '/' per the JAR/ZIP spec
        // and ClassLoader.getResourceAsStream's contract, regardless of
        // the host OS. File.separator would be wrong here (and break on
        // Windows).
        String resourcePath = "natives/" + classifier + "/" + libraryName; // NOSONAR
        Path extracted;
        try {
            extracted = extractResource(resourcePath, NativeLibraryLoader.class.getClassLoader());
        } catch (IOException e) {
            throw new UncheckedIOException("failed to extract native library " + resourcePath, e);
        }
        return SymbolLookup.libraryLookup(extracted, Arena.global());
    }

    static Path extractResource(String resourcePath, ClassLoader loader) throws IOException {
        try (InputStream in = loader.getResourceAsStream(resourcePath)) {
            if (in == null) {
                throw new IllegalStateException(
                        "no native library found on the classpath at '"
                                + resourcePath
                                + "' -- add a runtimeOnly dependency on the matching "
                                + "io.github.zirekhq:dengjen-tashkeel:<version>:<classifier> "
                                + "artifact, or set -D" + OVERRIDE_PROPERTY + " to a library "
                                + "you built yourself");
            }
            Path extracted = Files.createTempFile(
                    "dengjen-tashkeel-native-", "-" + resourcePath.replace('/', '_'), OWNER_ONLY_PERMISSIONS);
            extracted.toFile().deleteOnExit();
            // Files.copy(..., REPLACE_EXISTING) deletes and recreates the target, discarding
            // createTempFile's owner-only permissions. The temp file was just created uniquely
            // above, so there's nothing to replace -- write directly into it instead.
            try (OutputStream out = Files.newOutputStream(extracted, StandardOpenOption.TRUNCATE_EXISTING)) {
                in.transferTo(out);
            }
            return extracted;
        }
    }
}
