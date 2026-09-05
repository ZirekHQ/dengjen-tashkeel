package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Optional;
import org.junit.jupiter.api.Test;

class ClasspathNativeLibraryTest {

    @Test
    void diacritizeSucceedsUsingTheClasspathPackagedNativeLibrary() throws Exception {
        Tashkeel tashkeel = new Tashkeel();

        String result = tashkeel.diacritize("بسم الله الرحمن الرحيم", Optional.empty(), true);

        assertNotEquals("بسم الله الرحمن الرحيم", result);
        assertTrue(result.codePoints().anyMatch(c -> c >= 0x064B && c <= 0x0652),
                "result should contain Arabic diacritics");
    }
}
