use std::env;
use std::path::PathBuf;

fn main() {
    // Only regenerate the header when the FFI source changes.
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let output_file = PathBuf::from(&crate_dir).join("include").join("rpdf.h");

    // Ensure the include/ directory exists.
    std::fs::create_dir_all(output_file.parent().expect("no parent for output_file"))
        .expect("failed to create include/ directory");

    // Generate the C header.
    let config = cbindgen::Config::from_file(PathBuf::from(&crate_dir).join("cbindgen.toml"))
        .expect("failed to read cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("cbindgen failed to generate bindings")
        .write_to_file(&output_file);

    println!(
        "cargo:warning=C header written to {}",
        output_file.display()
    );
}
