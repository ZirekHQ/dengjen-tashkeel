# dengjen-tashkeel C API

C ABI layer over `dengjen-tashkeel` (crates/core), built on `ffi_support`, for consuming the diacritization engine from C/C++ or any other non-Rust host. Exposes `extern "C"` functions such as `dengjenTashkeelTashkeel` to diacritize a UTF-8 string and `dengjen_tashkeel_init` to load a model up front, with errors reported through an `ExternError` out-parameter and strings freed via `dengjen_tashkeel_free_string`.

Builds as the `dengjen_tashkeel_capi` cdylib.

## Usage

The header exports three functions, spelled exactly as shown: `dengjen_tashkeel_init`,
`dengjenTashkeelTashkeel`, and `dengjen_tashkeel_free_string`. Errors come back through the
`ExternError` out-parameter (`code == 0` is success). Release every returned string and every
non-null `err.message` with `dengjen_tashkeel_free_string`. Without a prior
`dengjen_tashkeel_init`, the first `dengjenTashkeelTashkeel` call initializes the bundled model
lazily, after which `dengjen_tashkeel_init` returns `UNKNOWN_ERROR`.

```c
#include <stdio.h>
#include "dengjen_tashkeel.h"

int main(void) {
    ExternError err = {0};

    /* Optional: NULL loads the bundled model. Must precede the first tashkeel call. */
    dengjen_tashkeel_init(NULL, &err);
    if (err.code != 0) {
        fprintf(stderr, "%s\n", err.message);
        dengjen_tashkeel_free_string(err.message);
        return 1;
    }

    /* NULL threshold disables taskeen; false = let the library segment sentences. */
    char *out = dengjenTashkeelTashkeel("بسم الله الرحمن الرحيم", NULL, false, &err);
    if (err.code != 0) {
        fprintf(stderr, "%s\n", err.message);
        dengjen_tashkeel_free_string(err.message);
        return 1;
    }
    puts(out);
    dengjen_tashkeel_free_string(out);
    return 0;
}
```

See [`ffi_usage_example.py`](./ffi_usage_example.py) for the same flow via `ctypes`, and the
[header](./dengjen_tashkeel.h) for the error codes.
