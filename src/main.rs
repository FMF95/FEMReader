use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
use femreader::pipeline::ReaderPipeline;
use femreader::scanner::DefaultFileSystemScanner;
use femreader::serializer::JsonSerializer;
use femreader::solver::{AbaqusAdapter, NastranAdapter, OptiStructAdapter, SolverAdapter};
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
    if !input_dir.exists() || !input_dir.is_dir() {
        return Err(format!(
            "Input directory does not exist or is not a directory: {}",
            input_dir.display()
        ));
    }

    let mut output_file: Option<PathBuf> = None;
    let mut pretty = false;

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
            unknown => {
                return Err(format!("Unknown argument: {unknown}"));
            }
        }
    }

    let mut cache_path = input_dir.clone();
    cache_path.push(".femreader-cache");

    let mut cache = IncrementalCache::load(&cache_path)
        .map_err(|e| format!("Unable to load cache {}: {e}", cache_path.display()))?;

    let adapters: Vec<Box<dyn SolverAdapter + Send + Sync>> = vec![
        Box::new(NastranAdapter),
        Box::new(OptiStructAdapter),
        Box::new(AbaqusAdapter),
    ];

    let pipeline = ReaderPipeline {
        scanner: DefaultFileSystemScanner,
        dependency_resolver: DefaultDependencyResolver::new(adapters),
        model_assembler: DefaultModelAssembler,
        serializer: JsonSerializer,
    };

    let output = pipeline
        .run_and_serialize(&input_dir, &mut cache, pretty)
        .map_err(|e| format!("Reader execution failed: {e}"))?;

    cache
        .save(&cache_path)
        .map_err(|e| format!("Unable to save cache {}: {e}", cache_path.display()))?;

    if let Some(path) = output_file {
        fs::write(&path, output)
            .map_err(|e| format!("Unable to write output file {}: {e}", path.display()))?;
    } else {
        println!("{output}");
    }

    Ok(())
}

fn print_help() {
    println!("femreader <input_directory> [--output <output_file>] [--pretty]");
    println!("  <input_directory>   Root directory to scan");
    println!("  --output <file>     Write serialized model to a file");
    println!("  --pretty            Pretty-print JSON output");
    println!("  GUI                 cargo run --features gui --bin femreader_gui -- <input_directory>");
}
