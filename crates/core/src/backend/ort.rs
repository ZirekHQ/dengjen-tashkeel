use crate::{DengjenTashkeelError, DengjenTashkeelResult, InferenceEngine};
use ndarray::{Array1, Array2};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::num::NonZeroUsize;
use std::path::Path;

mod session_pool;
use session_pool::SessionPool;

impl<R> From<ort::Error<R>> for DengjenTashkeelError {
    fn from(other: ort::Error<R>) -> Self {
        DengjenTashkeelError::InferenceError(format!(
            "Failed to run model using onnxruntime via ort. Caused by {}",
            other
        ))
    }
}

fn ort_session_run(
    pool: &SessionPool<Session>,
    input_ids: Vec<i64>,
    diac_ids: Vec<i64>,
    seq_length: usize,
) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
    let input_ids = Array2::<i64>::from_shape_vec((1, seq_length), input_ids).map_err(|e| {
        DengjenTashkeelError::InferenceError(format!("input_ids/seq_length mismatch: {e}"))
    })?;
    let diac_ids = Array2::<i64>::from_shape_vec((1, seq_length), diac_ids).map_err(|e| {
        DengjenTashkeelError::InferenceError(format!("diac_ids/seq_length mismatch: {e}"))
    })?;
    let input_length = Array1::<i64>::from_iter([seq_length as i64]);

    let (target_ids, logits): (Vec<u8>, Vec<f32>) = {
        let inputs = ort::inputs![
            Tensor::from_array(input_ids)?,
            Tensor::from_array(diac_ids)?,
            Tensor::from_array(input_length)?,
        ];
        let mut session = pool
            .acquire()
            .map_err(|e| DengjenTashkeelError::InferenceError(e.to_string()))?;
        let outputs = session.run(inputs)?;
        if outputs.len() < 2 {
            return Err(DengjenTashkeelError::InferenceError(format!(
                "model returned {} output tensor(s), expected 2 (target_ids, logits)",
                outputs.len()
            )));
        }
        let (target_shape, target_ids) = outputs[0].try_extract_tensor::<u8>()?;
        let (logits_shape, logits) = outputs[1].try_extract_tensor::<f32>()?;
        if target_ids.len() != seq_length {
            return Err(DengjenTashkeelError::InferenceError(format!(
                "model returned {} target ids (shape {target_shape:?}) for a sequence of length {seq_length}",
                target_ids.len()
            )));
        }
        if logits.len() < seq_length {
            return Err(DengjenTashkeelError::InferenceError(format!(
                "model returned {} logits (shape {logits_shape:?}), fewer than the sequence length {seq_length}",
                logits.len()
            )));
        }
        (target_ids.to_vec(), logits.to_vec())
    };

    Ok((target_ids, logits))
}

