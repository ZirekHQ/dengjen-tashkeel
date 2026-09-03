package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Optional;
import org.junit.jupiter.api.Test;

/**
 * Exercises the classpath-resource native-library load path that every real
 * Maven Central consumer uses (a {@code runtimeOnly} dependency on a
 * per-platform classifier jar) -- as opposed to {@code TashkeelTest}/{@code
 * TashkeelE2ETest} (the {@code integrationTest}/{@code e2e} suites), which
 * both run with {@code -Ddengjen.tashkeel.native.library.path} set and never
 * touch this path.
 *
 * <p>This suite's only source of a native library is {@code
 * classpathNativeTestJar} (see {@code build.gradle.kts}) on its runtime
 * classpath, packaged with exactly the {@code natives/<classifier>/} layout
 * the real {@code nativeJar-<classifier>} release tasks use, built from the
 * real debug cdylib. Its {@code Test} task deliberately gets no override
 * system property, so a pass here proves the packaging layout and {@link
 * NativeLibraryLoader}'s classpath-resolution logic actually agree at
 * runtime, not just by inspection.
 */
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
