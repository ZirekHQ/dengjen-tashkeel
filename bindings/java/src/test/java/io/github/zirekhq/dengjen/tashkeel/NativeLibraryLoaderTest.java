package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.jupiter.api.Test;

class NativeLibraryLoaderTest {

    @Test
    void extractResourceCopiesClasspathResourceToATempFile() throws IOException {
        Path extracted = NativeLibraryLoader.extractResource(
                "nativeloadertest/sample.txt", NativeLibraryLoaderTest.class.getClassLoader());

        assertTrue(Files.exists(extracted));
        assertEquals("hello native library\n", Files.readString(extracted));
        assertTrue(extracted.getFileName().toString().contains("nativeloadertest_sample.txt"));
    }

    @Test
    void extractResourceThrowsWithAHelpfulMessageWhenMissing() {
        IllegalStateException exception = assertThrows(
                IllegalStateException.class,
                () -> NativeLibraryLoader.extractResource(
                        "nativeloadertest/does-not-exist.txt", NativeLibraryLoaderTest.class.getClassLoader()));

        assertTrue(exception.getMessage().contains("nativeloadertest/does-not-exist.txt"));
        assertTrue(exception.getMessage().contains("dengjen.tashkeel.native.library.path"));
    }
}