/// Pads a batch of independently-sized sequences to a common length and runs them
/// through one `session.run()` call, splitting the result back into one entry per
/// input in the same order. Padding position values don't matter beyond being valid
/// vocabulary ids -- the model masks them out itself via the `input_lengths` tensor,
/// and every result here is trimmed back to its own real (unpadded) length before
/// returning, so a batch of one reproduces [`ort_session_run`]'s output exactly.
fn ort_session_run_batch(
    pool: &SessionPool<Session>,
    batch: Vec<(Vec<i64>, Vec<i64>, usize)>,
) -> DengjenTashkeelResult<Vec<(Vec<u8>, Vec<f32>)>> {
    if batch.is_empty() {
        return Ok(Vec::new());
    }

    const PAD_ID: i64 = 0;
    let batch_size = batch.len();
    let max_seq_length = batch
        .iter()
        .map(|(_, _, seq_length)| *seq_length)
        .max()
        .expect("batch checked non-empty above");
    let classes_per_position = crate::TARGET_ID_MAP.len();

    // A mismatched row can't be caught by the aggregate length check on the flattened
    // batch below: a too-long row and a correspondingly too-short row can sum to the
    // expected total, silently shifting every row's data across its real boundary.
    for (i, (input_ids, diac_ids, seq_length)) in batch.iter().enumerate() {
        if input_ids.len() != *seq_length {
            return Err(DengjenTashkeelError::InferenceError(format!(
                "batch item {i} has {} input_ids but a declared seq_length of {seq_length}",
                input_ids.len()
            )));
        }
        if diac_ids.len() != *seq_length {
            return Err(DengjenTashkeelError::InferenceError(format!(
                "batch item {i} has {} diac_ids but a declared seq_length of {seq_length}",
                diac_ids.len()
            )));
        }
    }

    let mut char_inputs = Vec::with_capacity(batch_size * max_seq_length);
    let mut diac_inputs = Vec::with_capacity(batch_size * max_seq_length);
    let mut input_lengths = Vec::with_capacity(batch_size);
    for (input_ids, diac_ids, seq_length) in &batch {
        char_inputs.extend_from_slice(input_ids);
        char_inputs.extend(std::iter::repeat_n(PAD_ID, max_seq_length - seq_length));
        diac_inputs.extend_from_slice(diac_ids);
        diac_inputs.extend(std::iter::repeat_n(PAD_ID, max_seq_length - seq_length));
        input_lengths.push(*seq_length as i64);
    }

    let char_inputs = Array2::<i64>::from_shape_vec((batch_size, max_seq_length), char_inputs)
        .map_err(|e| {
            DengjenTashkeelError::InferenceError(format!("batch char_inputs shape mismatch: {e}"))
        })?;
    let diac_inputs = Array2::<i64>::from_shape_vec((batch_size, max_seq_length), diac_inputs)
        .map_err(|e| {
            DengjenTashkeelError::InferenceError(format!("batch diac_inputs shape mismatch: {e}"))
        })?;
    let input_lengths = Array1::<i64>::from_iter(input_lengths);

    let inputs = ort::inputs![
        Tensor::from_array(char_inputs)?,
        Tensor::from_array(diac_inputs)?,
        Tensor::from_array(input_lengths)?,
    ];
    let mut session = pool
        .acquire()
        .map_err(|e| DengjenTashkeelError::InferenceError(e.to_string()))?;
    let outputs = session.run(inputs)?;
    if outputs.len() < 2 {
        return Err(DengjenTashkeelError::InferenceError(format!(
            "model returned {} output tensor(s), expected 2 (predictions, logits)",
            outputs.len()
        )));
    }
    let (pred_shape, predictions) = outputs[0].try_extract_tensor::<u8>()?;
    let (logits_shape, logits) = outputs[1].try_extract_tensor::<f32>()?;
    let expected_pred_shape = [batch_size as i64, max_seq_length as i64];
    if pred_shape[..] != expected_pred_shape {
        return Err(DengjenTashkeelError::InferenceError(format!(
            "model returned predictions with shape {pred_shape:?}, expected {expected_pred_shape:?}"
        )));
    }
    let expected_logits_shape = [
        batch_size as i64,
        max_seq_length as i64,
        classes_per_position as i64,
    ];
    if logits_shape[..] != expected_logits_shape {
        return Err(DengjenTashkeelError::InferenceError(format!(
            "model returned logits with shape {logits_shape:?}, expected {expected_logits_shape:?}"
        )));
    }

    Ok(batch
        .iter()
        .enumerate()
        .map(|(i, (_, _, seq_length))| {
            let pred_start = i * max_seq_length;
            let target_ids = predictions[pred_start..pred_start + seq_length].to_vec();

            let logits_start = i * max_seq_length * classes_per_position;
            let logits_end = logits_start + seq_length * classes_per_position;
            let sample_logits = logits[logits_start..logits_end].to_vec();

            (target_ids, sample_logits)
        })
        .collect())
}

const MODEL_BYTES: &[u8] = include_bytes!("../../data/ort/model.onnx");

/// Number of pooled sessions to build when the caller doesn't request a specific size.
///
/// One session per available thread of parallelism lets `rayon`'s sentence fan-out (or
/// any other concurrent caller) run inference without queuing on a single session, while
/// each session itself uses a single intra-op thread to avoid oversubscribing the machine.
fn default_pool_size() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap())
}

fn build_session(model_bytes: &[u8]) -> DengjenTashkeelResult<Session> {
    Ok(Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(1)?
        .commit_from_memory(model_bytes)?)
}

pub struct OrtEngine(SessionPool<Session>);

impl OrtEngine {
    pub fn from_bytes(
        model_bytes: &[u8],
        pool_size: Option<NonZeroUsize>,
    ) -> DengjenTashkeelResult<OrtEngine> {
        let pool_size = pool_size.unwrap_or_else(default_pool_size).get();
        let sessions = (0..pool_size)
            .map(|_| build_session(model_bytes))
            .collect::<DengjenTashkeelResult<Vec<_>>>()?;

        Ok(Self(SessionPool::new(sessions)))
    }
    pub fn from_path(
        model_path: impl AsRef<Path>,
        pool_size: Option<NonZeroUsize>,
    ) -> DengjenTashkeelResult<Self> {
        let model_path = model_path.as_ref();
        let model_bytes = std::fs::read(model_path).map_err(|e| {
            DengjenTashkeelError::InferenceError(format!(
                "Failed to read model file `{}`. Caused by: {e}",
                model_path.display()
            ))
        })?;
        Self::from_bytes(&model_bytes, pool_size)
    }
    pub fn with_bundled_model() -> DengjenTashkeelResult<OrtEngine> {
        Self::from_bytes(MODEL_BYTES, None)
    }
}

