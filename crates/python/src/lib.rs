#[cfg(feature = "ort-dylib")]
use dengjen_tashkeel::init_ort_dylib;
use dengjen_tashkeel::{
    create_inference_engine, do_tashkeel, DengjenTashkeelError, DynamicInferenceEngine,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyModule;

static INFERENCE_ENGINE: PyOnceLock<DynamicInferenceEngine> = PyOnceLock::new();

fn to_py_err(error: DengjenTashkeelError) -> PyErr {
    match error {
        DengjenTashkeelError::InputTooLong(max_len) => {
            PyValueError::new_err(format!("Input too long. Max length {max_len}"))
        }
        other => PyRuntimeError::new_err(format!("Failed to diacritize text. Caused by: {other}")),
    }
}

#[pyfunction]
#[pyo3(signature = (text, taskeen_threshold=None, preprocessed=None))]
fn tashkeel(
    py: Python,
    text: String,
    taskeen_threshold: Option<f32>,
    preprocessed: Option<bool>,
) -> PyResult<String> {
    let preprocessed = preprocessed.unwrap_or_default();
    let engine = match INFERENCE_ENGINE.get(py) {
        Some(eng) => eng,
        None => {
            let error =
                PyRuntimeError::new_err("Failed to retrieve inference engine global instance");
            return Err(error);
        }
    };
    py.detach(|| do_tashkeel(engine, &text, taskeen_threshold, preprocessed))
        .map_err(to_py_err)
}

#[cfg(feature = "ort-dylib")]
fn find_onnxruntime_lib(capi_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(capi_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            if cfg!(target_os = "windows") {
                name == "onnxruntime.dll"
            } else if cfg!(target_os = "macos") {
                name.starts_with("libonnxruntime.") && name.ends_with(".dylib")
            } else {
                name.starts_with("libonnxruntime.so")
            }
        })
}

#[cfg(feature = "ort-dylib")]
fn resolve_dylib_path(py: Python, module_name: &str) -> PyResult<std::path::PathBuf> {
    let module = py.import(module_name).map_err(|e| {
        PyRuntimeError::new_err(format!(
            "The `{module_name}` package is required to load onnxruntime. \
             Install it with `pip install onnxruntime`. Caused by: {e}"
        ))
    })?;
    let file: String = module.getattr("__file__")?.extract()?;
    let capi_dir = std::path::Path::new(&file)
        .parent()
        .map(|dir| dir.join("capi"))
        .ok_or_else(|| {
            PyRuntimeError::new_err(format!("`{module_name}` has no parent directory"))
        })?;

    find_onnxruntime_lib(&capi_dir).ok_or_else(|| {
        PyRuntimeError::new_err(format!(
            "Could not find the onnxruntime shared library under {}. \
             Make sure `onnxruntime` is installed with `pip install onnxruntime`.",
            capi_dir.display()
        ))
    })
}

#[pymodule]
fn dengjen_tashkeel_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    #[cfg(feature = "ort-dylib")]
    {
        let dylib_path = resolve_dylib_path(py, "onnxruntime")?;
        init_ort_dylib(&dylib_path).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to load onnxruntime. Caused by: {e}"))
        })?;
    }

    let engine = match create_inference_engine(None) {
        Ok(eng) => eng,
        Err(e) => {
            let error = PyRuntimeError::new_err(
                format!("Failed to create inference engine.\nCaused by: {}\nPlease make sure the system dependencies are properly installed", e)
            );
            return Err(error);
        }
    };
    // Mirrors capi's do_init_library: losing this race means some other
    if INFERENCE_ENGINE.set(py, engine).is_err() {
        log::warn!("Inference engine was already initialized; ignoring redundant init.");
    }

    m.add_function(wrap_pyfunction!(tashkeel, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_tashkeel(text: &str, taskeen_threshold: Option<f32>) -> PyResult<String> {
        Python::attach(|py| {
            INFERENCE_ENGINE.get_or_init(py, || create_inference_engine(None).unwrap());
            tashkeel(py, text.to_string(), taskeen_threshold, None)
        })
    }

    #[test]
    fn tashkeel_diacritizes_plain_text() {
        let result = call_tashkeel("بسم الله الرحمن الرحيم", None);

        let diacritized = result.unwrap();
        assert_ne!(diacritized, "بسم الله الرحمن الرحيم");
        assert!(!diacritized.is_empty());
    }

    #[test]
    fn tashkeel_with_taskeen_threshold_differs_from_default() {
        let without_taskeen = call_tashkeel("بسم الله الرحمن الرحيم", None).unwrap();
        let with_taskeen = call_tashkeel("بسم الله الرحمن الرحيم", Some(0.8)).unwrap();

        assert_ne!(without_taskeen, with_taskeen);
    }

    #[test]
    fn tashkeel_maps_input_too_long_to_value_error_not_runtime_error() {
        let too_long_text = "ا".repeat(dengjen_tashkeel::CHAR_LIMIT + 1);

        let result = call_tashkeel(&too_long_text, None);

        Python::attach(|py| {
            let err = result.unwrap_err();
            assert!(err.is_instance_of::<pyo3::exceptions::PyValueError>(py));
        });
    }

    #[cfg(feature = "ort-dylib")]
    #[test]
    fn find_onnxruntime_lib_picks_the_versioned_so_over_the_providers_lib() {
        let dir = tempfile::tempdir().unwrap();
        let capi = dir.path();
        std::fs::write(capi.join("libonnxruntime_providers_shared.so"), b"").unwrap();
        std::fs::write(capi.join("libonnxruntime.so.1.28.0"), b"").unwrap();

        let found = find_onnxruntime_lib(capi).unwrap();

        assert_eq!(found.file_name().unwrap(), "libonnxruntime.so.1.28.0");
    }

    #[cfg(feature = "ort-dylib")]
    #[test]
    fn find_onnxruntime_lib_returns_none_when_absent() {
        let dir = tempfile::tempdir().unwrap();

        let found = find_onnxruntime_lib(dir.path());

        assert!(found.is_none());
    }

    #[cfg(feature = "ort-dylib")]
    #[test]
    fn resolve_dylib_path_finds_the_lib_via_a_fake_installed_package() {
        let root = tempfile::tempdir().unwrap();
        let pkg_dir = root.path().join("fake_onnxruntime");
        let capi_dir = pkg_dir.join("capi");
        std::fs::create_dir_all(&capi_dir).unwrap();
        std::fs::write(pkg_dir.join("__init__.py"), b"").unwrap();
        std::fs::write(capi_dir.join("libonnxruntime.so.1.28.0"), b"").unwrap();

        Python::attach(|py| {
            let sys_path = py.import("sys").unwrap().getattr("path").unwrap();
            sys_path
                .call_method1("insert", (0, root.path().to_str().unwrap()))
                .unwrap();

            let found = resolve_dylib_path(py, "fake_onnxruntime").unwrap();

            assert_eq!(found.file_name().unwrap(), "libonnxruntime.so.1.28.0");
        });
    }

    #[cfg(feature = "ort-dylib")]
    #[test]
    fn resolve_dylib_path_errors_with_an_actionable_message_when_package_missing() {
        Python::attach(|py| {
            let err = resolve_dylib_path(py, "definitely_not_an_installed_package").unwrap_err();

            let message = err.value(py).to_string();
            assert!(message.contains("pip install onnxruntime"), "{message}");
        });
    }
}
