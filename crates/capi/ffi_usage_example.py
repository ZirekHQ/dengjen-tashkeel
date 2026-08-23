# coding: utf-8

import ctypes
import os


# Change this based on your platform and build: liblibtashkeel.so (Linux),
# liblibtashkeel.dylib (macOS), or libtashkeel.dll (Windows). The library's
# crate name is "libtashkeel" (see crates/capi/Cargo.toml's [lib] section),
# and cargo prefixes cdylib outputs with "lib" on Unix, hence the double
# "lib" in the Linux/macOS filenames. This script assumes it's run from
# crates/capi/ (its own directory) -- the workspace's target/ dir is two
# levels up from there.
LIBTASHKEEL_PATH = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "..", "target", "debug", "liblibtashkeel.so")
)


class ExternError(ctypes.Structure):
    """Mirrors the `ExternError` struct in libtashkeel.h: an int32 error
    code (0 == success) and an owned, nul-terminated error message pointer
    (null when there's no error)."""

    _fields_ = [
        ("code", ctypes.c_int32),
        ("message", ctypes.c_void_p),
    ]

    def take_message(self):
        """Read and free the error message. Call at most once per error."""
        if not self.message:
            return None
        text = ctypes.cast(self.message, ctypes.c_char_p).value.decode("utf-8")
        lib.libtashkeel_free_string(self.message)
        return text


lib = ctypes.cdll.LoadLibrary(LIBTASHKEEL_PATH)

lib.libtashkeelTashkeel.argtypes = (
    ctypes.c_char_p,
    ctypes.POINTER(ctypes.c_float),
    ctypes.c_bool,
    ctypes.POINTER(ExternError),
)
lib.libtashkeelTashkeel.restype = ctypes.c_void_p
lib.libtashkeel_init.argtypes = (ctypes.c_char_p, ctypes.POINTER(ExternError))
lib.libtashkeel_free_string.argtypes = (ctypes.c_void_p,)


def init(model_path=None):
    """Explicitly initialize libtashkeel with an optional custom ONNX model
    path (None uses the bundled default model). Calling tashkeel() without
    ever calling init() first also works -- it lazily initializes on first
    use with the bundled default model."""
    err = ExternError()
    path_bytes = model_path.encode("utf-8") if model_path else None
    lib.libtashkeel_init(path_bytes, ctypes.byref(err))
    if err.code != 0:
        raise RuntimeError(err.take_message())


def tashkeel(text, taskeen_threshold=None, preprocessed=False):
    err = ExternError()
    # taskeen_threshold is borrowed by libtashkeelTashkeel for the duration
    # of this call only -- it is never freed on the Rust side. ctypes keeps
    # this local c_float alive for exactly that long via the pointer's
    # internal reference, so no explicit cleanup is needed here.
    threshold_ptr = (
        ctypes.pointer(ctypes.c_float(taskeen_threshold))
        if taskeen_threshold is not None
        else None
    )
    ptr = lib.libtashkeelTashkeel(
        text.encode("utf-8"),
        threshold_ptr,
        preprocessed,
        ctypes.byref(err),
    )
    if err.code != 0:
        raise RuntimeError(err.take_message())
    try:
        return ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
    finally:
        lib.libtashkeel_free_string(ptr)


if __name__ == "__main__":
    print(tashkeel("إن روعة اللغة العربية لا تتبدى إلا لعشاقها"))
