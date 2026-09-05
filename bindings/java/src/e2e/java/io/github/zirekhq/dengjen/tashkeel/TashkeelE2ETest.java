package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.nio.file.Path;
import java.util.Optional;
import org.junit.jupiter.api.Test;

class TashkeelE2ETest {

    private static final String MODEL_PATH_ENV_VAR = "DENGJEN_TASHKEEL_TEST_MODEL_PATH";

    @Test
    void loadWithARealExternalModelDiacritizesText() throws Exception {
        String modelPath = System.getenv(MODEL_PATH_ENV_VAR);
        assumeTrue(modelPath != null, MODEL_PATH_ENV_VAR + " not set -- skipping e2e test");

        Tashkeel tashkeel = Tashkeel.load(Path.of(modelPath));

        String result = tashkeel.diacritize("بسم الله الرحمن الرحيم", Optional.empty(), true);

        assertNotEquals("بسم الله الرحمن الرحيم", result);
        assertTrue(
                result.codePoints().anyMatch(c -> c >= 0x064B && c <= 0x0652),
                "result should contain Arabic diacritics");
    }
}
