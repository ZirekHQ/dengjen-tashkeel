use once_cell::sync::Lazy;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::iter;
use thiserror::Error;

mod backend;
pub use self::backend::DynamicInferenceEngine;

#[cfg(any(feature = "ort-static", feature = "ort-dylib"))]
pub use self::backend::create_inference_engine;

#[cfg(feature = "ort-dylib")]
pub use self::backend::init_ort_dylib;

pub type DengjenTashkeelResult<T> = Result<T, DengjenTashkeelError>;

pub const CHAR_LIMIT: usize = 12000;
const PAD: char = '_';
const NUMERAL_SYMBOL: char = '#';
const NUMERALS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨',
    '٩',
];
static INPUT_ID_MAP: Lazy<HashMap<char, i64>> =
    Lazy::new(|| serde_json::from_str(include_str!("../data/input_id_map.json")).unwrap());
static TARGET_ID_MAP: Lazy<HashMap<u8, String>> = Lazy::new(|| {
    let target_id_map: HashMap<String, u8> =
        serde_json::from_str(include_str!("../data/target_id_map.json")).unwrap();
    reversed_mapping(&target_id_map)
});
static HINT_ID_MAP: Lazy<HashMap<String, i64>> =
    Lazy::new(|| serde_json::from_str(include_str!("../data/hint_id_map.json")).unwrap());
static TARGET_META_CHAR_IDS: Lazy<HashSet<u8>> = Lazy::new(|| {
    let pad = PAD.to_string();
    TARGET_ID_MAP
        .iter()
        .filter(|(_, name)| **name == pad)
        .map(|(&id, _)| id)
        .collect()
});
static ARABIC_DIACRITICS: Lazy<HashSet<char>> = Lazy::new(|| {
    HashSet::from_iter(
        [1618, 1617, 1614, 1615, 1616, 1611, 1612, 1613]
            .iter()
            .map(|i| char::from_u32(*i).unwrap()),
    )
});
static NORMALIZED_DIAC_MAP: Lazy<HashMap<&str, &str>> =
    Lazy::new(|| HashMap::from([("َّ", "َّ"), ("ًّ", "ًّ"), ("ُّ", "ُّ"), ("ٌّ", "ٌّ"), ("ِّ", "ِّ"), ("ٍّ", "ٍّ")]));

