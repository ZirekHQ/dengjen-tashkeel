
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import ffi_usage_example as m

# Mirrors dengjen_tashkeel::CHAR_LIMIT (crates/core/src/lib.rs).
CHAR_LIMIT = 12000


def test_full_lifecycle_and_error_paths():
    m.init()

    result = m.tashkeel("بسم الله الرحمن الرحيم")
    assert result != "بسم الله الرحمن الرحيم"
    assert result

    with_taskeen = m.tashkeel("بسم الله الرحمن الرحيم", taskeen_threshold=0.8)
    assert with_taskeen != result

    with pytest.raises(RuntimeError):
        m.init()

    too_long_text = "ا" * (CHAR_LIMIT + 1)
    with pytest.raises(RuntimeError):
        m.tashkeel(too_long_text, preprocessed=True)


def test_extern_error_take_message_returns_none_when_no_message():
    err = m.ExternError()
    assert err.take_message() is None
