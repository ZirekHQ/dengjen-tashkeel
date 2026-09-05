# coding: utf-8

import ctypes
import os


DENGJEN_TASHKEEL_PATH = os.path.abspath(
    os.path.join(
        os.path.dirname(__file__), "..", "..", "target", "debug", "libdengjen_tashkeel_capi.so"
    )
)


class ExternError(ctypes.Structure):
    """Mirrors the `ExternError` struct in dengjen_tashkeel.h: an int32
    error code (0 == success) and an owned, nul-terminated error message
    pointer (null when there's no error)."""

    _fields_ = [
        ("code", ctypes.c_int32),
        ("message", ctypes.c_void_p),
    ]

    def take_message(self):
        """Read and free the error message. Call at most once per error."""
        if not self.message:
            return None
        text = ctypes.cast(self.message, ctypes.c_char_p).value.decode("utf-8")
        lib.dengjen_tashkeel_free_string(self.message)
        return text


lib = ctypes.cdll.LoadLibrary(DENGJEN_TASHKEEL_PATH)

lib.dengjenTashkeelTashkeel.argtypes = (
    ctypes.c_char_p,
    ctypes.POINTER(ctypes.c_float),
    ctypes.c_bool,
    ctypes.POINTER(ExternError),
)
lib.dengjenTashkeelTashkeel.restype = ctypes.c_void_p
lib.dengjen_tashkeel_init.argtypes = (ctypes.c_char_p, ctypes.POINTER(ExternError))
lib.dengjen_tashkeel_free_string.argtypes = (ctypes.c_void_p,)


def init(model_path=None):
    """Explicitly initialize dengjen_tashkeel with an optional custom ONNX
    model path (None uses the bundled default model). Calling tashkeel()
    without ever calling init() first also works -- it lazily initializes on
    first use with the bundled default model."""
    err = ExternError()
    path_bytes = model_path.encode("utf-8") if model_path else None
    lib.dengjen_tashkeel_init(path_bytes, ctypes.byref(err))
    if err.code != 0:
        raise RuntimeError(err.take_message())


def tashkeel(text, taskeen_threshold=None, preprocessed=False):
    err = ExternError()
    threshold_ptr = (
        ctypes.pointer(ctypes.c_float(taskeen_threshold))
        if taskeen_threshold is not None
        else None
    )
    ptr = lib.dengjenTashkeelTashkeel(
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
        lib.dengjen_tashkeel_free_string(ptr)


if __name__ == "__main__":
    print(tashkeel("إن روعة اللغة العربية لا تتبدى إلا لعشاقها"))