pub trait InferenceEngine {
    fn infer(
        &self,
        input_ids: Vec<i64>,
        diac_ids: Vec<i64>,
        seq_length: usize,
    ) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)>;

    /// Runs a batch of independent sequences through the model, one result per input in
    /// the same order. The default implementation calls [`infer`](InferenceEngine::infer)
    /// once per item; backends that can serve a whole batch in one model call should
    /// override this instead.
    fn infer_batch(
        &self,
        batch: Vec<(Vec<i64>, Vec<i64>, usize)>,
    ) -> DengjenTashkeelResult<Vec<(Vec<u8>, Vec<f32>)>> {
        batch
            .into_iter()
            .map(|(input_ids, diac_ids, seq_length)| self.infer(input_ids, diac_ids, seq_length))
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum DengjenTashkeelError {
    #[error("input too long. Expected {0} characters")]
    InputTooLong(usize),
    #[error("Inference error. {0}")]
    InferenceError(String),
    #[error("Resource not found. {0}")]
    ModelLoadError(#[from] std::io::Error),
}

fn reversed_mapping<K, V>(input: &HashMap<K, V>) -> HashMap<V, K>
where
    K: ToOwned<Owned = K>,
    V: ToOwned<Owned = V> + std::hash::Hash + std::cmp::Eq,
{
    HashMap::from_iter(input.iter().map(|(k, v)| (v.to_owned(), k.to_owned())))
}

#[inline(always)]
fn is_diacritic_char(c: char) -> bool {
    ARABIC_DIACRITICS.contains(&c)
}

fn extract_chars_and_diacritics(
    input_text: &str,
    normalize_diacritics: bool,
) -> (String, Vec<String>) {
    let input_text = input_text.trim_start_matches(is_diacritic_char);

    let mut clean_chars = String::new();
    let mut diacritics = Vec::new();

    let mut pending_diac = String::with_capacity(2);
    input_text.chars().chain(iter::once(' ')).for_each(|c| {
        if is_diacritic_char(c) {
            pending_diac.push(c);
        } else {
            clean_chars.push(c);
            diacritics.push(std::mem::take(&mut pending_diac));
        }
    });

    clean_chars.pop().unwrap();
    diacritics.remove(0);

    if normalize_diacritics {
        for diac in diacritics.iter_mut() {
            if !HINT_ID_MAP.contains_key(diac) {
                if let Some(d) = NORMALIZED_DIAC_MAP.get(diac.as_str()) {
                    *diac = d.to_string();
                } else {
                    *diac = "".into();
                }
            }
        }
    }

    (clean_chars, diacritics)
}

fn to_valid_chars(input: impl Iterator<Item = char>) -> (String, HashSet<char>) {
    let mut valid = String::new();
    let mut invalid = HashSet::new();
    for c in input {
        if INPUT_ID_MAP.contains_key(&c) | ARABIC_DIACRITICS.contains(&c) {
            valid.push(c);
        } else if NUMERALS.contains(&c) {
            valid.push(NUMERAL_SYMBOL);
        } else {
            invalid.insert(c);
        }
    }
    (valid, invalid)
}

fn input_to_ids(input: impl Iterator<Item = char>) -> DengjenTashkeelResult<Vec<i64>> {
    input
        .map(|c| {
            INPUT_ID_MAP.get(&c).copied().ok_or_else(|| {
                DengjenTashkeelError::InferenceError(format!("unknown input character `{c}`"))
            })
        })
        .collect()
}

fn hint_to_ids(hints: Vec<String>) -> DengjenTashkeelResult<Vec<i64>> {
    hints
        .into_iter()
        .map(|s| {
            HINT_ID_MAP.get(&s).copied().ok_or_else(|| {
                DengjenTashkeelError::InferenceError(format!("unknown diacritic hint `{s}`"))
            })
        })
        .collect()
}

fn target_to_diacritics(
    target_ids: impl Iterator<Item = u8>,
) -> DengjenTashkeelResult<Vec<String>> {
    target_ids
        .filter(|id| !TARGET_META_CHAR_IDS.contains(id))
        .map(|diac_id| {
            TARGET_ID_MAP.get(&diac_id).cloned().ok_or_else(|| {
                DengjenTashkeelError::InferenceError(format!(
                    "model produced unknown target id `{diac_id}`"
                ))
            })
        })
        .collect()
}

fn annotate_text_with_diacritics(
    input: &str,
    diacritics: Vec<String>,
    removed_chars: HashSet<char>,
) -> DengjenTashkeelResult<String> {
    let mut output = String::new();
    let mut diac_iter = diacritics.into_iter();
    for c in input.chars() {
        if ARABIC_DIACRITICS.contains(&c) {
            continue;
        } else if removed_chars.contains(&c) {
            output.push(c);
        } else {
            output.push(c);
            let diac = diac_iter.next().ok_or_else(|| {
                DengjenTashkeelError::InferenceError(
                    "model output fewer diacritics than input characters".to_string(),
                )
            })?;
            output.push_str(&diac);
        }
    }
    Ok(output)
}

fn annotate_text_with_diacritics_taskeen(
    input: &str,
    diacritics: Vec<String>,
    removed_chars: HashSet<char>,
    logits: Vec<f32>,
    taskeen_threshold: Option<f32>,
) -> DengjenTashkeelResult<String> {
    let taskeen_threshold = taskeen_threshold.unwrap();
    let sukoon = char::from_u32(0x652).unwrap();
    let mut output = String::new();
    let mut diac_iter = diacritics.into_iter().zip(logits);
    for c in input.chars() {
        if ARABIC_DIACRITICS.contains(&c) {
            continue;
        } else if removed_chars.contains(&c) {
            output.push(c);
        } else {
            output.push(c);
            let (diac, logit) = diac_iter.next().ok_or_else(|| {
                DengjenTashkeelError::InferenceError(
                    "model output fewer diacritics/logits than input characters".to_string(),
                )
            })?;
            if logit > taskeen_threshold {
                output.push(sukoon);
            } else {
                output.push_str(&diac);
            }
        }
    }
    Ok(output)
}

/// A sentence's text alongside everything inference needs, computed once up front so a
/// batch of sentences can share a single model call instead of one call each.
struct Tokenized {
    text: String,
    input_ids: Vec<i64>,
    diac_ids: Vec<i64>,
    seq_length: usize,
    removed_chars: HashSet<char>,
}

fn tokenize(text: &str) -> DengjenTashkeelResult<Tokenized> {
    let text = text.trim();

    if text.chars().count() > CHAR_LIMIT {
        return Err(DengjenTashkeelError::InputTooLong(CHAR_LIMIT));
    }

    let (input_text, removed_chars) = to_valid_chars(text.chars());
    let (input_text, diacritics) = extract_chars_and_diacritics(&input_text, true);

    let input_ids = input_to_ids(input_text.chars())?;
    let diac_ids = hint_to_ids(diacritics)?;
    let seq_length = input_ids.len();

    Ok(Tokenized {
        text: text.to_string(),
        input_ids,
        diac_ids,
        seq_length,
        removed_chars,
    })
}

fn postprocess(
    text: String,
    removed_chars: HashSet<char>,
    target_ids: Vec<u8>,
    logits: Vec<f32>,
    taskeen_threshold: Option<f32>,
) -> DengjenTashkeelResult<String> {
    let diacritics = target_to_diacritics(target_ids.into_iter())?;
    if taskeen_threshold.is_none() {
        annotate_text_with_diacritics(&text, diacritics, removed_chars)
    } else {
        annotate_text_with_diacritics_taskeen(
            &text,
            diacritics,
            removed_chars,
            logits,
            taskeen_threshold,
        )
    }
}

fn map_sentences(
    sentences: &[String],
    engine: &(impl InferenceEngine + Send + Sync),
    taskeen_threshold: Option<f32>,
) -> DengjenTashkeelResult<Vec<String>> {
    if sentences.is_empty() {
        return Ok(Vec::new());
    }

    #[cfg(feature = "rayon")]
    let tokenized = sentences
        .par_iter()
        .map(|sent| tokenize(sent))
        .collect::<DengjenTashkeelResult<Vec<_>>>()?;
    #[cfg(not(feature = "rayon"))]
    let tokenized = sentences
        .iter()
        .map(|sent| tokenize(sent))
        .collect::<DengjenTashkeelResult<Vec<_>>>()?;

    // A sentence that tokenizes to nothing (all-out-of-vocabulary chars) has no
    // defined inference behavior; exclude it from the batch and pass it through unchanged.
    let mut batch = Vec::new();
    let mut batch_positions = Vec::new();
    for (i, tok) in tokenized.iter().enumerate() {
        if tok.seq_length > 0 {
            batch.push((tok.input_ids.clone(), tok.diac_ids.clone(), tok.seq_length));
            batch_positions.push(i);
        }
    }

    let expected_results = batch_positions.len();
    let timer = std::time::Instant::now();
    let batch_results = engine.infer_batch(batch)?;
    log::debug!("Inference time: {} ms", timer.elapsed().as_millis() as f32);

    if batch_results.len() != expected_results {
        return Err(DengjenTashkeelError::InferenceError(format!(
            "infer_batch returned {} result(s) for {expected_results} batch item(s)",
            batch_results.len()
        )));
    }

    let mut results: Vec<Option<(Vec<u8>, Vec<f32>)>> = vec![None; tokenized.len()];
    for (position, result) in batch_positions.into_iter().zip(batch_results) {
        results[position] = Some(result);
    }

    #[cfg(feature = "rayon")]
    let iter = tokenized.into_par_iter().zip(results);
    #[cfg(not(feature = "rayon"))]
    let iter = tokenized.into_iter().zip(results);

    iter.map(|(tok, result)| match result {
        Some((target_ids, logits)) => postprocess(
            tok.text,
            tok.removed_chars,
            target_ids,
            logits,
            taskeen_threshold,
        ),
        None => Ok(tok.text),
    })
    .collect()
}

pub fn do_tashkeel(
    engine: &(impl InferenceEngine + Send + Sync),
    text: &str,
    taskeen_threshold: Option<f32>,
    preprocessed: bool,
) -> DengjenTashkeelResult<String> {
    if preprocessed {
        return _do_tashkeel_impl(engine, text, taskeen_threshold);
    }

    let sentences = libtqsm::segment("ar", text).map_err(|e| {
        DengjenTashkeelError::InferenceError(format!(
            "Failed to segment input text into sentences: `{}`",
            e
        ))
    })?;
    map_sentences(&sentences, engine, taskeen_threshold).map(|v| v.join(" "))
}

pub fn _do_tashkeel_impl(
    engine: &(impl InferenceEngine + Send + Sync),
    text: &str,
    taskeen_threshold: Option<f32>,
) -> DengjenTashkeelResult<String> {
    let tok = tokenize(text)?;

    if tok.seq_length == 0 {
        log::debug!("Inference time: {} ms", 0.0);
        return Ok(tok.text);
    }

    let timer = std::time::Instant::now();
    let (target_ids, logits) = engine.infer(tok.input_ids, tok.diac_ids, tok.seq_length)?;
    log::debug!("Inference time: {} ms", timer.elapsed().as_millis() as f32);
    postprocess(
        tok.text,
        tok.removed_chars,
        target_ids,
        logits,
        taskeen_threshold,
    )
}

// Doc-hidden re-export of private hot-path internals: benches/ is a separate compilation
// unit from #[cfg(test)], so it can't see them otherwise; NullEngine stands in for a real
// model so do_tashkeel can be benchmarked without one.
#[cfg(feature = "internal-benchmarks")]
#[doc(hidden)]
pub mod bench_internal {
    /// Returns the `seq_length` of tokenizing `text` via the crate's private `tokenize`.
    pub fn tokenize(text: &str) -> super::DengjenTashkeelResult<usize> {
        super::tokenize(text).map(|t| t.seq_length)
    }

    pub struct NullEngine;

    impl super::InferenceEngine for NullEngine {
        fn infer(
            &self,
            _input_ids: Vec<i64>,
            _diac_ids: Vec<i64>,
            seq_length: usize,
        ) -> super::DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
            Ok((vec![5; seq_length], vec![0.0; seq_length]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static INFERENCE_ENGINE: Lazy<DynamicInferenceEngine> =
        Lazy::new(|| create_inference_engine(None).unwrap());

    #[test]
    fn test_extract_diacritics_when_empty() {
        let (chars, diacritics) = extract_chars_and_diacritics("", false);
        assert!(chars.is_empty());
        assert!(diacritics.is_empty());
    }

    #[test]
    fn test_extract_diacritics() {
        let text = "بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيمِ";
        let (chars, diacritics) = extract_chars_and_diacritics(text, true);

        assert_eq!(chars.chars().count(), diacritics.len());

        assert_eq!(chars.chars().next(), Some('ب'));
        assert_eq!(diacritics[0], "ِ");

        assert_eq!(chars.chars().nth(6), Some('ل'));
        assert_eq!(diacritics[6], "َّ");
    }

    #[test]
    fn test_basic_tashkeel() -> DengjenTashkeelResult<()> {
        let text = "بسم الله الرحمن الرحيم";

        let expected = "بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيم";
        let tashkeeled = do_tashkeel(&*INFERENCE_ENGINE, text, None, false)?;
        println!("Tashkeel: {}", tashkeeled);

        assert_ne!(tashkeeled, text);
        assert_eq!(tashkeeled, expected);

        Ok(())
    }
    #[test]
    fn test_taskeen() -> DengjenTashkeelResult<()> {
        let poem = [
            "من ذا يقارن حسنك المغري بصيف قد تجلى",
            "وفنون سحرك قد بدت في ناظري أسمى وأغلى",
            "تجني الرياح العاتيات على البراعم وهي جذلى",
            "والصيف يمضي مسرعا إذ عقده المحدود ولى",
            "ستعانقين العصر في شعري، وفيك أقول",
        ]
        .join(" ");

        let no_taskeen = do_tashkeel(&*INFERENCE_ENGINE, &poem, None, false)?;
        let taskeen = do_tashkeel(&*INFERENCE_ENGINE, &poem, Some(0.8), false)?;

        assert_ne!(taskeen, no_taskeen);

        let sukoon = char::from_u32(0x652).unwrap();
        let no_taskeen_sukoon_count = no_taskeen.chars().filter(|c| c == &sukoon).count();
        let taskeen_sukoon_count = taskeen.chars().filter(|c| c == &sukoon).count();
        assert!(taskeen_sukoon_count > no_taskeen_sukoon_count);

        Ok(())
    }
    #[test]
    fn test_hints() -> DengjenTashkeelResult<()> {
        let text = "بِسمِ اللّه الرّحمن الرّحيم ABC";
        do_tashkeel(&*INFERENCE_ENGINE, text, None, false)?;
        let text = "مّنْ يُقلِّب  ABC";
        do_tashkeel(&*INFERENCE_ENGINE, text, None, false)?;
        Ok(())
    }

    struct StubEngine {
        target_ids: Vec<u8>,
        logits: Vec<f32>,
    }

    impl InferenceEngine for StubEngine {
        fn infer(
            &self,
            _input_ids: Vec<i64>,
            _diac_ids: Vec<i64>,
            _seq_length: usize,
        ) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
            Ok((self.target_ids.clone(), self.logits.clone()))
        }
    }

    #[test]
    fn do_tashkeel_errors_on_input_over_char_limit() {
        let too_long_text: String = "ا".repeat(CHAR_LIMIT + 1);
        let engine = StubEngine {
            target_ids: vec![],
            logits: vec![],
        };

        let result = do_tashkeel(&engine, &too_long_text, None, true);

        assert!(matches!(result, Err(DengjenTashkeelError::InputTooLong(n)) if n == CHAR_LIMIT));
    }

    #[test]
    fn do_tashkeel_errors_on_unknown_target_id_instead_of_panicking() {
        let engine = StubEngine {
            target_ids: vec![255],
            logits: vec![0.0],
        };

        let result = do_tashkeel(&engine, "ا", None, true);

        assert!(
            matches!(result, Err(DengjenTashkeelError::InferenceError(msg)) if msg.contains("unknown target id"))
        );
    }

    #[test]
    fn do_tashkeel_errors_when_model_returns_fewer_diacritics_than_input_chars() {
        let pad_id = *TARGET_ID_MAP
            .iter()
            .find(|(_, name)| name.as_str() == "_")
            .expect("target_id_map.json defines a PAD entry")
            .0;
        let engine = StubEngine {
            target_ids: vec![5, pad_id],
            logits: vec![0.0, 0.0],
        };

        let result = do_tashkeel(&engine, "با", None, true);

        assert!(
            matches!(result, Err(DengjenTashkeelError::InferenceError(msg)) if msg.contains("fewer diacritics than input characters"))
        );
    }

    #[test]
    fn do_tashkeel_handles_already_diacritized_input() {
        let already_diacritized = "بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيمِ";

        let result = do_tashkeel(&*INFERENCE_ENGINE, already_diacritized, None, false);

        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn create_inference_engine_errors_on_nonexistent_model_path() {
        let bad_path = std::path::PathBuf::from("/nonexistent/path/to/model.onnx");

        let result = create_inference_engine(Some(bad_path));

        assert!(matches!(
            result,
            Err(DengjenTashkeelError::InferenceError(_))
        ));
    }

    struct CountingEngine {
        infer_calls: std::sync::atomic::AtomicUsize,
        infer_batch_calls: std::sync::atomic::AtomicUsize,
    }

    impl CountingEngine {
        fn new() -> Self {
            Self {
                infer_calls: std::sync::atomic::AtomicUsize::new(0),
                infer_batch_calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    impl InferenceEngine for CountingEngine {
        fn infer(
            &self,
            _input_ids: Vec<i64>,
            _diac_ids: Vec<i64>,
            seq_length: usize,
        ) -> DengjenTashkeelResult<(Vec<u8>, Vec<f32>)> {
            self.infer_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok((vec![5; seq_length], vec![0.0; seq_length]))
        }

        fn infer_batch(
            &self,
            batch: Vec<(Vec<i64>, Vec<i64>, usize)>,
        ) -> DengjenTashkeelResult<Vec<(Vec<u8>, Vec<f32>)>> {
            self.infer_batch_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            batch
                .into_iter()
                .map(|(_, _, seq_length)| Ok((vec![5; seq_length], vec![0.0; seq_length])))
                .collect()
        }
    }

    #[test]
    fn infer_batch_default_implementation_calls_infer_once_per_item() {
        let engine = StubEngine {
            target_ids: vec![5],
            logits: vec![0.0],
        };

        let results = engine
            .infer_batch(vec![(vec![1], vec![0], 1), (vec![2], vec![0], 1)])
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn map_sentences_routes_through_infer_batch_instead_of_infer_per_sentence() {
        let engine = CountingEngine::new();
        let sentences = vec!["اب".to_string(), "تث".to_string(), "جح".to_string()];

        let results = map_sentences(&sentences, &engine, None).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(
            engine
                .infer_batch_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1,
            "map_sentences should call infer_batch exactly once for the whole paragraph"
        );
        assert_eq!(
            engine.infer_calls.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "map_sentences should not fall back to per-sentence infer"
        );
    }

    #[test]
    fn map_sentences_passes_through_sentences_that_tokenize_to_nothing() {
        let engine = CountingEngine::new();
        // "@@@" has no chars in INPUT_ID_MAP or ARABIC_DIACRITICS, so it tokenizes to a
        // zero-length sequence.
        let sentences = vec!["اب".to_string(), "@@@".to_string(), "جح".to_string()];

        let results = map_sentences(&sentences, &engine, None).unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(
            results[1], "@@@",
            "an untokenizable sentence passes through unchanged"
        );
        assert_eq!(
            engine
                .infer_batch_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn to_valid_chars_never_panics_and_only_keeps_known_chars(input in ".*") {
                let (valid, _invalid) = to_valid_chars(input.chars());
                for c in valid.chars() {
                    prop_assert!(
                        c == NUMERAL_SYMBOL || INPUT_ID_MAP.contains_key(&c) || ARABIC_DIACRITICS.contains(&c)
                    );
                }
                prop_assert!(valid.chars().count() <= input.chars().count());
            }

            // tokenize's input_ids/diac_ids pairing depends on this alignment holding
            // for any input, not just well-formed Arabic text.
            #[test]
            fn extract_chars_and_diacritics_never_panics_and_aligns_lengths(
                input in ".*",
                normalize in any::<bool>(),
            ) {
                let (chars, diacritics) = extract_chars_and_diacritics(&input, normalize);
                prop_assert_eq!(chars.chars().count(), diacritics.len());
            }

            #[test]
            fn tokenize_never_panics(input in ".*") {
                if let Ok(tok) = tokenize(&input) {
                    prop_assert_eq!(tok.seq_length, tok.input_ids.len());
                }
            }
        }
    }
}
