package io.github.zirekhq.dengjen.tashkeel;

import java.io.Serializable;

/** Failure reported by the native tashkeel engine; {@link #reason()} says which kind. */
public final class TashkeelException extends Exception {

    /** The failure kind. */
    private final Reason reason;

    /**
     * Creates an exception whose message is the reason's message.
     *
     * @param reason the failure kind
     */
    public TashkeelException(Reason reason) {
        super(reason.message());
        this.reason = reason;
    }

    /**
     * Returns the failure kind.
     *
     * @return the reason, switchable over its sealed subtypes
     */
    public Reason reason() {
        return reason;
    }

    /** A failure kind, matching the error codes in {@code dengjen_tashkeel.h}. */
    public sealed interface Reason extends Serializable permits InputTooLong, InferenceError, ModelLoadError, AlreadyInitialized, InvalidArgument, Unknown {
        /**
         * Returns the native error message.
         *
         * @return the message, empty when the engine gave none
         */
        String message();
    }

    /**
     * A sentence exceeded the engine's length limit.
     *
     * @param message the native error message
     */
    public record InputTooLong(String message) implements Reason {
    }

    /**
     * The model failed during initialization or while running.
     *
     * @param message the native error message
     */
    public record InferenceError(String message) implements Reason {
    }

    /**
     * The model could not be loaded.
     *
     * @param message the native error message
     */
    public record ModelLoadError(String message) implements Reason {
    }

    /**
     * The engine is already loaded with a different model.
     *
     * @param message the native error message
     */
    public record AlreadyInitialized(String message) implements Reason {
    }

    /**
     * The engine rejected an argument.
     *
     * @param message the native error message
     */
    public record InvalidArgument(String message) implements Reason {
    }

    /**
     * An error code this binding does not know.
     *
     * @param code the native error code
     * @param message the native error message
     */
    public record Unknown(int code, String message) implements Reason {
    }
}
