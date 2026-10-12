package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.Optional;
import org.junit.jupiter.api.Test;

class TashkeelArgumentsTest {

    private final Tashkeel tashkeel = new Tashkeel();

    @Test
    void nullTextIsRejected() {
        assertThrows(NullPointerException.class, () -> tashkeel.diacritize(null, Optional.empty(), true));
    }

    @Test
    void nullThresholdIsRejected() {
        assertThrows(NullPointerException.class, () -> tashkeel.diacritize("a", null, true));
    }

    @Test
    void textContainingNulCharacterIsRejected() {
        assertThrows(IllegalArgumentException.class,
                () -> tashkeel.diacritize("a\u0000b", Optional.empty(), true));
    }
}
