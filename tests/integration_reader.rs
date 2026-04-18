use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
use femreader::model::SolverKind;
use femreader::pipeline::ReaderPipeline;
use femreader::scanner::{DefaultFileSystemScanner, FileSystemScanner};
use femreader::serializer::JsonSerializer;
use femreader::solver::{AbaqusAdapter, NastranAdapter, OptiStructAdapter, SolverAdapter};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.push(format!("femreader_test_{name}_{nanos}"));
    path
}

#[test]
fn builds_multiple_models_with_shared_dependency() {
    let root = unique_temp_dir("shared_dep");
    fs::create_dir_all(&root).expect("create temp root");

    let a = root.join("model_a.bdf");
    let b = root.join("model_b.bdf");
    let shared = root.join("shared.inc");

    fs::write(&a, "INCLUDE \"shared.inc\"\nGRID,1").expect("write a");
    fs::write(&b, "INCLUDE \"shared.inc\"\nGRID,2").expect("write b");
    fs::write(&shared, "$ shared content").expect("write shared");

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

    let mut cache = IncrementalCache::default();
    let content = pipeline.run(&root, &mut cache).expect("pipeline run");

    assert_eq!(content.models.len(), 2);
    assert_eq!(content.dependencies.len(), 2);

    let shared_id = format!(
        "file:{}",
        fs::canonicalize(&shared)
            .expect("canonical shared")
            .display()
    );

    let models_using_shared = content
        .models
        .iter()
        .filter(|model| model.file_ids.contains(&shared_id))
        .count();

    assert_eq!(models_using_shared, 2);

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn scanner_uses_incremental_cache() {
    let root = unique_temp_dir("incremental_cache");
    fs::create_dir_all(&root).expect("create temp root");

    let file_path = root.join("single.bdf");
    fs::write(&file_path, "GRID,1").expect("write test file");

    let scanner = DefaultFileSystemScanner;
    let mut cache = IncrementalCache::default();

    let first = scanner.scan(&root, &mut cache).expect("first scan");
    assert!(first.files.iter().all(|f| f.changed_since_last_scan));

    let second = scanner.scan(&root, &mut cache).expect("second scan");
    assert!(second.files.iter().all(|f| !f.changed_since_last_scan));

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn detects_solver_per_directory_from_extensions() {
    let root = unique_temp_dir("solver_detection");
    let nastran_dir = root.join("nastran");
    let optistruct_dir = root.join("optistruct");
    let abaqus_dir = root.join("abaqus");

    fs::create_dir_all(&nastran_dir).expect("create nastran dir");
    fs::create_dir_all(&optistruct_dir).expect("create optistruct dir");
    fs::create_dir_all(&abaqus_dir).expect("create abaqus dir");

    fs::write(nastran_dir.join("model.bdf"), "GRID,1").expect("write nastran model");
    fs::write(optistruct_dir.join("model.fem"), "GRID,1").expect("write optistruct model");
    fs::write(abaqus_dir.join("model.inp"), "*NODE").expect("write abaqus model");

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

    let mut cache = IncrementalCache::default();
    let content = pipeline.run(&root, &mut cache).expect("pipeline run");

    for file in &content.files {
        match file.extension.as_deref() {
            Some("bdf") => assert_eq!(file.solver_hint, SolverKind::Nastran),
            Some("fem") => assert_eq!(file.solver_hint, SolverKind::OptiStruct),
            Some("inp") => assert_eq!(file.solver_hint, SolverKind::Abaqus),
            _ => {}
        }
    }

    let mut model_solvers: Vec<SolverKind> = content.models.iter().map(|m| m.solver.clone()).collect();
    model_solvers.sort_by_key(|solver| match solver {
        SolverKind::Unknown => 0,
        SolverKind::Nastran => 1,
        SolverKind::OptiStruct => 2,
        SolverKind::Abaqus => 3,
    });

    assert_eq!(
        model_solvers,
        vec![SolverKind::Nastran, SolverKind::OptiStruct, SolverKind::Abaqus]
    );

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn keeps_solver_unknown_when_directory_has_mixed_solver_extensions() {
    let root = unique_temp_dir("mixed_solver_detection");
    fs::create_dir_all(&root).expect("create temp root");

    fs::write(root.join("mixed_model.fem"), "GRID,1").expect("write fem");
    fs::write(root.join("mixed_model.inp"), "*NODE").expect("write inp");

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

    let mut cache = IncrementalCache::default();
    let content = pipeline.run(&root, &mut cache).expect("pipeline run");

    assert!(
        content
            .files
            .iter()
            .all(|file| file.solver_hint == SolverKind::Unknown)
    );
    assert!(
        content
            .models
            .iter()
            .all(|model| model.solver == SolverKind::Unknown)
    );

    fs::remove_dir_all(&root).expect("cleanup temp root");
}
