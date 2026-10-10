package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Optional;
import org.junit.jupiter.api.MethodOrderer;
import org.junit.jupiter.api.Order;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestMethodOrder;
import org.junit.jupiter.api.io.TempDir;

// The native engine is a process-global singleton: tests that expect a failing
// init must run before any test that initialises it.
@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
class TashkeelTest {

    // Must match dengjen_tashkeel::CHAR_LIMIT (crates/core/src/lib.rs).
    private static final int CHAR_LIMIT = 12000;

    @Test
    @Order(2)
    void diacritizeLazilyInitializesTheDefaultEngineAndChangesTheText() throws Exception {
        Tashkeel tashkeel = new Tashkeel();

        String result = tashkeel.diacritize("بسم الله الرحمن الرحيم", Optional.empty(), true);

        assertNotEquals("بسم الله الرحمن الرحيم", result);
        assertTrue(result.codePoints().anyMatch(c -> c >= 0x064B && c <= 0x0652),
                "result should contain Arabic diacritics");
        assertEquals("بسم الله الرحمن الرحيم",
                result.replaceAll("[\\u064B-\\u0652]", ""),
                "stripping diacritics should recover the input verbatim");
    }

    @Test
    @Order(2)
    void diacritizeWithTaskeenThresholdSucceeds() throws Exception {
        Tashkeel tashkeel = new Tashkeel();

        String result = tashkeel.diacritize("بسم الله الرحمن الرحيم", Optional.of(0.8f), true);

        assertTrue(result != null && !result.isEmpty(), "result should be a non-empty string");
    }

    @Test
    @Order(2)
    void diacritizeOverCharLimitThrowsInputTooLong() {
        Tashkeel tashkeel = new Tashkeel();
        String tooLong = "ا".repeat(CHAR_LIMIT + 1);

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> tashkeel.diacritize(tooLong, Optional.empty(), true));

        assertInstanceOf(TashkeelException.InputTooLong.class, exception.reason());
    }

    @Test
    @Order(1)
    void loadWithNonexistentModelPathThrowsModelLoadError() {
        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.load(Path.of("/nonexistent/path/to/model.onnx")));

        assertInstanceOf(TashkeelException.ModelLoadError.class, exception.reason());
    }

    @Test
    @Order(1)
    void loadWithMalformedModelFileThrowsInferenceError(@TempDir Path tempDir) throws IOException {
        Path malformed = tempDir.resolve("dengjen-tashkeel-test.onnx");
        Files.writeString(malformed, "this is not a valid onnx model");

        TashkeelException exception = assertThrows(TashkeelException.class,
                () -> Tashkeel.load(malformed));

        assertInstanceOf(TashkeelException.InferenceError.class, exception.reason());
    }

    @Test
    @Order(3)
    void loadAfterTheEngineIsAlreadyInitializedThrowsUnknown() throws Exception {
        new Tashkeel().diacritize("بسم الله", Optional.empty(), true);

        TashkeelException exception = assertThrows(TashkeelException.class, Tashkeel::loadDefault);

        assertInstanceOf(TashkeelException.Unknown.class, exception.reason());
        assertEquals(4, ((TashkeelException.Unknown) exception.reason()).code());
    }

    @Test
    @Order(2)
    void diacritizeWithoutPreprocessingKeepsTheInputLetters() throws Exception {
        String input = "بسم الله الرحمن الرحيم";

        String result = new Tashkeel().diacritize(input, Optional.empty(), false);

        assertEquals(input, result.replaceAll("[\\u064B-\\u0652]", ""));
    }

    @Test
    @Order(2)
    void diacritizeEmptyStringReturnsEmptyString() throws Exception {
        assertEquals("", new Tashkeel().diacritize("", Optional.empty(), true));
    }

    @Test
    @Order(2)
    void repeatedDiacritizeCallsReturnTheSameResult() throws Exception {
        Tashkeel tashkeel = new Tashkeel();

        String first = tashkeel.diacritize("بسم الله", Optional.empty(), true);

        for (int i = 0; i < 50; i++) {
            assertEquals(first, tashkeel.diacritize("بسم الله", Optional.empty(), true));
        }
    }

    @Test
    @Order(2)
    void diacritizeNullTextThrowsNullPointerException() {
        assertThrows(NullPointerException.class,
                () -> new Tashkeel().diacritize(null, Optional.empty(), true));
    }

    @Test
    @Order(2)
    void loadNullModelPathThrowsNullPointerException() {
        assertThrows(NullPointerException.class, () -> Tashkeel.load(null));
    }
}
