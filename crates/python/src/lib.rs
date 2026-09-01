use dengjen_tashkeel::{create_inference_engine, do_tashkeel, DynamicInferenceEngine};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::PyModule;

static INFERENCE_ENGINE: PyOnceLock<DynamicInferenceEngine> = PyOnceLock::new();

/// Diacritize Arabic text.
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
    match do_tashkeel(engine, &text, taskeen_threshold, preprocessed) {
        Ok(diacritized_text) => Ok(diacritized_text),
        Err(e) => {
            let error =
                PyRuntimeError::new_err(format!("Failed to diacritize text. Caused by: {}", e));
            Err(error)
        }
    }
}

/// A Python wrapper for libtashkeel.
#[pymodule]
fn pylibtashkeel(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let engine = match create_inference_engine(None) {
        Ok(eng) => eng,
        Err(e) => {
            let error = PyRuntimeError::new_err(
                format!("Failed to create inference engine.\nCaused by: {}\nPlease make sure the system dependencies are properly installed", e)
            );
            return Err(error);
        }
    };
    INFERENCE_ENGINE.set(py, engine).ok();

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
}
