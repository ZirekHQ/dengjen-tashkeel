# dengjen-tashkeel Python bindings

PyO3-based Python bindings over `dengjen-tashkeel` (crates/core), exposing `tashkeel(text, taskeen_threshold=None, preprocessed=None)`, which diacritizes Arabic text using a module-level inference engine, and `init(model_path=None)`, which loads that engine up front or from a custom ONNX file.

Importing the module locates the shared library of the pip-installed `onnxruntime` package
(`onnxruntime>=1.28,<2`, declared as a dependency); a missing or broken `onnxruntime` raises
`RuntimeError` at import. The engine loads on the first `tashkeel` call with the bundled model, or
earlier through `init`, which raises `RuntimeError` if it is already loaded. `tashkeel` raises
`ValueError` when a sentence exceeds `CHAR_LIMIT` (12,000 characters) and `RuntimeError` for any
other failure, including a model that cannot be loaded.
Don't install `onnxruntime-gpu` alongside `onnxruntime`; they share one import name.

```python
from dengjen_tashkeel_py import tashkeel

tashkeel("بسم الله الرحمن الرحيم")
```

Builds as the `dengjen_tashkeel_py` cdylib, importable from Python as `dengjen_tashkeel_py`.

## Support

This crate is part of [Zirek](https://github.com/ZirekHQ). 💖 [Support on Open Collective](https://opencollective.com/zirek)
