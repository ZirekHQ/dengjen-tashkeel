package io.github.zirekhq.dengjentashkeel;

import com.sun.jna.Pointer;
import com.sun.jna.Structure;

/**
 * Mirrors {@code struct ExternError { ErrorCode code; char *message; }}
 * from {@code dengjen_tashkeel.h}. Field order matters -- it must match
 * the C layout exactly (code first, then message).
 */
@Structure.FieldOrder({"code", "message"})
public class ExternError extends Structure {
    public int code;
    public Pointer message;

    public static class ByReference extends ExternError implements Structure.ByReference {
    }
}
