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

final class NativeLibraryLoader {

    private static final String OVERRIDE_PROPERTY = "dengjen.tashkeel.native.library.path";

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
            try (OutputStream out = Files.newOutputStream(extracted, StandardOpenOption.TRUNCATE_EXISTING)) {
                in.transferTo(out);
            }
            return extracted;
        }
    }
}
