package io.github.zirekhq.dengjen.tashkeel;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Optional;

/** Reads the model shipped by the {@code dengjen-tashkeel-model} artifact, when it is on the classpath. */
final class BundledModel {

    static final String RESOURCE = "io/github/zirekhq/dengjen/tashkeel/model/model.onnx";

    private BundledModel() {
    }

    /** Copies the model to a temporary file the caller must delete; empty when the artifact is absent. */
    static Optional<Path> extract(ClassLoader loader) throws IOException {
        try (InputStream in = loader.getResourceAsStream(RESOURCE)) {
            if (in == null) {
                return Optional.empty();
            }
            Path extracted = Files.createTempFile("dengjen-tashkeel-model-", ".onnx");
            try {
                Files.copy(in, extracted, StandardCopyOption.REPLACE_EXISTING);
            } catch (IOException e) {
                Files.deleteIfExists(extracted);
                throw e;
            }
            return Optional.of(extracted);
        }
    }
}
