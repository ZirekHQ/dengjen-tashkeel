use dengjen_tashkeel::{
    create_inference_engine, do_tashkeel_with, DengjenTashkeelError, DynamicInferenceEngine,
    ErrorKind, TashkeelOptions,
};
use ffi_support::{call_with_result, rust_string_to_c, ErrorCode, ExternError, FfiStr};
use once_cell::sync::OnceCell;
use std::ffi::c_char;
use std::path::PathBuf;

static INFERENCE_ENGINE: OnceCell<DynamicInferenceEngine> = OnceCell::new();

mod error_codes {
    pub const DENGJEN_TASHKEEL_INPUT_TOO_LONG: i32 = 1;
    pub const DENGJEN_TASHKEEL_INFERENCE_ERROR: i32 = 2;
    pub const DENGJEN_TASHKEEL_MODEL_LOAD_ERROR: i32 = 3;
    pub const DENGJEN_TASHKEEL_ALREADY_INITIALIZED: i32 = 4;
    pub const DENGJEN_TASHKEEL_INVALID_ARGUMENT: i32 = 5;
    pub const DENGJEN_TASHKEEL_UNKNOWN_ERROR: i32 = 99;
}

#[derive(Debug)]
struct DengjenTashkeelFFIError(i32, String);

impl From<DengjenTashkeelError> for DengjenTashkeelFFIError {
    fn from(other: DengjenTashkeelError) -> Self {
        let code = match other.kind() {
            ErrorKind::InputTooLong => error_codes::DENGJEN_TASHKEEL_INPUT_TOO_LONG,
            ErrorKind::Inference => error_codes::DENGJEN_TASHKEEL_INFERENCE_ERROR,
            ErrorKind::ModelLoad => error_codes::DENGJEN_TASHKEEL_MODEL_LOAD_ERROR,
            _ => error_codes::DENGJEN_TASHKEEL_UNKNOWN_ERROR,
        };
        Self(code, other.to_string())
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
/// `s` must be either null or a string pointer returned by
/// `dengjenTashkeelTashkeel` or stored in `ExternError.message`, and not yet
/// freed. Passing any other pointer, freeing it twice, or using `s` after this
/// call is undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dengjen_tashkeel_free_string(s: *mut c_char) {
    unsafe { ffi_support::destroy_c_string(s) }
}

/// # Safety
/// `text_ptr` must be a valid NUL-terminated string. A null `text_ptr` is
/// reported as `DENGJEN_TASHKEEL_INVALID_ARGUMENT`; invalid UTF-8 is converted lossily.
/// `taskeen_threshold` must be either null or point to a single, properly
/// aligned `c_float` that remains valid for the duration of this call.
/// Ownership of the pointee is NOT transferred: this function only reads
/// through the pointer and never frees it. The caller retains ownership
/// and is responsible for freeing it (if heap-allocated) after this call
/// returns.
/// `out_error` must be either null (in which case this call reports no
/// error and returns a null pointer) or point to a single, properly
/// aligned, writable `ExternError` valid for the duration of this call.
/// # Returns
/// A non-null return is a Rust-allocated, NUL-terminated string owned by
/// the caller, who must release it with `dengjen_tashkeel_free_string`.
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
    let mut options = TashkeelOptions::new().with_preprocessed(preprocessed);
    if let Some(threshold) = taskeen_threshold {
        options = options.with_taskeen_threshold(threshold);
    }
    call_with_result(out_error, move || run_tashkeel(text_ptr, &options))
}

fn run_tashkeel(
    text_ptr: FfiStr,
    options: &TashkeelOptions,
) -> DengjenTashkeelFFIResult<*mut c_char> {
    let text = text_ptr.into_opt_string().ok_or_else(|| {
        DengjenTashkeelFFIError(
            error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT,
            "text must not be null".to_string(),
        )
    })?;
    let engine = INFERENCE_ENGINE.get_or_try_init(|| create_inference_engine(None))?;
    let diacritized_text = do_tashkeel_with(engine, &text, options)?;
    Ok(rust_string_to_c(diacritized_text))
}

