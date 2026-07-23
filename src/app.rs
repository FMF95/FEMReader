use crate::assembler::DefaultModelAssembler;
use crate::cache::IncrementalCache;
use crate::dependency::DefaultDependencyResolver;
use crate::pipeline::ReaderPipeline;
use crate::scanner::DefaultFileSystemScanner;
use crate::serializer::JsonSerializer;
use crate::solver::{AbaqusAdapter, NastranAdapter, OptiStructAdapter, SolverAdapter};
use std::path::PathBuf;

pub struct ReaderRunConfig {
    pub input_dir: PathBuf,
    pub pretty: bool,
    pub delete_cache: bool,
}

pub fn run_reader(config: &ReaderRunConfig) -> Result<String, String> {
    if !config.input_dir.exists() || !config.input_dir.is_dir() {
        return Err(format!(
            "Input directory does not exist or is not a directory: {}",
            config.input_dir.display()
        ));
    }

    let mut cache_path = config.input_dir.clone();
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
        .run_and_serialize(&config.input_dir, &mut cache, config.pretty)
        .map_err(|e| format!("Reader execution failed: {e}"))?;

    if config.delete_cache {
        IncrementalCache::remove_file(&cache_path)
            .map_err(|e| format!("Unable to delete cache {}: {e}", cache_path.display()))?;
    } else {
        cache
            .save(&cache_path)
            .map_err(|e| format!("Unable to save cache {}: {e}", cache_path.display()))?;
    }

    Ok(output)
}
