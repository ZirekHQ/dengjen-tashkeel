package io.github.zirekhq.dengjentashkeel;

/**
 * Thrown when the native dengjen-tashkeel engine reports an error.
 * {@link #reason()} carries the specific failure as a sealed,
 * exhaustively switchable value mirroring the Rust
 * {@code DengjenTashkeelError} enum and the {@code ErrorCode} constants
 * in {@code dengjen_tashkeel.h}.
 */
public final class TashkeelException extends Exception {

    private final Reason reason;

    public TashkeelException(Reason reason) {
        super(reason.message());
        this.reason = reason;
    }

    public Reason reason() {
        return reason;
    }

    public sealed interface Reason permits InputTooLong, InferenceError, ModelLoadError, Unknown {
        String message();
    }

    /** ErrorCode 1: input exceeds {@code dengjen_tashkeel::CHAR_LIMIT} characters. */
    public record InputTooLong(String message) implements Reason {
    }

    /** ErrorCode 2: the engine failed during inference (includes a bad or malformed model path/file). */
    public record InferenceError(String message) implements Reason {
    }

    /** ErrorCode 3: the model file failed to load. */
    public record ModelLoadError(String message) implements Reason {
    }

    /** Any other ErrorCode, including PANIC (-1) and INVALID_HANDLE (-1000). */
    public record Unknown(int code, String message) implements Reason {
    }
}