/// Options for `dengjenTashkeelTashkeelWithOptions`.
///
/// Set `struct_size` to `sizeof(DengjenTashkeelOptions)` when filling the struct. The library
/// reads only the fields that `struct_size` covers and uses defaults for the rest, so a caller
/// built against an older header, with fewer fields, keeps working when fields are appended.
/// Flag fields are `0` (off) or non-zero (on).
#[repr(C)]
pub struct DengjenTashkeelOptions {
    pub struct_size: u32,
    pub preprocessed: u8,
    pub has_taskeen_threshold: u8,
    pub taskeen_threshold: libc::c_float,
}

/// # Safety
/// `options` must be null or point to at least `struct_size` readable bytes, the first four of
/// which hold `struct_size` itself.
unsafe fn options_from_c(
    options: *const DengjenTashkeelOptions,
) -> DengjenTashkeelFFIResult<TashkeelOptions> {
    if options.is_null() {
        return Ok(TashkeelOptions::new());
    }
    let size_field = std::mem::size_of::<u32>();
    let covered = unsafe { std::ptr::read_unaligned(options.cast::<u32>()) } as usize;
    if covered < size_field {
        return Err(DengjenTashkeelFFIError(
            error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT,
            format!("options.struct_size is {covered}, expected at least {size_field}"),
        ));
    }
    let read = |offset: usize, size: usize| covered >= offset + size;
    let base = options.cast::<u8>();
    let preprocessed = std::mem::offset_of!(DengjenTashkeelOptions, preprocessed);
    let has_threshold = std::mem::offset_of!(DengjenTashkeelOptions, has_taskeen_threshold);
    let threshold = std::mem::offset_of!(DengjenTashkeelOptions, taskeen_threshold);

    let mut converted = TashkeelOptions::new();
    if read(preprocessed, 1) {
        let flag = unsafe { base.add(preprocessed).read() };
        converted = converted.with_preprocessed(flag != 0);
    }
    if read(has_threshold, 1) && read(threshold, std::mem::size_of::<libc::c_float>()) {
        let flag = unsafe { base.add(has_threshold).read() };
        if flag != 0 {
            let value =
                unsafe { std::ptr::read_unaligned(base.add(threshold).cast::<libc::c_float>()) };
            converted = converted.with_taskeen_threshold(value);
        }
    }
    Ok(converted)
}

/// # Safety
/// Same contract as `dengjenTashkeelTashkeel`, except that the settings come from `options`.
/// `options` must be either null (defaults: no taskeen, segment sentences) or point to at least
/// `struct_size` readable bytes of a `DengjenTashkeelOptions`, where `struct_size` is its first
/// field and at least 4. Fields beyond `struct_size` take their defaults.
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn dengjenTashkeelTashkeelWithOptions(
    text_ptr: FfiStr,
    options: *const DengjenTashkeelOptions,
    out_error: *mut ExternError,
) -> *mut c_char {
    let Some(out_error) = (unsafe { out_error.as_mut() }) else {
        return std::ptr::null_mut();
    };
    call_with_result(out_error, move || {
        let options = unsafe { options_from_c(options) }?;
        run_tashkeel(text_ptr, &options)
    })
}

/// # Safety
/// A repeated call, or a call after a successful `dengjenTashkeelTashkeel`,
/// reports `DENGJEN_TASHKEEL_ALREADY_INITIALIZED` and keeps the existing engine.
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

