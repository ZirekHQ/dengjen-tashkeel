/* Generated with cbindgen:0.29.4 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define INPUT_TOO_LONG 1

#define INFERENCE_ERROR 2

#define MODEL_LOAD_ERROR 3

#define UNKNOWN_ERROR 99

typedef const char *FfiStr;

typedef int32_t ErrorCode;
#define ErrorCode_SUCCESS 0
#define ErrorCode_PANIC -1
#define ErrorCode_INVALID_HANDLE -1000

typedef struct ExternError {
  ErrorCode code;
  char *message;
} ExternError;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

void libtashkeel_free_string(char *s);

char *libtashkeelTashkeel(FfiStr text_ptr,
                          const float *taskeen_threshold,
                          bool preprocessed,
                          struct ExternError *out_error);

void libtashkeel_init(FfiStr model_path_ptr, struct ExternError *out_error);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
