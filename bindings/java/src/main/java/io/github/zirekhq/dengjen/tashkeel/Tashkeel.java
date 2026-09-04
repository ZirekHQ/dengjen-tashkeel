package io.github.zirekhq.dengjen.tashkeel;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.nio.file.Path;
import java.util.Optional;

public final class Tashkeel implements AutoCloseable {

    private static final int SUCCESS = 0;
    private static final int INPUT_TOO_LONG = 1;
    private static final int INFERENCE_ERROR = 2;
    private static final int MODEL_LOAD_ERROR = 3;

    Tashkeel() {
    }

    public static Tashkeel load(Path modelPath) throws TashkeelException {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
            MemorySegment modelPathPtr = arena.allocateFrom(modelPath.toString());
            invokeInit(modelPathPtr, outError);
            checkError(outError);
            return new Tashkeel();
        }
    }

    public static Tashkeel loadDefault() throws TashkeelException {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment outError = arena.allocate(TashkeelLib.EXTERN_ERROR);
            invokeInit(MemorySegment.NULL, outError);
            checkError(outError);
            return new Tashkeel();
        }
    }

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