impl InferenceEngine for OrtEngine {
    fn infer(
        &self,
        input_ids: Vec<i64>,
        diac_ids: Vec<i64>,
        seq_length: usize,
    ) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
        ort_session_run(&self.0, input_ids, diac_ids, seq_length)
    }

    fn infer_batch(
        &self,
        batch: Vec<(Vec<i64>, Vec<i64>, usize)>,
    ) -> DengjenTashkeelResult<Vec<(Vec<u8>, Vec<f32>)>> {
        ort_session_run_batch(&self.0, batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn from_bytes_errors_on_malformed_model_data() {
        let malformed_bytes = b"this is not a valid onnx model";

        let result = OrtEngine::from_bytes(malformed_bytes, None);

        assert!(matches!(
            result,
            Err(DengjenTashkeelError::InferenceError(_))
        ));
    }

    #[test]
    fn infer_runs_against_the_bundled_model_directly() {
        let engine = OrtEngine::with_bundled_model().unwrap();
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
        let input_ids = vec![1i64, 2, 3];
        let diac_ids = vec![0i64, 0];

        let result = engine.infer(input_ids, diac_ids, 3);

        assert!(matches!(
            result,
            Err(DengjenTashkeelError::InferenceError(_))
        ));
    }

    #[test]
    fn infer_succeeds_when_called_concurrently_from_multiple_threads() {
        let pool_size = std::num::NonZeroUsize::new(2).unwrap();
        let engine = Arc::new(OrtEngine::from_bytes(MODEL_BYTES, Some(pool_size)).unwrap());

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let engine = Arc::clone(&engine);
                std::thread::spawn(move || engine.infer(vec![1i64, 2, 3], vec![0i64, 0, 0], 3))
            })
            .collect();

        for handle in handles {
            let (target_ids, logits) = handle.join().unwrap().unwrap();
            assert_eq!(target_ids.len(), 3);
            assert!(!logits.is_empty());
        }
    }

    fn bundled_pool() -> SessionPool<Session> {
        let session = Session::builder()
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .unwrap()
            .with_intra_threads(1)
            .unwrap()
            .commit_from_memory(MODEL_BYTES)
            .unwrap();
        SessionPool::new(vec![session])
    }

    #[test]
    fn batch_of_one_matches_a_direct_single_item_run() {
        let pool = bundled_pool();
        let input_ids = vec![1i64, 2, 3];
        let diac_ids = vec![0i64, 0, 0];

        let direct = ort_session_run(&pool, input_ids.clone(), diac_ids.clone(), 3).unwrap();
        let batched = ort_session_run_batch(&pool, vec![(input_ids, diac_ids, 3)]).unwrap();

        assert_eq!(batched.len(), 1);
        assert_eq!(batched[0], direct);
    }

    #[test]
    fn batched_results_are_trimmed_to_each_items_own_seq_length() {
        let pool = bundled_pool();
        let batch = vec![
            (vec![1i64, 2, 3], vec![0i64, 0, 0], 3),
            (vec![1i64, 2, 3, 4, 5], vec![0i64, 0, 0, 0, 0], 5),
            (vec![1i64, 2], vec![0i64, 0], 2),
        ];

        let results = ort_session_run_batch(&pool, batch).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0.len(), 3);
        assert_eq!(results[1].0.len(), 5);
        assert_eq!(results[2].0.len(), 2);
        assert_eq!(results[0].1.len(), 3 * crate::TARGET_ID_MAP.len());
        assert_eq!(results[1].1.len(), 5 * crate::TARGET_ID_MAP.len());
        assert_eq!(results[2].1.len(), 2 * crate::TARGET_ID_MAP.len());
    }

    #[test]
    fn empty_batch_short_circuits_without_touching_the_pool() {
        let pool = bundled_pool();

        let results = ort_session_run_batch(&pool, vec![]).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn batch_errors_when_an_items_input_ids_length_does_not_match_its_seq_length() {
        let pool = bundled_pool();
        let batch = vec![(vec![1i64, 2, 3], vec![0i64, 0, 0], 5)];

        let result = ort_session_run_batch(&pool, batch);

        assert!(
            matches!(result, Err(DengjenTashkeelError::InferenceError(msg)) if msg.contains("input_ids"))
        );
    }

    #[test]
    fn batch_errors_when_an_items_diac_ids_length_does_not_match_its_seq_length() {
        let pool = bundled_pool();
        let batch = vec![(vec![1i64, 2, 3], vec![0i64, 0], 3)];

        let result = ort_session_run_batch(&pool, batch);

        assert!(
            matches!(result, Err(DengjenTashkeelError::InferenceError(msg)) if msg.contains("diac_ids"))
        );
    }

    #[test]
    fn a_malformed_pair_of_batch_items_does_not_silently_corrupt_row_boundaries() {
        // Without per-item validation, an over-long first row and a correspondingly
        // short second row could sum to the expected total element count and slip
        // past `Array2::from_shape_vec`'s aggregate length check.
        let pool = bundled_pool();
        let batch = vec![
            (vec![1i64, 2, 3, 4, 5], vec![0i64, 0, 0, 0, 0], 3),
            (vec![1i64], vec![0i64], 3),
        ];

        let result = ort_session_run_batch(&pool, batch);

        assert!(matches!(
            result,
            Err(DengjenTashkeelError::InferenceError(_))
        ));
    }
}
