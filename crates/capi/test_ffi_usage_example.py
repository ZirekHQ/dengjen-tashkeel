import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

try:
    import ffi_usage_example as m
except OSError as error:
    pytest.skip(f"capi library not built: {error}", allow_module_level=True)

# Mirrors dengjen_tashkeel::CHAR_LIMIT (crates/core/src/lib.rs).
CHAR_LIMIT = 12000
SAMPLE = "بسم الله الرحمن الرحيم"


def test_tashkeel_diacritizes_without_explicit_init():
    result = m.tashkeel(SAMPLE)

    assert result
    assert result != SAMPLE


def test_taskeen_threshold_changes_the_output():
    assert m.tashkeel(SAMPLE, taskeen_threshold=0.8) != m.tashkeel(SAMPLE)


def test_init_after_the_engine_exists_raises():
    m.tashkeel(SAMPLE)

    with pytest.raises(RuntimeError, match="already initialized"):
        m.init()


def test_input_over_the_char_limit_raises():
    with pytest.raises(RuntimeError, match="too long"):
        m.tashkeel("ا" * (CHAR_LIMIT + 1), preprocessed=True)


def test_extern_error_take_message_returns_none_when_no_message():
    err = m.ExternError()
    assert err.take_message() is None


def test_default_lib_name_selects_extension_per_platform(monkeypatch):
    monkeypatch.setattr(sys, "platform", "win32")
    assert m._default_lib_name() == "dengjen_tashkeel_capi.dll"

    monkeypatch.setattr(sys, "platform", "darwin")
    assert m._default_lib_name() == "libdengjen_tashkeel_capi.dylib"

    monkeypatch.setattr(sys, "platform", "linux")
    assert m._default_lib_name() == "libdengjen_tashkeel_capi.so"
