package io.github.zirekhq.dengjen.tashkeel;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
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
 * <p>Requires the native {@code dengjen_tashkeel_capi} shared library
 * either bundled via a native classifier artifact, or pointed at
 * explicitly via {@code -Ddengjen.tashkeel.native.library.path}. See the
 * repository README for how to obtain it.
 */
public final class Tashkeel implements AutoCloseable {

    private static final int SUCCESS = 0;
    private static final int INPUT_TOO_LONG = 1;
    private static final int INFERENCE_ERROR = 2;
    private static final int MODEL_LOAD_ERROR = 3;

    // Package-private: tests construct directly to avoid consuming the
    // one-shot global-init slot that load()/loadDefault() would. External
    // callers only ever see the static factories.
    Tashkeel() {
    }

    /** Initializes the native engine with a specific ONNX model file. */
    public static Tashkeel load(Path modelPath) throws TashkeelException {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
            MemorySegment modelPathPtr = arena.allocateFrom(modelPath.toString());
            invokeInit(modelPathPtr, outError);
            checkError(outError);
            return new Tashkeel();
        }
    }

    /** Initializes the native engine with its bundled default model. */
    public static Tashkeel loadDefault() throws TashkeelException {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
            invokeInit(MemorySegment.NULL, outError);
            checkError(outError);
            return new Tashkeel();
        }
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
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
            MemorySegment textPtr = arena.allocateFrom(text);
            MemorySegment thresholdPtr = taskeenThreshold
                    .map(value -> toNativeFloat(arena, value))
                    .orElse(MemorySegment.NULL);

            MemorySegment resultPtr;
            try {
                resultPtr = (MemorySegment)
                        TashkeelLib.TASHKEEL.invokeExact(textPtr, thresholdPtr, preprocessed, outError);
            } catch (Throwable t) {
                throw new IllegalStateException("dengjenTashkeelTashkeel downcall failed", t);
            }
            checkError(outError);
            try {
                return readString(resultPtr);
            } finally {
                freeString(resultPtr);
            }
        }
    }

    @Override
    public void close() {
        // The C ABI has no per-call teardown -- the native engine lives
        // for the process's lifetime. Present for API symmetry and
        // try-with-resources.
    }

    private static void invokeInit(MemorySegment modelPathPtr, MemorySegment outError) {
        try {
            TashkeelLib.INIT.invokeExact(modelPathPtr, outError);
        } catch (Throwable t) {
            throw new IllegalStateException("dengjen_tashkeel_init downcall failed", t);
        }
    }

    private static void checkError(MemorySegment outError) throws TashkeelException {
        int code = outError.get(ValueLayout.JAVA_INT, TashkeelLib.EXTERN_ERROR_CODE_OFFSET);
        if (code == SUCCESS) {
            return;
        }
        MemorySegment messagePtr = outError.get(ValueLayout.ADDRESS, TashkeelLib.EXTERN_ERROR_MESSAGE_OFFSET);
        String message;
        try {
            message = readString(messagePtr);
        } finally {
            freeString(messagePtr);
        }
        throw new TashkeelException(switch (code) {
            case INPUT_TOO_LONG -> new TashkeelException.InputTooLong(message);
            case INFERENCE_ERROR -> new TashkeelException.InferenceError(message);
            case MODEL_LOAD_ERROR -> new TashkeelException.ModelLoadError(message);
            default -> new TashkeelException.Unknown(code, message);
        });
    }

    // A pointer returned from a native call comes back as a zero-length
    // MemorySegment; it must be reinterpreted to a usable size before it
    // can be dereferenced. MemorySegment.getString defaults to UTF-8, so
    // -- unlike JNA -- no manual charset handling is needed here.
    private static String readString(MemorySegment ptr) {
        if (ptr.equals(MemorySegment.NULL)) {
            return "";
        }
        return ptr.reinterpret(Long.MAX_VALUE).getString(0);
    }

    private static void freeString(MemorySegment ptr) {
        if (ptr.equals(MemorySegment.NULL)) {
            return;
        }
        try {
            TashkeelLib.FREE_STRING.invokeExact(ptr);
        } catch (Throwable t) {
            throw new IllegalStateException("dengjen_tashkeel_free_string downcall failed", t);
        }
    }

    private static MemorySegment toNativeFloat(Arena arena, float value) {
        MemorySegment segment = arena.allocate(ValueLayout.JAVA_FLOAT);
        segment.set(ValueLayout.JAVA_FLOAT, 0, value);
        return segment;
    }
}
