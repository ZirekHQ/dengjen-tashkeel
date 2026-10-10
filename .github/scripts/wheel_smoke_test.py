"""Exercises the installed dengjen_tashkeel_py wheel's public surface."""
from dengjen_tashkeel_py import tashkeel

TEXT = "بسم الله الرحمن الرحيم"
CHAR_LIMIT = 12000

default = tashkeel(TEXT)
assert default and default != TEXT, default

with_taskeen = tashkeel(TEXT, taskeen_threshold=0.8)
assert with_taskeen and with_taskeen != default, with_taskeen

preprocessed = tashkeel(TEXT, preprocessed=True)
assert isinstance(preprocessed, str) and preprocessed, preprocessed

try:
    tashkeel("ا" * (CHAR_LIMIT + 1))
except ValueError as err:
    assert "too long" in str(err).lower(), err
else:
    raise AssertionError("over-limit input did not raise ValueError")

print(default)
