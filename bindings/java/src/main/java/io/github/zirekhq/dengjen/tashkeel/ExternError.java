package io.github.zirekhq.dengjen.tashkeel;

import com.sun.jna.Pointer;
import com.sun.jna.Structure;

/**
 * Mirrors {@code struct ExternError { ErrorCode code; char *message; }}
 * from {@code dengjen_tashkeel.h}. Field order matters -- it must match
 * the C layout exactly (code first, then message).
 *
 * <p>Public only because JNA 5.15.0's {@code Structure} field reflection
 * requires a public declaring class -- it does not call {@code
 * setAccessible(true)} before accessing fields. This is not part of this
 * library's supported API and may change without notice. Do not narrow
 * this back to package-private: doing so compiles fine but breaks native
 * field marshalling at runtime with an {@code IllegalAccessException}.
 */
@Structure.FieldOrder({"code", "message"})
public class ExternError extends Structure {
    public int code;
    public Pointer message;

    public static class ByReference extends ExternError implements Structure.ByReference {
    }
}
