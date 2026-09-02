package io.github.zirekhq.dengjentashkeel;

import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.Optional;

/**
 * Diacritizes Arabic text via dengjen-tashkeel's native inference engine.
 *
 * <p>The underlying native library holds its inference engine in a single
 * process-wide slot: the first successful {@link #load} or {@link
 * #loadDefault} call -- or the first {@link #diacritize} call on an
 * uninitialized instance, which lazily loads the default model -- wins
 * for the life of the JVM process. A later {@code load}/{@code
 * loadDefault} call throws a {@link TashkeelException.Unknown} rather
 * than replacing it. Every {@code Tashkeel} instance is a stateless
 * handle onto that same global engine; constructing more than one has no
 * effect beyond the first successful initialization.
 *
 * <p>Requires the native {@code dengjen_tashkeel_capi} shared library on
 * {@code jna.library.path} or {@code java.library.path}. See the
 * repository README for how to obtain it.
 */
public final class Tashkeel implements AutoCloseable {

    private static final int SUCCESS = 0;
    private static final int INPUT_TOO_LONG = 1;
    private static final int INFERENCE_ERROR = 2;
    private static final int MODEL_LOAD_ERROR = 3;

    private static final NativeLibrary LIB = Native.load("dengjen_tashkeel_capi", NativeLibrary.class);

    // Package-private: tests construct directly to avoid consuming the
    // one-shot global-init slot that load()/loadDefault() would. External
    // callers only ever see the static factories.
    Tashkeel() {
    }

    /** Initializes the native engine with a specific ONNX model file. */
    public static Tashkeel load(Path modelPath) throws TashkeelException {
        ExternError.ByReference outError = new ExternError.ByReference();
        LIB.dengjen_tashkeel_init(toNativeUtf8(modelPath.toString()), outError);
        checkError(outError);
        return new Tashkeel();
    }

    /** Initializes the native engine with its bundled default model. */
    public static Tashkeel loadDefault() throws TashkeelException {
        ExternError.ByReference outError = new ExternError.ByReference();
        LIB.dengjen_tashkeel_init(null, outError);
        checkError(outError);
        return new Tashkeel();
    }

    /**
     * Diacritizes {@code text}. Lazily initializes the default engine if
     * nothing has initialized it yet.
     *
     * @param taskeenThreshold confidence threshold for the taskeen
     *     (sukoon) diacritic; {@link Optional#empty()} uses the engine's
     *     default
     * @param preprocessed whether {@code text} has already been run
     *     through the library's Arabic text normalization
     */
    public String diacritize(String text, Optional<Float> taskeenThreshold, boolean preprocessed)
            throws TashkeelException {
        ExternError.ByReference outError = new ExternError.ByReference();
        Pointer textPtr = toNativeUtf8(text);
        Pointer thresholdPtr = taskeenThreshold.map(Tashkeel::toNativeFloat).orElse(null);

        Pointer resultPtr = LIB.dengjenTashkeelTashkeel(textPtr, thresholdPtr, preprocessed, outError);
        checkError(outError);
        try {
            return resultPtr.getString(0, "UTF-8");
        } finally {
            LIB.dengjen_tashkeel_free_string(resultPtr);
        }
    }

    @Override
    public void close() {
        // The C ABI has no per-call teardown -- the native engine lives
        // for the process's lifetime. Present for API symmetry and
        // try-with-resources.
    }

    private static void checkError(ExternError outError) throws TashkeelException {
        if (outError.code == SUCCESS) {
            return;
        }
        String message = outError.message == null ? "" : outError.message.getString(0, "UTF-8");
        LIB.dengjen_tashkeel_free_string(outError.message);
        throw new TashkeelException(switch (outError.code) {
            case INPUT_TOO_LONG -> new TashkeelException.InputTooLong(message);
            case INFERENCE_ERROR -> new TashkeelException.InferenceError(message);
            case MODEL_LOAD_ERROR -> new TashkeelException.ModelLoadError(message);
            default -> new TashkeelException.Unknown(outError.code, message);
        });
    }

    private static Pointer toNativeUtf8(String s) {
        byte[] bytes = s.getBytes(StandardCharsets.UTF_8);
        Memory memory = new Memory(bytes.length + 1L);
        memory.write(0, bytes, 0, bytes.length);
        memory.setByte(bytes.length, (byte) 0);
        return memory;
    }

    private static Pointer toNativeFloat(float value) {
        Memory memory = new Memory(Float.BYTES);
        memory.setFloat(0, value);
        return memory;
    }
}
