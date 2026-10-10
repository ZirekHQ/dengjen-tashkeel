# dengjen-tashkeel Python bindings

PyO3-based Python bindings over `dengjen-tashkeel` (crates/core), exposing a single `tashkeel(text, taskeen_threshold=None, preprocessed=None)` function that diacritizes Arabic text using a module-level inference engine initialized on import.

Importing the module locates the shared library of the pip-installed `onnxruntime` package
(`onnxruntime>=1.28,<2`, declared as a dependency) and creates the engine with the bundled model;
a missing or broken `onnxruntime` raises `RuntimeError` at import. `tashkeel` raises `ValueError`
when a sentence exceeds `CHAR_LIMIT` (12,000 characters) and `RuntimeError` for any other failure.
Don't install `onnxruntime-gpu` alongside `onnxruntime`; they share one import name.

```python
from dengjen_tashkeel_py import tashkeel

tashkeel("بسم الله الرحمن الرحيم")
```

Builds as the `dengjen_tashkeel_py` cdylib, importable from Python as `dengjen_tashkeel_py`.
