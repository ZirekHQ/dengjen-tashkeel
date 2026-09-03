package io.github.zirekhq.dengjen.tashkeel;

import com.sun.jna.Library;
import com.sun.jna.Pointer;

/**
 * Raw JNA mapping of {@code dengjen_tashkeel.h}. Package-private --
 * callers use {@link Tashkeel}, not this interface, directly.
 *
 * <p>Strings cross this boundary as manually UTF-8-encoded {@link
 * Pointer}s (see {@link Tashkeel}), not JNA's default String marshalling
 * -- JNA's default native string encoding follows the JVM's platform
 * charset, which corrupts Arabic text on platforms that don't default to
 * UTF-8.
 */
interface NativeLibrary extends Library {

    Pointer dengjenTashkeelTashkeel(
            Pointer textPtr, Pointer taskeenThresholdPtr, boolean preprocessed, ExternError.ByReference outError);

    void dengjen_tashkeel_init(Pointer modelPathPtr, ExternError.ByReference outError);

    void dengjen_tashkeel_free_string(Pointer s);
}
