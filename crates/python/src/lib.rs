#[cfg(feature = "ort-dylib")]
use dengjen_tashkeel::init_ort_dylib;
use dengjen_tashkeel::{
    create_inference_engine, do_tashkeel_with, DengjenTashkeelError, DynamicInferenceEngine,
    TashkeelOptions,
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

fn engine_creation_error(error: DengjenTashkeelError) -> PyErr {
    PyRuntimeError::new_err(format!(
        "Failed to create inference engine.\nCaused by: {error}\n\
         Please make sure the system dependencies are properly installed"
    ))
}

fn engine(py: Python<'_>) -> PyResult<&DynamicInferenceEngine> {
    if let Some(existing) = INFERENCE_ENGINE.get(py) {
        return Ok(existing);
    }
    // Loading builds the ONNX sessions; other Python threads keep running meanwhile. A thread
    // that loses the race drops its engine and uses the winner's.
    let created = py
        .detach(|| create_inference_engine(None))
        .map_err(engine_creation_error)?;
    let _ = INFERENCE_ENGINE.set(py, created);
    INFERENCE_ENGINE
        .get(py)
        .ok_or_else(|| PyRuntimeError::new_err("Failed to retrieve inference engine"))
}

/// Loads the engine up front, optionally from a custom ONNX model file.
///
/// Without an explicit call the bundled model loads on the first `tashkeel` call.
/// Raises `RuntimeError` when an engine is already loaded or the model cannot be loaded.
#[pyfunction]
#[pyo3(signature = (model_path=None))]
fn init(py: Python, model_path: Option<std::path::PathBuf>) -> PyResult<()> {
    if INFERENCE_ENGINE.get(py).is_some() {
        return Err(PyRuntimeError::new_err(
            "Inference engine is already initialized.",
        ));
    }
    let created = py
        .detach(|| create_inference_engine(model_path))
        .map_err(engine_creation_error)?;
    INFERENCE_ENGINE
        .set(py, created)
        .map_err(|_| PyRuntimeError::new_err("Inference engine is already initialized."))
}

#[pyfunction]
#[pyo3(signature = (text, taskeen_threshold=None, preprocessed=None))]
fn tashkeel(
    py: Python,
    text: String,
    taskeen_threshold: Option<f32>,
    preprocessed: Option<bool>,
) -> PyResult<String> {
    let mut options = TashkeelOptions::new().with_preprocessed(preprocessed.unwrap_or_default());
    if let Some(threshold) = taskeen_threshold {
        options = options.with_taskeen_threshold(threshold);
    }
    let engine = engine(py)?;
    py.detach(|| do_tashkeel_with(engine, &text, &options))
        .map_err(to_py_err)
}

#[cfg(any(feature = "ort-dylib", test))]
fn is_onnxruntime_lib_name(name: &str) -> bool {
    if cfg!(target_os = "windows") {
        name == "onnxruntime.dll"
    } else if cfg!(target_os = "macos") {
        name.starts_with("libonnxruntime.") && name.ends_with(".dylib")
    } else {
        name.starts_with("libonnxruntime.so")
    }
}

#[cfg(any(feature = "ort-dylib", test))]
fn version_key(name: &str) -> Vec<u64> {
    name.split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse().ok())
        .collect()
}

#[cfg(any(feature = "ort-dylib", test))]
fn pick_onnxruntime_lib<'a>(names: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    names
        .filter(|name| is_onnxruntime_lib_name(name))
        .max_by(|a, b| version_key(a).cmp(&version_key(b)).then_with(|| a.cmp(b)))
}

