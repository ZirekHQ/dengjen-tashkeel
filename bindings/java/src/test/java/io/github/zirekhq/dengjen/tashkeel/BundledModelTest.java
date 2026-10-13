package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.net.URL;
import java.net.URLClassLoader;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class BundledModelTest {

    @Test
    void extractsTheModelFromTheClasspathAsReadableFile() throws Exception {
        Optional<Path> extracted = BundledModel.extract(BundledModelTest.class.getClassLoader());

        assertTrue(extracted.isPresent(), "dengjen-tashkeel-model must be on the test runtime classpath");
        try {
            assertTrue(Files.size(extracted.get()) > 1_000_000);
        } finally {
            Files.deleteIfExists(extracted.get());
        }
    }

    @Test
    void reportsNothingWhenTheModelArtifactIsAbsent() throws Exception {
        try (URLClassLoader empty = new URLClassLoader(new URL[0], null)) {
            assertEquals(Optional.empty(), BundledModel.extract(empty));
        }
    }
}
