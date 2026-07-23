use femreader::app::{ReaderRunConfig, run_reader};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return Ok(());
    }

    let input_dir = PathBuf::from(&args[1]);

    let mut output_file: Option<PathBuf> = None;
    let mut pretty = false;
    let mut delete_cache = false;

    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--output" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("--output requires a file path".to_string());
                };
                output_file = Some(PathBuf::from(path));
                index += 2;
            }
            "--pretty" => {
                pretty = true;
                index += 1;
            }
            "--delete-cache" => {
                delete_cache = true;
                index += 1;
            }
            unknown => {
                return Err(format!("Unknown argument: {unknown}"));
            }
        }
    }

    let output = run_reader(&ReaderRunConfig {
        input_dir,
        pretty,
        delete_cache,
    })?;

    if let Some(path) = output_file {
        fs::write(&path, output)
            .map_err(|e| format!("Unable to write output file {}: {e}", path.display()))?;
    } else {
        println!("{output}");
    }

    Ok(())
}

fn print_help() {
    println!("femreader <input_directory> [--output <output_file>] [--pretty] [--delete-cache]");
    println!("  <input_directory>   Root directory to scan");
    println!("  --output <file>     Write serialized model to a file");
    println!("  --pretty            Pretty-print JSON output");
    println!("  --delete-cache      Remove .femreader-cache after reading models");
    println!(
        "  GUI                 cargo run --features gui --bin femreader_gui -- <input_directory>"
    );
}
