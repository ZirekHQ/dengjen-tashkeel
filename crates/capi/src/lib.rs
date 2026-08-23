use ffi_support::{
    call_with_result, define_string_destructor, rust_string_to_c, ErrorCode, ExternError, FfiStr,
};
use libtashkeel_core::{
    create_inference_engine, do_tashkeel, DynamicInferenceEngine, LibtashkeelError,
};
use once_cell::sync::OnceCell;
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::Once;

static INFERENCE_ENGINE: OnceCell<DynamicInferenceEngine> = OnceCell::new();
static INIT_LIBTASHKEEL: Once = Once::new();

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

define_string_destructor!(libtashkeel_free_string);

/// # Safety
/// The `taskeen_threshold_ptr` should be properly alighned as `c_float`
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn libtashkeelTashkeel(
    text_ptr: FfiStr,
    taskeen_threshold: *const libc::c_float,
    preprocessed: bool,
    out_error: &mut ExternError,
) -> *mut c_char {
    let text = text_ptr.into_string();
    let taskeen_threshold = unsafe {
        let retval = taskeen_threshold.as_ref().copied();
        libc::free(taskeen_threshold as *mut libc::c_void);
        retval
    };
    call_with_result(out_error, move || {
        INIT_LIBTASHKEEL.call_once(|| {
            do_init_library(None).unwrap();
        });
        let diacritized_text = ffi_do_tashkeel(
            INFERENCE_ENGINE.get().unwrap(),
            &text,
            taskeen_threshold,
            preprocessed,
        )?;
        let retval = rust_string_to_c(diacritized_text);
        Ok::<*mut c_char, LibtashkeelFFIError>(retval)
    })
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn libtashkeel_init(model_path_ptr: FfiStr, out_error: &mut ExternError) {
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
    INIT_LIBTASHKEEL.call_once(|| ());
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
        // Step 1: initialize via libtashkeel_init with a null path (-> None
        // -> bundled default model) FIRST, matching the intended real usage
        // (init once, then call tashkeel). This deliberately avoids ever
        // exercising libtashkeelTashkeel's own lazy-init fallback
        // (INIT_LIBTASHKEEL.call_once(|| { do_init_library(None).unwrap() })):
        // do_init_library's first line unconditionally calls
        // INIT_LIBTASHKEEL.call_once(|| ()) again on the SAME Once -- a
        // reentrant call from within that Once's own initialization closure,
        // which per std::sync::Once's documented behavior deadlocks. This is
        // a genuine pre-existing bug in the production lazy-init fallback
        // path (confirmed by reproducing the hang directly against this
        // repo), not something to fix as part of test coverage -- real
        // consumers always call libtashkeel_init before libtashkeelTashkeel,
        // so this dormant path has likely never been hit in practice. Flag
        // it to the user; don't fix it here.
        let mut out_error = new_out_error();
        let null_path = unsafe { FfiStr::from_raw(std::ptr::null()) };
        libtashkeel_init(null_path, &mut out_error);
        assert!(out_error.get_code().is_success());

        // Step 2: now that INFERENCE_ENGINE is set, libtashkeelTashkeel's own
        // call_once sees INIT_LIBTASHKEEL already complete and skips its
        // closure entirely -- no reentrant call, no deadlock.
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

        // Step 3: invalid UTF-8 lossily converts instead of erroring.
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

        // Step 4: INFERENCE_ENGINE is now permanently set for this process.
        // A second call to libtashkeel_init (even with valid arguments)
        // must hit the "already initialized" branch.
        let mut out_error = new_out_error();
        let null_path = unsafe { FfiStr::from_raw(std::ptr::null()) };
        libtashkeel_init(null_path, &mut out_error);
        assert_eq!(out_error.get_code().code(), ErrorCodes::UNKNOWN_ERROR);

        // Step 5: bad model path/file report INFERENCE_ERROR regardless of
        // the above state -- create_inference_engine fails before
        // do_init_library would ever reach the .set() call.
        let bad_path = CString::new("/nonexistent/path/to/model.onnx").unwrap();
        let mut out_error = new_out_error();
        libtashkeel_init(FfiStr::from_cstr(&bad_path), &mut out_error);
        assert_eq!(out_error.get_code().code(), ErrorCodes::INFERENCE_ERROR);

        let mut malformed = tempfile::NamedTempFile::new().unwrap();
        malformed
            .write_all(b"this is not a valid onnx model")
            .unwrap();
        let path = CString::new(malformed.path().to_str().unwrap()).unwrap();
        let mut out_error = new_out_error();
        libtashkeel_init(FfiStr::from_cstr(&path), &mut out_error);
        assert_eq!(out_error.get_code().code(), ErrorCodes::INFERENCE_ERROR);
    }
}
