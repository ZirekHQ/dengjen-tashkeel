package io.github.zirekhq.dengjen.tashkeel;

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

    public record InputTooLong(String message) implements Reason {
    }

    public record InferenceError(String message) implements Reason {
    }

    public record ModelLoadError(String message) implements Reason {
    }

    public record Unknown(int code, String message) implements Reason {
    }
}
