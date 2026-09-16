# dengjen-tashkeel C API

C ABI layer over `dengjen-tashkeel` (crates/core), built on `ffi_support`, for consuming the diacritization engine from C/C++ or any other non-Rust host. Exposes `extern "C"` functions such as `dengjenTashkeelTashkeel` to diacritize a UTF-8 string and `dengjen_tashkeel_init` to load a model up front, with errors reported through an `ExternError` out-parameter and strings freed via `dengjen_tashkeel_free_string`.

Builds as the `dengjen_tashkeel_capi` cdylib.
