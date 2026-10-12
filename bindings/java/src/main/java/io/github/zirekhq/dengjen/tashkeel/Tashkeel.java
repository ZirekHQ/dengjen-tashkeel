package io.github.zirekhq.dengjen.tashkeel;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.nio.file.Path;
import java.util.Objects;
import java.util.Optional;

public final class Tashkeel implements AutoCloseable {

    private static final int SUCCESS = 0;
    private static final int INPUT_TOO_LONG = 1;
    private static final int INFERENCE_ERROR = 2;
    private static final int MODEL_LOAD_ERROR = 3;
    private static final int ALREADY_INITIALIZED = 4;
    private static final int INVALID_ARGUMENT = 5;

    // The native engine is process-global, so the instances load() hands out share one engine.
    private static final Object LOAD_LOCK = new Object();
    private static Optional<Loaded> loaded = Optional.empty();

    record Loaded(Tashkeel engine, Optional<Path> modelPath) {
    }

    Tashkeel() {
    }

    /**
     * Loads the engine from {@code modelPath}.
     *
     * <p>Repeating a successful load with the same path returns the same instance; loading a
     * different model, or the default one, after that throws {@link TashkeelException.AlreadyInitialized}
     * because the native engine cannot be replaced.
     */
    public static Tashkeel load(Path modelPath) throws TashkeelException {
        Objects.requireNonNull(modelPath, "modelPath must not be null");
        return loadFrom(Optional.of(modelPath.toAbsolutePath().normalize()));
    }

    /**
     * Loads the bundled model; repeating a successful call returns the same instance.
     */
    public static Tashkeel loadDefault() throws TashkeelException {
        return loadFrom(Optional.empty());
    }

    private static Tashkeel loadFrom(Optional<Path> modelPath) throws TashkeelException {
        synchronized (LOAD_LOCK) {
            if (loaded.isPresent()) {
                return reuse(loaded.get(), modelPath);
            }
            try (Arena arena = Arena.ofConfined()) {
                MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
                MemorySegment modelPathPtr = modelPath
                        .map(path -> arena.allocateFrom(path.toString()))
                        .orElse(MemorySegment.NULL);
                invokeInit(modelPathPtr, outError);
                checkError(outError);
            }
            Tashkeel engine = new Tashkeel();
            loaded = Optional.of(new Loaded(engine, modelPath));
            return engine;
        }
    }

    static Tashkeel reuse(Loaded existing, Optional<Path> requested) throws TashkeelException {
        if (existing.modelPath().equals(requested)) {
            return existing.engine();
        }
        throw new TashkeelException(new TashkeelException.AlreadyInitialized(
                "engine already loaded from " + describe(existing.modelPath())
                        + "; cannot load " + describe(requested)));
    }

    private static String describe(Optional<Path> modelPath) {
        return modelPath.map(path -> "model " + path).orElse("the bundled model");
    }

    public String diacritize(String text, Optional<Float> taskeenThreshold, boolean preprocessed)
            throws TashkeelException {
        Objects.requireNonNull(text, "text must not be null");
        Objects.requireNonNull(taskeenThreshold, "taskeenThreshold must not be null; use Optional.empty()");
        if (text.indexOf('\u0000') >= 0) {
            throw new IllegalArgumentException("text must not contain U+0000");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
            MemorySegment textPtr = arena.allocateFrom(text);
            MemorySegment thresholdPtr = taskeenThreshold
                    .map(value -> toNativeFloat(arena, value))
                    .orElse(MemorySegment.NULL);

            MethodHandle tashkeel = TashkeelLib.handles().tashkeel();
            MemorySegment resultPtr;
            try {
                resultPtr = (MemorySegment)
                        tashkeel.invokeExact(textPtr, thresholdPtr, preprocessed, outError);
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
        // No-op: init sets a process-global native singleton with no teardown
        // of its own (dengjen_tashkeel.h) -- nothing to release per instance.
        // Kept AutoCloseable so a future native deinit slots in without breaking callers.
    }

    private static void invokeInit(MemorySegment modelPathPtr, MemorySegment outError) {
        MethodHandle init = TashkeelLib.handles().init();
        try {
            init.invokeExact(modelPathPtr, outError);
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
        throw new TashkeelException(reasonFor(code, message));
    }

    static TashkeelException.Reason reasonFor(int code, String message) {
        return switch (code) {
            case INPUT_TOO_LONG -> new TashkeelException.InputTooLong(message);
            case INFERENCE_ERROR -> new TashkeelException.InferenceError(message);
            case MODEL_LOAD_ERROR -> new TashkeelException.ModelLoadError(message);
            case ALREADY_INITIALIZED -> new TashkeelException.AlreadyInitialized(message);
            case INVALID_ARGUMENT -> new TashkeelException.InvalidArgument(message);
            default -> new TashkeelException.Unknown(code, message);
        };
    }

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
        MethodHandle freeString = TashkeelLib.handles().freeString();
        try {
            freeString.invokeExact(ptr);
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
