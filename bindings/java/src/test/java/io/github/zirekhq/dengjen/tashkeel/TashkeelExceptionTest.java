package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;

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
        assertEquals("too long: too long msg", describe(new TashkeelException.InputTooLong("too long msg")));
        assertEquals("inference: inference msg", describe(new TashkeelException.InferenceError("inference msg")));
        assertEquals("model: model msg", describe(new TashkeelException.ModelLoadError("model msg")));
        assertEquals("unknown(-1): panic", describe(new TashkeelException.Unknown(-1, "panic")));
    }

    private static String describe(TashkeelException.Reason reason) {
        return switch (reason) {
            case TashkeelException.InputTooLong r -> "too long: " + r.message();
            case TashkeelException.InferenceError r -> "inference: " + r.message();
            case TashkeelException.ModelLoadError r -> "model: " + r.message();
            case TashkeelException.Unknown r -> "unknown(" + r.code() + "): " + r.message();
        };
    }

    @Test
    void nativeErrorCodesMapToTheirReasons() {
        assertEquals(new TashkeelException.InputTooLong("m"), Tashkeel.reasonFor(1, "m"));
        assertEquals(new TashkeelException.InferenceError("m"), Tashkeel.reasonFor(2, "m"));
        assertEquals(new TashkeelException.ModelLoadError("m"), Tashkeel.reasonFor(3, "m"));
        assertEquals(new TashkeelException.Unknown(99, "m"), Tashkeel.reasonFor(99, "m"));
    }
}
