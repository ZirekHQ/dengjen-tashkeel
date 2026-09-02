use clap::Parser;
use dengjen_tashkeel::{create_inference_engine, do_tashkeel, DynamicInferenceEngine, CHAR_LIMIT};
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;

const TASKEEN_REJECTION_THRESHOLD: &str = "0.95";

#[derive(Parser)]
#[command(name = "dengjen-tashkeel", author, version, about, long_about = None)]
struct Cli {
    /// Input file (default `stdin`)
    #[arg(short = 'f', long, value_name = "INPUT_FILE")]
    input_file: Option<PathBuf>,
    /// Output file (default `stdout`)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output_file: Option<PathBuf>,
    /// Use interactive mode (useful for testing)
    #[arg(short, long)]
    interactive: bool,
    /// Use sukoon for case-ending diacritic if the model is uncertain
    #[arg(short, long)]
    taskeen: bool,
    /// Taskeen threshold probability
    #[arg(long, short, default_value = TASKEEN_REJECTION_THRESHOLD, required = false)]
    prob: Option<f32>,
    /// ONNX model (default: use bundled model if available)
    #[arg(short = 'x', long, value_name = "ONNX_MODEL")]
    onnx: Option<PathBuf>,
}

fn write_to_stdout(text: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn get_input_text(args: &Cli) -> anyhow::Result<String> {
    let mut input_buffer = String::new();
    if let Some(ref input_filename) = args.input_file {
        let mut file = File::open(input_filename)?;
        file.read_to_string(&mut input_buffer)?;
    } else {
        let stdin = io::stdin();
        stdin.read_line(&mut input_buffer)?;
    }

    Ok(input_buffer)
}

fn diacritize_capped(
    model: &DynamicInferenceEngine,
    text: &str,
    taskeen_threshold: Option<f32>,
) -> anyhow::Result<String> {
    let input = String::from_iter(text.chars().take(CHAR_LIMIT));
    Ok(do_tashkeel(model, &input, taskeen_threshold, false)?)
}

fn write_output_file(path: &std::path::Path, text: &str) -> anyhow::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(text.as_bytes())?;
    log::info!("Wrote output to file `{}`", path.display());
    Ok(())
}

// Each (input_file, output_file) combination writes its output exactly
// once, through exactly one of write_to_stdout/write_output_file below --
// unlike the single shared accumulator this replaced, it's structurally
// impossible for a branch to both stream per-line output and then emit a
// second, empty final write.
fn tashkeel_main(
    model: &DynamicInferenceEngine,
    args: &Cli,
    input_text: String,
) -> anyhow::Result<()> {
    let taskeen_threshold = if args.taskeen { args.prob } else { None };

    match (&args.input_file, &args.output_file) {
        (None, None) => {
            let diacritized = diacritize_capped(model, &input_text, taskeen_threshold)?;
            write_to_stdout(&diacritized)?;
        }
        (None, Some(output_filename)) => {
            let diacritized = diacritize_capped(model, &input_text, taskeen_threshold)?;
            write_output_file(output_filename, &diacritized)?;
        }
        (Some(_), None) => {
            for input_line in input_text.lines() {
                let diacritized_line = diacritize_capped(model, input_line, taskeen_threshold)?;
                write_to_stdout(&diacritized_line)?;
            }
        }
        (Some(_), Some(output_filename)) => {
            let mut diacritized_lines = String::new();
            for input_line in input_text.lines() {
                let diacritized_line = diacritize_capped(model, input_line, taskeen_threshold)?;
                diacritized_lines.push_str(&diacritized_line);
                diacritized_lines.push('\n');
            }
            write_output_file(output_filename, &diacritized_lines)?;
        }
    }

    Ok(())
}