/// Loads the onnxruntime shared library at `ort_path_ptr` for this build's dynamic backend.
///
/// Only exported when built with the `ort-dylib` feature; the header guards the declaration with
/// `DENGJEN_TASHKEEL_ORT_DYLIB`. Call it before the first
/// `dengjenTashkeelTashkeel` or `dengjen_tashkeel_init`; without it the library is looked up
/// through `ORT_DYLIB_PATH` and the platform loader search path. A null `ort_path_ptr` is
/// reported as `DENGJEN_TASHKEEL_INVALID_ARGUMENT`.
///
/// # Safety
/// `out_error` must be either null (in which case this call is a silent
/// no-op) or point to a single, properly aligned, writable `ExternError`
/// valid for the duration of this call.
#[cfg(feature = "ort-dylib")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn dengjen_tashkeel_init_ort_dylib(
    ort_path_ptr: FfiStr,
    out_error: *mut ExternError,
) {
    let Some(out_error) = (unsafe { out_error.as_mut() }) else {
        return;
    };
    let ort_path = ort_path_ptr.into_opt_string().map(PathBuf::from);
    call_with_result(out_error, move || {
        let path = ort_path.ok_or_else(|| {
            DengjenTashkeelFFIError(
                error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT,
                "onnxruntime library path must not be null.".to_string(),
            )
        })?;
        dengjen_tashkeel::init_ort_dylib(path).map_err(DengjenTashkeelFFIError::from)
    })
}

fn already_initialized() -> DengjenTashkeelFFIError {
    DengjenTashkeelFFIError(
        error_codes::DENGJEN_TASHKEEL_ALREADY_INITIALIZED,
        "Inference engine is already initialized.".to_string(),
    )
}

fn do_init_library(model_path: Option<PathBuf>) -> DengjenTashkeelFFIResult<()> {
    if INFERENCE_ENGINE.get().is_some() {
        return Err(already_initialized());
    }
    let engine = create_inference_engine(model_path)?;
    INFERENCE_ENGINE
        .set(engine)
        .map_err(|_| already_initialized())
}

