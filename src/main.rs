//! forge – command-line HTML → PDF converter.
//!
//! Usage:
//!   forge <input.html> [output.pdf] [--landscape] [--title "My Report"]
//!
//! If `output.pdf` is omitted the PDF is written next to the input file with
//! the same stem (e.g. `report.html` → `report.pdf`).

use std::{env, fs, path::PathBuf, process};

use pdf_forge::pipeline::{generate_pdf, PageOrientation, PipelineConfig};

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    let mut input_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;
    let mut landscape = false;
    let mut title: Option<String> = None;
    let mut positional = 0usize;

    let mut iter = args.iter().skip(1).peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--landscape" | "-l" => landscape = true,
            "--title" | "-t" => match iter.next() {
                Some(v) => title = Some(v.clone()),
                None => {
                    title = Some("Template".to_string())
                }
            },
            "--help" | "-h" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            other if other.starts_with('-') => {
                eprintln!("Unknown flag: {other}");
                print_usage(&args[0]);
                process::exit(1);
            }
            path => {
                if positional == 0 {
                    input_path = Some(PathBuf::from(path));
                } else if positional == 1 {
                    output_path = Some(PathBuf::from(path));
                } else {
                    eprintln!("Unexpected argument: {path}");
                    print_usage(&args[0]);
                    process::exit(1);
                }
                positional += 1;
            }
        }
    }

    let input = match input_path {
        Some(p) => p,
        None => {
            eprintln!("Error: no input file specified.");
            print_usage(&args[0]);
            process::exit(1);
        }
    };

    // Default output: same directory + same stem as input, but with .pdf
    let output = output_path.unwrap_or_else(|| {
        let mut o = input.clone();
        o.set_extension("pdf");
        o
    });

    let html = match fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {e}", input.display());
            process::exit(1);
        }
    };

    // Default title: stem of the input filename.
    let default_title = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("rpdf output")
        .to_string();

    let config = PipelineConfig {
        title: title.unwrap_or(default_title),
        orientation: if landscape {
            PageOrientation::Landscape
        } else {
            PageOrientation::Portrait
        },
        ..PipelineConfig::default()
    };

    match generate_pdf(&html, &config) {
        Ok((bytes, layout)) => {
            // Create output directory if necessary.
            if let Some(parent) = output.parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        eprintln!("Error creating output directory: {e}");
                        process::exit(1);
                    }
                }
            }
            if let Err(e) = fs::write(&output, &bytes) {
                eprintln!("Error writing '{}': {e}", output.display());
                process::exit(1);
            }
            let pages = layout.pages.len();
            eprintln!(
                "Wrote '{}' ({} bytes, {} page{})",
                output.display(),
                bytes.len(),
                pages,
                if pages == 1 { "" } else { "s" }
            );
        }
        Err(e) => {
            eprintln!("Error generating PDF: {e}");
            process::exit(1);
        }
    }
}

fn print_usage(prog: &str) {
    eprintln!("forge – HTML to PDF converter (pdf-forge)");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {prog} <input.html> [output.pdf] [--landscape] [--title \"My Report\"]");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <input.html>   HTML file to convert (images must be base64 data URIs; others are skipped)");
    eprintln!("  [output.pdf]   Output path  (default: same stem as input with .pdf)");
    eprintln!();
    eprintln!("Flags:");
    eprintln!("  --title, -t    Document title in PDF metadata (default: input filename stem)");
    eprintln!("  --landscape    Use landscape page orientation (A4 841×595 pt)");
    eprintln!("  --help         Print this message");
}
