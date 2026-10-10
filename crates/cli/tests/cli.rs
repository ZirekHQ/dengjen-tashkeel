use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_dengjen-tashkeel");

#[test]
fn help_flag_prints_usage_and_exits_successfully() {
    let output = Command::new(BIN).arg("--help").output().unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("--input-file"));
}

#[test]
fn interactive_mode_with_an_input_file_exits_with_an_error() {
    let output = Command::new(BIN)
        .args(["--input-file", "in.txt", "--interactive"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Interactive mode"));
}

#[test]
fn missing_model_file_exits_with_an_error() {
    let output = Command::new(BIN)
        .args(["--onnx", "/nonexistent/model.onnx"])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn stdin_line_is_diacritized_to_stdout() {
    let mut child = Command::new(BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all("بسم الله الرحمن الرحيم\n".as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("بِسْمِ"));
}