#[cfg(feature = "ort-dylib")]
fn find_onnxruntime_lib(capi_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let names: Vec<String> = std::fs::read_dir(capi_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    let picked = pick_onnxruntime_lib(names.iter().map(String::as_str))?;
    Some(capi_dir.join(picked))
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
    #[cfg(feature = "ort-dylib")]
    {
        let py = m.py();
        let dylib_path = resolve_dylib_path(py, "onnxruntime")?;
        init_ort_dylib(&dylib_path).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to load onnxruntime. Caused by: {e}"))
        })?;
    }

    m.add_function(wrap_pyfunction!(tashkeel, m)?)?;
    m.add_function(wrap_pyfunction!(init, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_tashkeel(text: &str, taskeen_threshold: Option<f32>) -> PyResult<String> {
        call_tashkeel_with(text, taskeen_threshold, None)
    }

    fn call_tashkeel_with(
        text: &str,
        taskeen_threshold: Option<f32>,
        preprocessed: Option<bool>,
    ) -> PyResult<String> {
        Python::attach(|py| tashkeel(py, text.to_string(), taskeen_threshold, preprocessed))
    }

    #[test]
    fn to_py_err_maps_input_too_long_to_value_error_naming_the_limit() {
        Python::attach(|py| {
            let err = to_py_err(DengjenTashkeelError::InputTooLong(10));

            assert!(err.is_instance_of::<PyValueError>(py));
            assert!(err.to_string().contains("10"));
        });
    }

    #[test]
    fn to_py_err_maps_other_errors_to_runtime_error_carrying_the_cause() {
        Python::attach(|py| {
            let inference = to_py_err(DengjenTashkeelError::InferenceError("boom".into()));
            let load = to_py_err(DengjenTashkeelError::ModelLoadError(std::io::Error::other(
                "missing model",
            )));

            assert!(inference.is_instance_of::<PyRuntimeError>(py));
            assert!(inference.to_string().contains("boom"));
            assert!(load.is_instance_of::<PyRuntimeError>(py));
            assert!(load.to_string().contains("missing model"));
        });
    }

    #[test]
    fn init_with_a_missing_model_file_is_a_runtime_error() {
        Python::attach(|py| {
            let missing = std::path::PathBuf::from("/nonexistent/path/to/model.onnx");

            let err = init(py, Some(missing)).unwrap_err();

            assert!(err.is_instance_of::<PyRuntimeError>(py));
        });
    }

    #[test]
    fn init_after_the_engine_is_loaded_is_a_runtime_error() {
        call_tashkeel("بسم الله", None).unwrap();

        Python::attach(|py| {
            let err = init(py, None).unwrap_err();

            assert!(err.is_instance_of::<PyRuntimeError>(py));
            assert!(err.to_string().contains("already initialized"));
        });
    }

    #[test]
    fn tashkeel_accepts_preprocessed_input() {
        let result = call_tashkeel_with("بسم الله الرحمن الرحيم", None, Some(true));

        assert!(!result.unwrap().is_empty());
    }

    #[cfg(not(feature = "ort-dylib"))]
    #[test]
    fn module_init_registers_tashkeel_and_init_without_loading_the_engine() {
        Python::attach(|py| {
            let module = PyModule::new(py, "dengjen_tashkeel_py").unwrap();

            dengjen_tashkeel_py(&module).unwrap();

            assert!(module.hasattr("tashkeel").unwrap());
            assert!(module.hasattr("init").unwrap());
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pick_onnxruntime_lib_prefers_the_highest_numeric_version() {
        let names = [
            "libonnxruntime.so",
            "libonnxruntime.so.1.9.0",
            "libonnxruntime.so.1.20.1",
            "unrelated.so",
        ];

        let picked = pick_onnxruntime_lib(names.into_iter());

        assert_eq!(picked, Some("libonnxruntime.so.1.20.1"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pick_onnxruntime_lib_is_independent_of_listing_order() {
        let forward = ["libonnxruntime.so.1.2.0", "libonnxruntime.so"];
        let reversed = ["libonnxruntime.so", "libonnxruntime.so.1.2.0"];

        assert_eq!(
            pick_onnxruntime_lib(forward.into_iter()),
            pick_onnxruntime_lib(reversed.into_iter())
        );
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
        let with_taskeen = call_tashkeel("بسم الله الرحمن الرحيم", Some(0.95)).unwrap();

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
    fn onnxruntime_lib_fixture_names() -> (&'static str, &'static str) {
        if cfg!(target_os = "windows") {
            ("onnxruntime.dll", "onnxruntime_providers_shared.dll")
        } else if cfg!(target_os = "macos") {
            (
                "libonnxruntime.1.28.0.dylib",
                "libonnxruntime_providers_shared.dylib",
            )
        } else {
            (
                "libonnxruntime.so.1.28.0",
                "libonnxruntime_providers_shared.so",
            )
        }
    }

    #[cfg(feature = "ort-dylib")]
    #[test]
    fn find_onnxruntime_lib_picks_the_versioned_lib_over_the_providers_lib() {
        let dir = tempfile::tempdir().unwrap();
        let capi = dir.path();
        let (versioned, providers) = onnxruntime_lib_fixture_names();
        std::fs::write(capi.join(providers), b"").unwrap();
        std::fs::write(capi.join(versioned), b"").unwrap();

        let found = find_onnxruntime_lib(capi).unwrap();

        assert_eq!(found.file_name().unwrap(), versioned);
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
        let (versioned, _providers) = onnxruntime_lib_fixture_names();
        std::fs::write(capi_dir.join(versioned), b"").unwrap();

        Python::attach(|py| {
            let sys_path = py.import("sys").unwrap().getattr("path").unwrap();
            sys_path
                .call_method1("insert", (0, root.path().to_str().unwrap()))
                .unwrap();

            let found = resolve_dylib_path(py, "fake_onnxruntime").unwrap();

            assert_eq!(found.file_name().unwrap(), versioned);
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
