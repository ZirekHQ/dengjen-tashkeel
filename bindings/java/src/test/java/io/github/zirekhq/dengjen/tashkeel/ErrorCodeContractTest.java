package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;

class ErrorCodeContractTest {

    private static final Path HEADER = Path.of("../../crates/capi/dengjen_tashkeel.h");
    private static final Pattern DEFINE = Pattern.compile("^#define (DENGJEN_TASHKEEL_[A-Z_]+) (\\d+)$");

    private static Map<String, Integer> headerCodes() throws IOException {
        return Files.readAllLines(HEADER).stream()
                .map(DEFINE::matcher)
                .filter(Matcher::matches)
                .collect(Collectors.toMap(m -> m.group(1), m -> Integer.parseInt(m.group(2))));
    }

    @Test
    void javaReasonsMatchTheCHeaderCodes() throws IOException {
        Map<String, Integer> codes = headerCodes();

        assertInstanceOf(TashkeelException.InputTooLong.class,
                Tashkeel.reasonFor(codes.get("DENGJEN_TASHKEEL_INPUT_TOO_LONG"), "m"));
        assertInstanceOf(TashkeelException.InferenceError.class,
                Tashkeel.reasonFor(codes.get("DENGJEN_TASHKEEL_INFERENCE_ERROR"), "m"));
        assertInstanceOf(TashkeelException.ModelLoadError.class,
                Tashkeel.reasonFor(codes.get("DENGJEN_TASHKEEL_MODEL_LOAD_ERROR"), "m"));
    }

    @Test
    void remainingHeaderCodesMapToUnknownWithTheirCode() throws IOException {
        Map<String, Integer> codes = headerCodes();

        for (String name : new String[] {
            "DENGJEN_TASHKEEL_ALREADY_INITIALIZED",
            "DENGJEN_TASHKEEL_INVALID_ARGUMENT",
            "DENGJEN_TASHKEEL_UNKNOWN_ERROR"
        }) {
            TashkeelException.Unknown unknown =
                    assertInstanceOf(TashkeelException.Unknown.class, Tashkeel.reasonFor(codes.get(name), "m"));
            assertEquals(codes.get(name), unknown.code());
        }
    }
}
