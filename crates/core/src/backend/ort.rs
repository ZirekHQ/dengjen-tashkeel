use crate::{InferenceEngine, LibtashkeelError, LibtashkeelResult};
use ndarray::{Array1, Array2};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::path::Path;
use std::sync::Mutex;

impl<R> From<ort::Error<R>> for LibtashkeelError {
    fn from(other: ort::Error<R>) -> Self {
        LibtashkeelError::InferenceError(format!(
            "Failed to run model using onnxruntime via ort. Caused by {}",
            other
        ))
    }
}

fn ort_session_run(
    session: &Mutex<Session>,
    input_ids: Vec<i64>,
    diac_ids: Vec<i64>,
    seq_length: usize,
) -> LibtashkeelResult<(Vec<u8>, Vec<f32>)> {
    let input_ids = Array2::<i64>::from_shape_vec((1, seq_length), input_ids).map_err(|e| {
        LibtashkeelError::InferenceError(format!("input_ids/seq_length mismatch: {e}"))
    })?;
    let diac_ids = Array2::<i64>::from_shape_vec((1, seq_length), diac_ids).map_err(|e| {
        LibtashkeelError::InferenceError(format!("diac_ids/seq_length mismatch: {e}"))
    })?;
    let input_length = Array1::<i64>::from_iter([seq_length as i64]);

    let (target_ids, logits): (Vec<u8>, Vec<f32>) = {
        let inputs = ort::inputs![
            Tensor::from_array(input_ids)?,
            Tensor::from_array(diac_ids)?,
            Tensor::from_array(input_length)?,
        ];
        let mut session = session.lock().unwrap();
        let outputs = session.run(inputs)?;
        let (_, target_ids) = outputs[0].try_extract_tensor::<u8>()?;
        let (_, logits) = outputs[1].try_extract_tensor::<f32>()?;
        (target_ids.to_vec(), logits.to_vec())
    };

    Ok((target_ids, logits))
}

const MODEL_BYTES: &[u8] = include_bytes!("../../data/ort/model.onnx");

pub struct OrtEngine(Mutex<Session>);

impl OrtEngine {
    pub fn from_bytes(model_bytes: &[u8]) -> LibtashkeelResult<OrtEngine> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_parallel_execution(true)?
            .with_inter_threads(2)?
            .with_intra_threads(2)?
            .commit_from_memory(model_bytes)?;

        Ok(Self(Mutex::new(session)))
    }
    pub fn from_path(model_path: impl AsRef<Path>) -> LibtashkeelResult<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            // .with_allocator(ort::AllocatorType::Arena)?
            // .with_memory_pattern(true)?
            // .with_parallel_execution(true)?
            // .with_inter_threads(2)?
            // .with_intra_threads(2)?
            .commit_from_file(model_path)?;

        Ok(Self(Mutex::new(session)))
    }
    pub fn with_bundled_model() -> LibtashkeelResult<OrtEngine> {
        Self::from_bytes(MODEL_BYTES)
    }
}

impl InferenceEngine for OrtEngine {
    fn infer(
        &self,
        input_ids: Vec<i64>,
        diac_ids: Vec<i64>,
        seq_length: usize,
    ) -> LibtashkeelResult<(Vec<u8>, Vec<f32>)> {
        ort_session_run(&self.0, input_ids, diac_ids, seq_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_bytes_errors_on_malformed_model_data() {
        let malformed_bytes = b"this is not a valid onnx model";

        let result = OrtEngine::from_bytes(malformed_bytes);

        assert!(matches!(result, Err(LibtashkeelError::InferenceError(_))));
    }

    #[test]
    fn infer_runs_against_the_bundled_model_directly() {
        let engine = OrtEngine::with_bundled_model().unwrap();
        // "بسم" tokenized via the same maps do_tashkeel uses internally;
        // this test only needs *some* valid, non-empty token sequence to
        // exercise ort_session_run's Tensor::from_array/try_extract_tensor
        // path independent of the full do_tashkeel pipeline.
        let input_ids = vec![1i64, 2, 3];
        let diac_ids = vec![0i64, 0, 0];

        let result = engine.infer(input_ids, diac_ids, 3);

        assert!(result.is_ok());
        let (target_ids, logits) = result.unwrap();
        assert_eq!(target_ids.len(), 3);
        assert!(!logits.is_empty());
    }

    #[test]
    fn infer_errors_instead_of_panicking_on_seq_length_mismatch() {
        let engine = OrtEngine::with_bundled_model().unwrap();
        // diac_ids has 2 entries but seq_length claims 3 -- from_shape_vec
        // must reject this as an error, not panic.
        let input_ids = vec![1i64, 2, 3];
        let diac_ids = vec![0i64, 0];

        let result = engine.infer(input_ids, diac_ids, 3);

        assert!(matches!(result, Err(LibtashkeelError::InferenceError(_))));
    }
}
