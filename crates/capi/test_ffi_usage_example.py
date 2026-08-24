# coding: utf-8

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

import ffi_usage_example as m

# Mirrors libtashkeel_core::CHAR_LIMIT (crates/core/src/lib.rs).
CHAR_LIMIT = 12000


def test_full_lifecycle_and_error_paths():
    # Step 1: explicit init() with the bundled default model succeeds. This
    # runs FIRST because INFERENCE_ENGINE is a process-global singleton on
    # the Rust side (shared across every ctypes call made from this Python
    # process) -- calling init() after any tashkeel() call would instead
    # hit the "already initialized" branch tested in Step 4.
    m.init()

    # Step 2: a normal call succeeds and actually diacritizes the text.
    result = m.tashkeel("بسم الله الرحمن الرحيم")
    assert result != "بسم الله الرحمن الرحيم"
    assert result

    # Step 3: taskeen_threshold changes the output.
    with_taskeen = m.tashkeel("بسم الله الرحمن الرحيم", taskeen_threshold=0.8)
    assert with_taskeen != result

    # Step 4: INFERENCE_ENGINE is now permanently set for this process. A
    # second explicit init() call must hit the "already initialized" branch.
    with pytest.raises(RuntimeError):
        m.init()

    # Step 5: input over CHAR_LIMIT raises via the INPUT_TOO_LONG path.
    too_long_text = "ا" * (CHAR_LIMIT + 1)
    with pytest.raises(RuntimeError):
        m.tashkeel(too_long_text, preprocessed=True)


def test_extern_error_take_message_returns_none_when_no_message():
    err = m.ExternError()
    assert err.take_message() is None
