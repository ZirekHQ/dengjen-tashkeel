use crate::{DengjenTashkeelResult, InferenceEngine};
use std::path::PathBuf;

pub struct DynamicInferenceEngine(Box<dyn InferenceEngine + Send + Sync>);

impl DynamicInferenceEngine {
    pub fn new(engine: Box<dyn InferenceEngine + Send + Sync>) -> Self {
        Self(engine)
    }
}

impl InferenceEngine for DynamicInferenceEngine {
    fn infer(
        &self,
        input_ids: Vec<i64>,
        diac_ids: Vec<i64>,
        seq_length: usize,
    ) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
        self.0.infer(input_ids, diac_ids, seq_length)
    }

    fn infer_batch(
        &self,
        batch: Vec<(Vec<i64>, Vec<i64>, usize)>,
    ) -> DengjenTashkeelResult<Vec<(Vec<u8>, Vec<f32>)>> {
        self.0.infer_batch(batch)
    }
}

#[cfg(any(feature = "ort-static", feature = "ort-dylib"))]
mod ort;

#[cfg(any(feature = "ort-static", feature = "ort-dylib"))]
pub fn create_inference_engine(
    model_path: Option<PathBuf>,
) -> DengjenTashkeelResult<DynamicInferenceEngine> {
    use self::ort::OrtEngine;

    log::info!("Built with `ORT` inference backend.");

    match model_path {
        Some(path) => {
            log::info!("Loading model from path: `{}`", path.display());
            let engine = OrtEngine::from_path(&path, None)?;
            Ok(DynamicInferenceEngine::new(Box::new(engine)))
        }
        None => {
            log::info!("Using bundled model");
            let engine = OrtEngine::with_bundled_model()?;
            Ok(DynamicInferenceEngine::new(Box::new(engine)))
        }
    }
}

#[cfg(feature = "ort-dylib")]
pub fn init_ort_dylib(path: impl AsRef<std::path::Path>) -> DengjenTashkeelResult<()> {
    let path = path.as_ref();
    ::ort::init_from(path)
        .map_err(|e| {
            crate::DengjenTashkeelError::InferenceError(format!(
                "Failed to load onnxruntime dynamic library from `{}`. Caused by: {e}",
                path.display()
            ))
        })?
        .commit();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct BatchCounter(Arc<AtomicUsize>);

    impl InferenceEngine for BatchCounter {
        fn infer(
            &self,
            _input_ids: Vec<i64>,
            _diac_ids: Vec<i64>,
            _seq_length: usize,
        ) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
            unreachable!("batch must not fall back to per-item infer")
        }

        fn infer_batch(
            &self,
            batch: Vec<(Vec<i64>, Vec<i64>, usize)>,
        ) -> DengjenTashkeelResult<Vec<(Vec<u8>, Vec<f32>)>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(batch.iter().map(|_| (vec![], vec![])).collect())
        }
    }

    #[test]
    fn dynamic_engine_forwards_infer_batch_to_the_wrapped_engine() {
        let calls = Arc::new(AtomicUsize::new(0));
        let engine = DynamicInferenceEngine::new(Box::new(BatchCounter(Arc::clone(&calls))));

        let results = engine
            .infer_batch(vec![(vec![1], vec![0], 1), (vec![2], vec![0], 1)])
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
