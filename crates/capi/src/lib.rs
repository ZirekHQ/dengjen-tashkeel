use dengjen_tashkeel::{
    create_inference_engine, do_tashkeel, DengjenTashkeelError, DynamicInferenceEngine,
};
use ffi_support::{call_with_result, rust_string_to_c, ErrorCode, ExternError, FfiStr};
use once_cell::sync::OnceCell;
use std::ffi::c_char;
use std::path::PathBuf;

static INFERENCE_ENGINE: OnceCell<DynamicInferenceEngine> = OnceCell::new();

mod error_codes {
    pub const INPUT_TOO_LONG: i32 = 1;
    pub const INFERENCE_ERROR: i32 = 2;
    pub const MODEL_LOAD_ERROR: i32 = 3;
    pub const UNKNOWN_ERROR: i32 = 99;
}

#[derive(Debug)]
struct DengjenTashkeelFFIError(i32, String);

impl From<DengjenTashkeelError> for DengjenTashkeelFFIError {
    fn from(other: DengjenTashkeelError) -> Self {
        let (code, message) = match other {
            DengjenTashkeelError::InputTooLong(max_len) => (
                error_codes::INPUT_TOO_LONG,
                format!("Input too long. Max length {}", max_len),
            ),
            DengjenTashkeelError::InferenceError(msg) => (error_codes::INFERENCE_ERROR, msg),
            DengjenTashkeelError::ModelLoadError(e) => {
                (error_codes::MODEL_LOAD_ERROR, e.to_string())
            }
        };
        Self(code, message)
    }
}

impl From<DengjenTashkeelFFIError> for ExternError {
    fn from(other: DengjenTashkeelFFIError) -> Self {
        let err_code = ErrorCode::new(other.0);
        ExternError::new_error(err_code, other.1)
    }
}

type DengjenTashkeelFFIResult<T> = Result<T, DengjenTashkeelFFIError>;

/// # Safety
/// `s` must be either null or one such pointer, not yet freed. Passing any
/// other pointer, freeing it twice, or using `s` after this call is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dengjen_tashkeel_free_string(s: *mut c_char) {
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
pub unsafe extern "C" fn dengjenTashkeelTashkeel(
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
        let text = text_ptr.into_string();
        let engine = INFERENCE_ENGINE.get_or_try_init(|| create_inference_engine(None))?;
        let diacritized_text = ffi_do_tashkeel(engine, &text, taskeen_threshold, preprocessed)?;
        let retval = rust_string_to_c(diacritized_text);
        Ok::<*mut c_char, DengjenTashkeelFFIError>(retval)
    })
}

/// # Safety
/// `out_error` must be either null (in which case this call is a silent
/// no-op) or point to a single, properly aligned, writable `ExternError`
/// valid for the duration of this call.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn dengjen_tashkeel_init(
    model_path_ptr: FfiStr,
    out_error: *mut ExternError,
) {
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
) -> DengjenTashkeelFFIResult<String> {
    Ok(do_tashkeel(model, text, taskeen_threshold, preprocessed)?)
}

fn do_init_library(model_path: Option<PathBuf>) -> DengjenTashkeelFFIResult<()> {
    let engine = create_inference_engine(model_path)?;
    if INFERENCE_ENGINE.set(engine).is_err() {
        Err(DengjenTashkeelFFIError(
            error_codes::UNKNOWN_ERROR,
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
    fn tashkeel_lazily_initializes_engine_and_diacritizes_without_prior_init() {
        let text = CString::new("بسم الله الرحمن الرحيم").unwrap();
        let mut out_error = new_out_error();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();

        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(
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
        unsafe { dengjen_tashkeel_free_string(result_ptr) };
    }

    #[test]
    fn null_text_ptr_is_caught_as_panic_not_ub() {
        let mut out_error = new_out_error();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();
        let null_text_ptr = unsafe { FfiStr::from_raw(std::ptr::null()) };

        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(null_text_ptr, taskeen_threshold, true, &mut out_error)
        };

        assert_eq!(out_error.get_code(), ErrorCode::PANIC);
        assert!(result_ptr.is_null());
    }

    #[test]
    fn invalid_utf8_is_lossily_converted_not_rejected() {
        let invalid_utf8 = CString::new(vec![0xFFu8, 0xFEu8]).unwrap();
        let mut out_error = new_out_error();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();

        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(
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
        unsafe { dengjen_tashkeel_free_string(result_ptr) };
    }

    #[test]
    fn dengjen_tashkeel_init_reports_already_initialized_when_engine_exists() {
        let text = CString::new("بسم الله").unwrap();
        let mut warm_up_error = new_out_error();
        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(
                FfiStr::from_cstr(&text),
                std::ptr::null(),
                true,
                &mut warm_up_error,
            )
        };
        assert!(
            warm_up_error.get_code().is_success(),
            "warm-up must succeed for this test's premise (engine already initialized)"
        );
        if !result_ptr.is_null() {
            unsafe { dengjen_tashkeel_free_string(result_ptr) };
        }

        let mut out_error = new_out_error();
        let null_path = unsafe { FfiStr::from_raw(std::ptr::null()) };
        unsafe { dengjen_tashkeel_init(null_path, &mut out_error) };

        assert_eq!(out_error.get_code().code(), error_codes::UNKNOWN_ERROR);
    }

    #[test]
    fn dengjen_tashkeel_init_with_bad_model_path_reports_inference_error() {
        let bad_path = CString::new("/nonexistent/path/to/model.onnx").unwrap();
        let mut out_error = new_out_error();

        unsafe { dengjen_tashkeel_init(FfiStr::from_cstr(&bad_path), &mut out_error) };

        assert_eq!(out_error.get_code().code(), error_codes::INFERENCE_ERROR);
    }

    #[test]
    fn dengjen_tashkeel_init_with_malformed_model_file_reports_inference_error() {
        let mut malformed = tempfile::NamedTempFile::new().unwrap();
        malformed
            .write_all(b"this is not a valid onnx model")
            .unwrap();
        let path = CString::new(malformed.path().to_str().unwrap()).unwrap();
        let mut out_error = new_out_error();

        unsafe { dengjen_tashkeel_init(FfiStr::from_cstr(&path), &mut out_error) };

        assert_eq!(out_error.get_code().code(), error_codes::INFERENCE_ERROR);
    }

    #[test]
    fn tashkeel_over_char_limit_reports_input_too_long() {
        let too_long_text = "ا".repeat(dengjen_tashkeel::CHAR_LIMIT + 1);
        let too_long_cstring = CString::new(too_long_text).unwrap();
        let mut out_error = new_out_error();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();

        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(
                FfiStr::from_cstr(&too_long_cstring),
                taskeen_threshold,
                true,
                &mut out_error,
            )
        };

        assert_eq!(out_error.get_code().code(), error_codes::INPUT_TOO_LONG);
        assert!(result_ptr.is_null());
    }

    #[test]
    fn null_out_error_is_a_safe_no_op_not_ub() {
        let text = CString::new("بسم الله").unwrap();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();

        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(
                FfiStr::from_cstr(&text),
                taskeen_threshold,
                true,
                std::ptr::null_mut(),
            )
        };
        assert!(result_ptr.is_null());

        let null_path = unsafe { FfiStr::from_raw(std::ptr::null()) };
        unsafe { dengjen_tashkeel_init(null_path, std::ptr::null_mut()) };
    }
}
