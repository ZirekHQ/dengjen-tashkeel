use std::env;

const LEGACY_MACROS: &str = "\
#ifndef DENGJEN_TASHKEEL_NO_LEGACY_MACROS
#define INPUT_TOO_LONG DENGJEN_TASHKEEL_INPUT_TOO_LONG
#define INFERENCE_ERROR DENGJEN_TASHKEEL_INFERENCE_ERROR
#define MODEL_LOAD_ERROR DENGJEN_TASHKEEL_MODEL_LOAD_ERROR
#define ALREADY_INITIALIZED DENGJEN_TASHKEEL_ALREADY_INITIALIZED
#define INVALID_ARGUMENT DENGJEN_TASHKEEL_INVALID_ARGUMENT
#define UNKNOWN_ERROR DENGJEN_TASHKEEL_UNKNOWN_ERROR
#endif";

fn main() {
    println!("cargo:rerun-if-changed=./src/lib.rs");
    println!("cargo:rerun-if-changed=./build.rs");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_include_version(true)
        .with_documentation(true)
        .with_parse_deps(true)
        .with_parse_include(&["ffi-support"])
        .with_cpp_compat(true)
        .with_language(cbindgen::Language::C)
        .include_item("ExternError")
        .with_trailer(LEGACY_MACROS)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("dengjen_tashkeel.h");
}
