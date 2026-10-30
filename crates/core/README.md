# dengjen-tashkeel (core)

Core Rust library for Arabic diacritic (tashkeel) restoration: tokenizes input text, runs it through an ONNX inference engine, and reconstructs the diacritized output. Exposes `create_inference_engine` to load the bundled or a custom ONNX model, and `do_tashkeel` to diacritize text, with `DengjenTashkeelError` covering input-length, inference, and model-load failures.

Consumed by the other crates in this workspace (`cli`, `capi`, `python`), which each wrap this engine for a different host environment, and usable directly as a Rust dependency.

## Support

This crate is part of [Zirek](https://github.com/ZirekHQ). 💖 [Support on Open Collective](https://opencollective.com/zirek)