#[cfg(all(test, feature = "bundled-model"))]
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
    fn null_text_ptr_is_reported_as_invalid_argument() {
        let mut out_error = new_out_error();
        let taskeen_threshold: *const libc::c_float = std::ptr::null();
        let null_text_ptr = unsafe { FfiStr::from_raw(std::ptr::null()) };

        let result_ptr = unsafe {
            dengjenTashkeelTashkeel(null_text_ptr, taskeen_threshold, true, &mut out_error)
        };

        assert_eq!(
            out_error.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT
        );
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

        assert_eq!(
            out_error.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_ALREADY_INITIALIZED
        );
    }

    #[test]
    fn dengjen_tashkeel_init_with_bad_model_path_reports_model_load_error() {
        let error = create_inference_engine(Some(PathBuf::from("/nonexistent/path/to/model.onnx")))
            .err()
            .expect("a missing model file must not load");

        assert_eq!(
            DengjenTashkeelFFIError::from(error).0,
            error_codes::DENGJEN_TASHKEEL_MODEL_LOAD_ERROR
        );
    }

    #[test]
    fn dengjen_tashkeel_init_with_malformed_model_file_reports_inference_error() {
        let mut malformed = tempfile::NamedTempFile::new().unwrap();
        malformed
            .write_all(b"this is not a valid onnx model")
            .unwrap();
        let error = create_inference_engine(Some(malformed.path().to_path_buf()))
            .err()
            .expect("a malformed model file must not load");

        assert_eq!(
            DengjenTashkeelFFIError::from(error).0,
            error_codes::DENGJEN_TASHKEEL_INFERENCE_ERROR
        );
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

        assert_eq!(
            out_error.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_INPUT_TOO_LONG
        );
        assert!(result_ptr.is_null());
    }

    #[test]
    fn free_string_accepts_null_and_releases_a_returned_string() {
        unsafe { dengjen_tashkeel_free_string(std::ptr::null_mut()) };

        let owned = rust_string_to_c("owned".to_string());
        unsafe { dengjen_tashkeel_free_string(owned) };
    }

    #[test]
    fn ffi_error_message_is_the_core_error_text() {
        let core_error = DengjenTashkeelError::InputTooLong(7);
        let expected = core_error.to_string();

        let ffi_error = DengjenTashkeelFFIError::from(core_error);

        assert_eq!(ffi_error.0, error_codes::DENGJEN_TASHKEEL_INPUT_TOO_LONG);
        assert_eq!(ffi_error.1, expected);
    }

    #[test]
    fn model_load_error_maps_to_its_own_code() {
        let io_error = std::io::Error::from(std::io::ErrorKind::NotFound);

        let ffi_error =
            DengjenTashkeelFFIError::from(DengjenTashkeelError::ModelLoadError(io_error));

        assert_eq!(ffi_error.0, error_codes::DENGJEN_TASHKEEL_MODEL_LOAD_ERROR);
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

    fn c_options(
        struct_size: usize,
        preprocessed: u8,
        threshold: Option<f32>,
    ) -> DengjenTashkeelOptions {
        DengjenTashkeelOptions {
            struct_size: struct_size as u32,
            preprocessed,
            has_taskeen_threshold: threshold.is_some() as u8,
            taskeen_threshold: threshold.unwrap_or_default(),
        }
    }

    #[test]
    fn missing_options_default_to_no_taskeen_and_segmented_input() {
        let options = unsafe { options_from_c(std::ptr::null()) }.unwrap();

        assert_eq!(options, TashkeelOptions::new());
    }

    #[test]
    fn c_options_carry_the_threshold_and_preprocessed_flag() {
        let size = std::mem::size_of::<DengjenTashkeelOptions>();

        let raw = c_options(size, 1, Some(0.8));

        let options = unsafe { options_from_c(&raw) }.unwrap();

        assert_eq!(
            options,
            TashkeelOptions::new()
                .with_preprocessed(true)
                .with_taskeen_threshold(0.8)
        );
    }

    #[test]
    fn c_options_without_a_threshold_ignore_the_threshold_field() {
        let size = std::mem::size_of::<DengjenTashkeelOptions>();
        let mut raw = c_options(size, 0, None);
        raw.taskeen_threshold = 0.9;

        let options = unsafe { options_from_c(&raw) }.unwrap();

        assert_eq!(options.taskeen_threshold, None);
        assert!(!options.preprocessed);
    }

    #[test]
    fn c_options_from_a_newer_header_with_a_larger_struct_size_are_accepted() {
        let larger = std::mem::size_of::<DengjenTashkeelOptions>() + 16;

        let raw = c_options(larger, 1, None);

        let options = unsafe { options_from_c(&raw) };

        assert!(options.unwrap().preprocessed);
    }

    #[test]
    fn c_options_from_an_older_header_use_defaults_for_the_fields_they_lack() {
        let older = c_options(
            std::mem::offset_of!(DengjenTashkeelOptions, preprocessed) + 1,
            1,
            Some(0.5),
        );

        let options = unsafe { options_from_c(&older) }.unwrap();

        assert!(options.preprocessed);
        assert_eq!(options.taskeen_threshold, None);
    }

    #[test]
    fn c_options_that_do_not_cover_struct_size_itself_are_an_invalid_argument() {
        let raw = c_options(2, 1, None);

        let error = unsafe { options_from_c(&raw) }.unwrap_err();

        assert_eq!(error.0, error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT);
        assert!(error.1.contains("struct_size"));
    }

    #[test]
    fn with_options_entry_point_reports_a_zero_struct_size_before_touching_the_engine() {
        let text = CString::new("بسم الله").unwrap();
        let raw = c_options(0, 0, None);
        let mut out_error = new_out_error();

        let result_ptr = unsafe {
            dengjenTashkeelTashkeelWithOptions(FfiStr::from_cstr(&text), &raw, &mut out_error)
        };

        assert_eq!(
            out_error.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT
        );
        assert!(result_ptr.is_null());
    }

    #[test]
    fn with_options_entry_point_reports_null_text_as_invalid_argument() {
        let mut out_error = new_out_error();
        let null_text_ptr = unsafe { FfiStr::from_raw(std::ptr::null()) };

        let result_ptr = unsafe {
            dengjenTashkeelTashkeelWithOptions(null_text_ptr, std::ptr::null(), &mut out_error)
        };

        assert_eq!(
            out_error.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT
        );
        assert!(result_ptr.is_null());
    }

    #[test]
    fn with_options_entry_point_matches_the_positional_one() {
        let text = CString::new("بسم الله الرحمن الرحيم").unwrap();
        let size = std::mem::size_of::<DengjenTashkeelOptions>();
        let raw = c_options(size, 1, Some(0.95));
        let threshold = 0.95_f32;
        let mut positional_error = new_out_error();
        let mut options_error = new_out_error();

        let positional = unsafe {
            dengjenTashkeelTashkeel(
                FfiStr::from_cstr(&text),
                &threshold,
                true,
                &mut positional_error,
            )
        };
        let with_options = unsafe {
            dengjenTashkeelTashkeelWithOptions(FfiStr::from_cstr(&text), &raw, &mut options_error)
        };

        assert!(positional_error.get_code().is_success());
        assert!(options_error.get_code().is_success());
        let read = |ptr| unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
        assert_eq!(read(positional), read(with_options));
        unsafe {
            dengjen_tashkeel_free_string(positional);
            dengjen_tashkeel_free_string(with_options);
        };
    }

    // Java (TashkeelLib.EXTERN_ERROR) hand-declares this layout: int32 code at 0, pointer at 8.
    #[test]
    fn extern_error_layout_matches_the_one_the_java_binding_declares() {
        let error = ExternError::new_error(ErrorCode::new(7), "boom");
        let base = &error as *const ExternError as *const u8;

        let code = unsafe { std::ptr::read_unaligned(base as *const i32) };
        let message = unsafe { std::ptr::read_unaligned(base.add(8) as *const *const c_char) };

        assert_eq!(std::mem::size_of::<ExternError>(), 16);
        assert_eq!(std::mem::align_of::<ExternError>(), 8);
        assert_eq!(code, 7);
        assert_eq!(unsafe { CStr::from_ptr(message) }.to_str().unwrap(), "boom");
        unsafe { error.manually_release() };
    }
}

