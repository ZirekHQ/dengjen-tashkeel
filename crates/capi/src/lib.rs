use dengjen_tashkeel::{
    create_inference_engine, do_tashkeel, DynamicInferenceEngine, LibtashkeelError,
};
use ffi_support::{call_with_result, rust_string_to_c, ErrorCode, ExternError, FfiStr};
use once_cell::sync::OnceCell;
use std::ffi::c_char;
use std::path::PathBuf;

static INFERENCE_ENGINE: OnceCell<DynamicInferenceEngine> = OnceCell::new();

#[allow(non_snake_case)]
mod ErrorCodes {
    pub const INPUT_TOO_LONG: i32 = 1;
    pub const INFERENCE_ERROR: i32 = 2;
    pub const MODEL_LOAD_ERROR: i32 = 3;
    pub const UNKNOWN_ERROR: i32 = 99;
}

#[derive(Debug)]
struct LibtashkeelFFIError(i32, String);

impl From<LibtashkeelError> for LibtashkeelFFIError {
    fn from(other: LibtashkeelError) -> Self {
        let (code, message) = match other {
            LibtashkeelError::InputTooLong(max_len) => (
                ErrorCodes::INPUT_TOO_LONG,
                format!("Input too long. Max length {}", max_len),
            ),
            LibtashkeelError::InferenceError(msg) => (ErrorCodes::INFERENCE_ERROR, msg),
            LibtashkeelError::ModelLoadError(e) => (ErrorCodes::MODEL_LOAD_ERROR, e.to_string()),
        };
        Self(code, message)
    }
}

impl From<LibtashkeelFFIError> for ExternError {
    fn from(other: LibtashkeelFFIError) -> Self {
        let err_code = ErrorCode::new(other.0);
        ExternError::new_error(err_code, other.1)
    }
}

type LibtashkeelFFIResult<T> = Result<T, LibtashkeelFFIError>;

/// # Safety
/// `s` must be either null or a pointer previously returned by
/// `libtashkeelTashkeel`, not yet freed. Passing any other pointer, freeing
/// it twice, or using `s` after this call is undefined behavior.
///
/// Hand-written (rather than `define_string_destructor!`) purely so
/// cbindgen -- which parses source syntactically and never expands foreign
/// macros -- can see this symbol and declare it in libtashkeel.h; behavior
/// is identical to what that macro would generate.
#[no_mangle]
pub unsafe extern "C" fn libtashkeel_free_string(s: *mut c_char) {
    unsafe { ffi_support::destroy_c_string(s) }
}

/// # Safety
/// `taskeen_threshold` must be either null or point to a single, properly
/// aligned `c_float` that remains valid for the duration of this call.
/// Ownership of the pointee is NOT transferred: this function only reads
/// through the pointer and never frees it. The caller retains ownership
/// and is responsible for freeing it (if heap-allocated) after this call
/// returns.
/// `out_error` must be either null (in which case this call reports no
/// error and returns a null pointer) or point to a single, properly
/// aligned, writable `ExternError` valid for the duration of this call.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn libtashkeelTashkeel(
    text_ptr: FfiStr,
    taskeen_threshold: *const libc::c_float,
    preprocessed: bool,
    out_error: *mut ExternError,
) -> *mut c_char {
    let Some(out_error) = (unsafe { out_error.as_mut() }) else {
        return std::ptr::null_mut();
    };
    let taskeen_threshold = unsafe { taskeen_threshold.as_ref().copied() };
    call_with_result(out_error, move || {
        // Deliberately inside this closure: text_ptr.into_string() panics on
        // a null pointer, and call_with_result's catch_unwind converts that
        // panic into a clean ExternError (code PANIC = -1) instead of
        // letting it unwind past this extern "C" boundary and abort.
        let text = text_ptr.into_string();
        let engine = INFERENCE_ENGINE.get_or_try_init(|| create_inference_engine(None))?;
        let diacritized_text = ffi_do_tashkeel(engine, &text, taskeen_threshold, preprocessed)?;
        let retval = rust_string_to_c(diacritized_text);
        Ok::<*mut c_char, LibtashkeelFFIError>(retval)
    })
}

/// # Safety
/// `out_error` must be either null (in which case this call is a silent
/// no-op) or point to a single, properly aligned, writable `ExternError`
/// valid for the duration of this call.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn libtashkeel_init(model_path_ptr: FfiStr, out_error: *mut ExternError) {
    let Some(out_error) = (unsafe { out_error.as_mut() }) else {
        return;
    };
    let model_path = model_path_ptr.into_opt_string().map(PathBuf::from);
    call_with_result(out_error, move || do_init_library(model_path))
}

fn ffi_do_tashkeel(
    model: &DynamicInferenceEngine,
    text: &str,
    taskeen_threshold: Option<f32>,
    preprocessed: bool,
) -> LibtashkeelFFIResult<String> {
    Ok(do_tashkeel(model, text, taskeen_threshold, preprocessed)?)
}

