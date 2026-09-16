# dengjen-tashkeel CLI

Command-line frontend for `dengjen-tashkeel` (crates/core): reads Arabic text from a file or stdin, diacritizes it via the core engine, and writes the result to a file or stdout. Runs interactively by default when no `--input-file`/`--output-file` is given, and supports `--taskeen`/`--prob` to substitute a sukoon for case-ending diacritics the model is unsure about, plus `--onnx` to load a custom model instead of the bundled one.

Installed as the `dengjen-tashkeel` binary.