#[cfg(all(test, any(feature = "ort-dylib", not(feature = "bundled-model"))))]
mod build_variant_tests {
    use super::*;
    #[cfg(not(feature = "bundled-model"))]
    #[test]
    fn init_without_a_model_path_reports_model_load_error_when_nothing_is_bundled() {
        let mut err = ExternError::success();
        unsafe { dengjen_tashkeel_init(FfiStr::from_raw(std::ptr::null()), &mut err) };
        assert_eq!(
            err.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_MODEL_LOAD_ERROR
        );
        unsafe { dengjen_tashkeel_free_string(err.get_raw_message() as *mut c_char) };
    }

    #[cfg(feature = "ort-dylib")]
    #[test]
    fn init_ort_dylib_rejects_null_and_missing_paths() {
        let mut err = ExternError::success();
        unsafe { dengjen_tashkeel_init_ort_dylib(FfiStr::from_raw(std::ptr::null()), &mut err) };
        assert_eq!(
            err.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_INVALID_ARGUMENT
        );
        unsafe { dengjen_tashkeel_free_string(err.get_raw_message() as *mut c_char) };

        let missing = std::ffi::CString::new("/nonexistent/libonnxruntime.so").unwrap();
        let mut err = ExternError::success();
        unsafe {
            dengjen_tashkeel_init_ort_dylib(FfiStr::from_cstr(&missing), &mut err);
        }
        assert_eq!(
            err.get_code().code(),
            error_codes::DENGJEN_TASHKEEL_INFERENCE_ERROR
        );
        unsafe { dengjen_tashkeel_free_string(err.get_raw_message() as *mut c_char) };
    }
}
