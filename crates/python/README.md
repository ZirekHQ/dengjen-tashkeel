# dengjen-tashkeel Python bindings

PyO3-based Python bindings over `dengjen-tashkeel` (crates/core), exposing a single `tashkeel(text, taskeen_threshold=None, preprocessed=None)` function that diacritizes Arabic text using a module-level inference engine initialized on import.

Builds as the `dengjen_tashkeel_py` cdylib, importable from Python as `dengjen_tashkeel_py`.
