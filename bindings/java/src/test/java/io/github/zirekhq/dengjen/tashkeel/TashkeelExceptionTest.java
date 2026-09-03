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
        if (reason instanceof TashkeelException.InputTooLong r) {
            return "too long: " + r.message();
        } else if (reason instanceof TashkeelException.InferenceError r) {
            return "inference: " + r.message();
        } else if (reason instanceof TashkeelException.ModelLoadError r) {
            return "model: " + r.message();
        } else if (reason instanceof TashkeelException.Unknown r) {
            return "unknown(" + r.code() + "): " + r.message();
        } else {
            throw new AssertionError("unreachable: Reason is sealed to these four variants");
        }
    }
}