fn do_init_library(model_path: Option<PathBuf>) -> LibtashkeelFFIResult<()> {
    let engine = create_inference_engine(model_path)?;
    if INFERENCE_ENGINE.set(engine).is_err() {
        Err(LibtashkeelFFIError(
            ErrorCodes::UNKNOWN_ERROR,
            "Unexpected error. Failed to init global inference_engine instance.".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::io::Write;

    fn new_out_error() -> ExternError {
        ExternError::success()
    }

    #[test]
    fn tashkeel_lifecycle_and_error_paths() {
        // Step 1: call libtashkeelTashkeel directly, with NO prior
        // libtashkeel_init call. INFERENCE_ENGINE.get_or_try_init lazily
        // creates the engine from the bundled default model right here.
        // (This exact sequence used to deadlock before #13 was fixed: the
        // old lazy-init path re-entered the same std::sync::Once it was
        // already running inside.)
        let text = CString::new("بسم الله الرحمن الرحيم").unwrap();
        let mut out_error = new_out_error();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();

        let result_ptr = unsafe {
            libtashkeelTashkeel(
                FfiStr::from_cstr(&text),
                taskeen_threshold,
                true,
                &mut out_error,
            )
        };

        assert!(out_error.get_code().is_success());
        assert!(!result_ptr.is_null());
        let diacritized = unsafe { CStr::from_ptr(result_ptr) }
            .to_str()
            .unwrap()
            .to_string();
        assert_ne!(diacritized, "بسم الله الرحمن الرحيم");
        unsafe { libtashkeel_free_string(result_ptr) };

        // Step 1b: a null text_ptr no longer aborts the process. into_string()
        // panics on it, but that panic now happens inside call_with_result's
        // closure, so its catch_unwind converts it into a clean ExternError
        // (code PANIC = -1) instead of unwinding past this extern "C" frame.
        let mut out_error = new_out_error();
        let null_text_ptr = unsafe { FfiStr::from_raw(std::ptr::null()) };
        let result_ptr =
            unsafe { libtashkeelTashkeel(null_text_ptr, taskeen_threshold, true, &mut out_error) };
        assert_eq!(out_error.get_code(), ErrorCode::PANIC);
        assert!(result_ptr.is_null());

        // Step 2: invalid UTF-8 lossily converts instead of erroring.
        let invalid_utf8 = CString::new(vec![0xFFu8, 0xFEu8]).unwrap();
        let mut out_error = new_out_error();
        let result_ptr = unsafe {
            libtashkeelTashkeel(
                FfiStr::from_cstr(&invalid_utf8),
                taskeen_threshold,
                true,
                &mut out_error,
            )
        };
        assert!(
            out_error.get_code().is_success(),
            "invalid UTF-8 should be lossily converted, not rejected"
        );
        assert!(!result_ptr.is_null());
        unsafe { libtashkeel_free_string(result_ptr) };

        // Step 3: INFERENCE_ENGINE is now permanently set for this process
        // (lazily, from Step 1). An explicit libtashkeel_init call must hit
        // the "already initialized" branch.
        let mut out_error = new_out_error();
        let null_path = unsafe { FfiStr::from_raw(std::ptr::null()) };
        unsafe { libtashkeel_init(null_path, &mut out_error) };
        assert_eq!(out_error.get_code().code(), ErrorCodes::UNKNOWN_ERROR);

        // Step 4: bad model path/file report INFERENCE_ERROR regardless of
        // the above state -- create_inference_engine fails before
        // do_init_library would ever reach the .set() call.
        let bad_path = CString::new("/nonexistent/path/to/model.onnx").unwrap();
        let mut out_error = new_out_error();
        unsafe { libtashkeel_init(FfiStr::from_cstr(&bad_path), &mut out_error) };
        assert_eq!(out_error.get_code().code(), ErrorCodes::INFERENCE_ERROR);

        let mut malformed = tempfile::NamedTempFile::new().unwrap();
        malformed
            .write_all(b"this is not a valid onnx model")
            .unwrap();
        let path = CString::new(malformed.path().to_str().unwrap()).unwrap();
        let mut out_error = new_out_error();
        unsafe { libtashkeel_init(FfiStr::from_cstr(&path), &mut out_error) };
        assert_eq!(out_error.get_code().code(), ErrorCodes::INFERENCE_ERROR);

        // Step 5: input over CHAR_LIMIT returns INPUT_TOO_LONG.
        let too_long_text = "ا".repeat(dengjen_tashkeel::CHAR_LIMIT + 1);
        let too_long_cstring = CString::new(too_long_text).unwrap();
        let mut out_error = new_out_error();
        let result_ptr = unsafe {
            libtashkeelTashkeel(
                FfiStr::from_cstr(&too_long_cstring),
                taskeen_threshold,
                true,
                &mut out_error,
            )
        };
        assert_eq!(out_error.get_code().code(), ErrorCodes::INPUT_TOO_LONG);
        assert!(result_ptr.is_null());
    }

    #[test]
    fn null_out_error_is_a_safe_no_op_not_ub() {
        let text = CString::new("بسم الله").unwrap();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();

        let result_ptr = unsafe {
            libtashkeelTashkeel(
                FfiStr::from_cstr(&text),
                taskeen_threshold,
                true,
                std::ptr::null_mut(),
            )
        };
        assert!(result_ptr.is_null());

        let null_path = unsafe { FfiStr::from_raw(std::ptr::null()) };
        unsafe { libtashkeel_init(null_path, std::ptr::null_mut()) };
    }
}