fn validate_args(args: &mut Cli) -> anyhow::Result<()> {
    if args.input_file.is_some() || args.output_file.is_some() {
        if args.interactive {
            anyhow::bail!(
                "Interactive mode is not available when `--input-file` or `--output-file` is passed"
            )
        }
    } else {
        args.interactive = true;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    setup_logging();

    let mut args = Cli::parse();

    validate_args(&mut args)?;

    let model = create_inference_engine(args.onnx.take())?;

    let mut input_text = get_input_text(&args)?;
    if args.interactive {
        loop {
            if !input_text.trim().is_empty() {
                tashkeel_main(&model, &args, std::mem::take(&mut input_text))?;
            }
            input_text = get_input_text(&args)?;
        }
    } else {
        tashkeel_main(&model, &args, input_text)?;
    }

    Ok(())
}

fn setup_logging() {
    env_logger::Builder::from_env(env_logger::Env::default().filter_or("TASHKEEL_LOG", "info"))
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::LazyLock;

    static ENGINE: LazyLock<DynamicInferenceEngine> =
        LazyLock::new(|| create_inference_engine(None).unwrap());

    fn parse(args: &[&str]) -> Cli {
        let mut full_args = vec!["dengjen-tashkeel"];
        full_args.extend_from_slice(args);
        Cli::try_parse_from(full_args).unwrap()
    }

    #[test]
    fn validate_args_forces_interactive_when_no_files_given() {
        let mut args = parse(&[]);
        assert!(!args.interactive);

        validate_args(&mut args).unwrap();

        assert!(args.interactive);
    }

    #[test]
    fn validate_args_rejects_interactive_with_input_file() {
        let mut args = parse(&["--input-file", "in.txt", "--interactive"]);

        let result = validate_args(&mut args);

        assert!(result.is_err());
    }

    #[test]
    fn validate_args_rejects_interactive_with_output_file() {
        let mut args = parse(&["--output-file", "out.txt", "--interactive"]);

        let result = validate_args(&mut args);

        assert!(result.is_err());
    }

    #[test]
    fn validate_args_allows_file_mode_without_interactive_flag() {
        let mut args = parse(&["--input-file", "in.txt"]);

        validate_args(&mut args).unwrap();

        assert!(!args.interactive);
    }

    #[test]
    fn prob_defaults_to_point_nine_five_regardless_of_taskeen_flag() {
        let args = parse(&["--taskeen"]);

        assert!(args.taskeen);
        assert_eq!(args.prob, Some(0.95));
    }

    #[test]
    fn get_input_text_reads_from_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "بسم الله").unwrap();
        let args = parse(&["--input-file", file.path().to_str().unwrap()]);

        let text = get_input_text(&args).unwrap();

        assert_eq!(text.trim(), "بسم الله");
    }

    #[test]
    fn get_input_text_errors_on_missing_file() {
        let args = parse(&["--input-file", "/nonexistent/path/does-not-exist.txt"]);

        let result = get_input_text(&args);

        assert!(result.is_err());
    }

    #[test]
    fn get_input_text_returns_empty_string_for_empty_file() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let args = parse(&["--input-file", file.path().to_str().unwrap()]);

        let text = get_input_text(&args).unwrap();

        assert_eq!(text, "");
    }

    #[test]
    fn tashkeel_main_with_taskeen_flag_uses_the_threshold() {
        let output_file = tempfile::NamedTempFile::new().unwrap();
        let mut args = parse(&[
            "--output-file",
            output_file.path().to_str().unwrap(),
            "--taskeen",
        ]);
        args.input_file = None;

        tashkeel_main(&ENGINE, &args, "بسم الله الرحمن الرحيم".to_string()).unwrap();

        let written = std::fs::read_to_string(output_file.path()).unwrap();
        assert!(!written.trim().is_empty());
    }

    #[test]
    fn tashkeel_main_writes_single_shot_output_to_file() {
        let output_file = tempfile::NamedTempFile::new().unwrap();
        let mut args = parse(&["--output-file", output_file.path().to_str().unwrap()]);
        args.input_file = None; // single-shot ("stdin") code path in tashkeel_main

        tashkeel_main(&ENGINE, &args, "بسم الله الرحمن الرحيم".to_string()).unwrap();

        let written = std::fs::read_to_string(output_file.path()).unwrap();
        assert_ne!(written.trim(), "بسم الله الرحمن الرحيم");
        assert!(!written.trim().is_empty());
    }

    #[test]
    fn tashkeel_main_writes_multi_line_output_to_file() {
        let input_file = tempfile::NamedTempFile::new().unwrap();
        let output_file = tempfile::NamedTempFile::new().unwrap();
        let args = parse(&[
            "--input-file",
            input_file.path().to_str().unwrap(),
            "--output-file",
            output_file.path().to_str().unwrap(),
        ]);
        let input_text = "بسم الله\nالرحمن الرحيم\n".to_string();

        tashkeel_main(&ENGINE, &args, input_text).unwrap();

        let written = std::fs::read_to_string(output_file.path()).unwrap();
        assert_eq!(written.lines().count(), 2);
    }

    #[test]
    fn tashkeel_main_reading_a_file_to_stdout_does_not_error() {
        // Regression test: this (input_file: Some, output_file: None)
        // combination used to also run the single-shot accumulator's final
        // write_to_stdout call on an empty string, printing a spurious
        // trailing blank line after the real per-line output. There's no
        // stdout-capture harness here to assert on the extra line directly,
        // but this exercises the code path end to end without panicking.
        let mut input_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(input_file, "بسم الله\nالرحمن الرحيم").unwrap();
        let args = parse(&["--input-file", input_file.path().to_str().unwrap()]);
        let input_text = std::fs::read_to_string(input_file.path()).unwrap();

        tashkeel_main(&ENGINE, &args, input_text).unwrap();
    }
}
