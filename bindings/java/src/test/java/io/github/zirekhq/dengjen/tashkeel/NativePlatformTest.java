package io.github.zirekhq.dengjen.tashkeel;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class NativePlatformTest {

    @Test
    void linuxX64IsRecognized() {
        assertEquals("linux-x86_64", NativePlatform.classifier("Linux", "amd64"));
        assertEquals("linux-x86_64", NativePlatform.classifier("Linux", "x86_64"));
    }

    @Test
    void windowsX64IsRecognized() {
        assertEquals("windows-x64", NativePlatform.classifier("Windows 11", "amd64"));
    }

    @Test
    void macosAarch64IsRecognized() {
        assertEquals("macos-aarch64", NativePlatform.classifier("Mac OS X", "aarch64"));
        assertEquals("macos-aarch64", NativePlatform.classifier("Darwin", "arm64"));
    }

    @Test
    void linuxAarch64IsUnsupported() {
        assertThrows(IllegalStateException.class, () -> NativePlatform.classifier("Linux", "aarch64"));
    }

    @Test
    void unknownOsIsUnsupported() {
        assertThrows(IllegalStateException.class, () -> NativePlatform.classifier("SunOS", "x86_64"));
    }

    @Test
    void unknownWindowsArchIsUnsupported() {
        assertThrows(IllegalStateException.class, () -> NativePlatform.classifier("Windows 11", "aarch64"));
    }

    @Test
    void unknownMacosArchIsUnsupported() {
        assertThrows(IllegalStateException.class, () -> NativePlatform.classifier("Mac OS X", "x86_64"));
    }
}
