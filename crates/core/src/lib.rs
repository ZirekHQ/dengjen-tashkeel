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
    // Read PAD's id from target_id_map.json's own vocabulary instead of
    // borrowing INPUT_ID_MAP's id for it: the two vocab files are maintained
    // independently, so nothing guarantees they agree on this value.
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
            .map(|i| unsafe { char::from_u32_unchecked(*i) }),
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

fn input_to_ids(input: impl Iterator<Item = char>) -> Vec<i64> {
    Vec::from_iter(input.map(|c| INPUT_ID_MAP[&c]))
}

fn hint_to_ids(hints: Vec<String>) -> Vec<i64> {
    Vec::from_iter(hints.into_iter().map(|s| HINT_ID_MAP[&s]))
}

// target_ids comes straight from the loaded ONNX model's output tensor
// (potentially a user-supplied model via `--onnx`/`dengjen_tashkeel_init`), so
// its values are untrusted: an id outside TARGET_ID_MAP's vocabulary must
// become an error here, not an index panic.
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

// `diacritics` must have exactly one entry per annotatable character in
// `input`; that invariant depends on the model producing a diacritic (or
// PAD, filtered out by target_to_diacritics) for every input position, so
// it can't be proven at compile time. Treat a mismatch as inference
// failure -- not a panic -- same untrusted-model-output rationale as above.
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

fn map_sentences(
    sentences: &[String],
    engine: &(impl InferenceEngine + Send + Sync),
    taskeen_threshold: Option<f32>,
) -> DengjenTashkeelResult<Vec<String>> {
    #[cfg(feature = "rayon")]
    {
        sentences
            .par_iter()
            .map(|sent| _do_tashkeel_impl(engine, sent, taskeen_threshold))
            .collect()
    }
    #[cfg(not(feature = "rayon"))]
    {
        sentences
            .iter()
            .map(|sent| _do_tashkeel_impl(engine, sent, taskeen_threshold))
            .collect()
    }
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
    let text = text.trim();

    if text.chars().count() > CHAR_LIMIT {
        return Err(DengjenTashkeelError::InputTooLong(CHAR_LIMIT));
    }

    let (input_text, removed_chars) = to_valid_chars(text.chars());
    let (input_text, diacritics) = extract_chars_and_diacritics(&input_text, true);

    let input_ids = input_to_ids(input_text.chars());
    let diac_ids = hint_to_ids(diacritics);
    let seq_length = input_ids.len();

    if seq_length > 0 {
        let timer = std::time::Instant::now();
        let (target_ids, logits) = engine.infer(input_ids, diac_ids, seq_length)?;
        let inference_ms = timer.elapsed().as_millis() as f32;
        log::debug!("Inference time: {} ms", inference_ms);
        let diacritics = target_to_diacritics(target_ids.into_iter())?;
        let final_text = if taskeen_threshold.is_none() {
            annotate_text_with_diacritics(text, diacritics, removed_chars)?
        } else {
            annotate_text_with_diacritics_taskeen(
                text,
                diacritics,
                removed_chars,
                logits,
                taskeen_threshold,
            )?
        };
        Ok(final_text)
    } else {
        log::debug!("Inference time: {} ms", 0.0);
        Ok(text.into())
    }
}

// ==============================
#[cfg(test)]
mod tests {
    use super::*;

    static INFERENCE_ENGINE: Lazy<DynamicInferenceEngine> =
        Lazy::new(|| create_inference_engine(None).unwrap());

    #[test]
    fn test_extract_diacritics_when_empty() {
        let (chars, diacritics) = extract_chars_and_diacritics("", false);
        assert_eq!(chars.is_empty(), true);
        assert_eq!(diacritics.is_empty(), true);
    }

    #[test]
    fn test_extract_diacritics() {
        let text = "بِسْمِ اللَّهِ الرَّحْمَنِ الرَّحِيمِ";
        let (chars, diacritics) = extract_chars_and_diacritics(text, true);

        assert_eq!(chars.chars().count(), diacritics.len());

        assert_eq!(chars.chars().nth(0), Some('ب'));
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

        assert_eq!(taskeen == no_taskeen, false);

        let sukoon = char::from_u32(0x652).unwrap();
        let no_taskeen_sukoon_count = no_taskeen.chars().filter(|c| c == &sukoon).count();
        let taskeen_sukoon_count = taskeen.chars().filter(|c| c == &sukoon).count();
        assert_eq!(taskeen_sukoon_count > no_taskeen_sukoon_count, true);

        Ok(())
    }
    #[test]
    fn test_hints() -> DengjenTashkeelResult<()> {
        let text = "بِسمِ اللّه الرّحمن الرّحيم ABC";
        do_tashkeel(&*INFERENCE_ENGINE, &text, None, false)?;
        let text = "مّنْ يُقلِّب  ABC";
        do_tashkeel(&*INFERENCE_ENGINE, &text, None, false)?;
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
        // The CHAR_LIMIT check in _do_tashkeel_impl runs before engine.infer
        // is ever called, so this never needs the real model -- a stub with
        // no configured output still proves the guard fires. preprocessed =
        // true skips libtqsm sentence segmentation, so the full length
        // reaches the check directly.
        let engine = StubEngine {
            target_ids: vec![],
            logits: vec![],
        };

        let result = do_tashkeel(&engine, &too_long_text, None, true);

        assert!(matches!(result, Err(DengjenTashkeelError::InputTooLong(n)) if n == CHAR_LIMIT));
    }

    #[test]
    fn do_tashkeel_errors_on_unknown_target_id_instead_of_panicking() {
        // 255 is not a valid key in target_id_map.json's value set.
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
        // Two input characters ("با") need two diacritics; the stub returns a
        // real diacritic (5 = fatha, per target_id_map.json) for the first slot
        // and PAD (filtered out by target_to_diacritics) for the second, so the
        // annotation loop runs out one character short -- on the second char,
        // not the first.
        let pad_id = *TARGET_META_CHAR_IDS.iter().next().unwrap();
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
}
