use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
use femreader::model::{
    FileCategory, HyperMeshCommentFields, ModelContent, OptiStructCardKind, OptiStructCommentKind,
    OptiStructItem, SolverKind,
};
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

fn file_id_for_path(content: &ModelContent, path: &std::path::Path) -> String {
    let canonical = fs::canonicalize(path).expect("canonical path");
    content
        .files
        .iter()
        .find(|file| file.path == canonical)
        .map(|file| file.id.clone())
        .expect("file id for path")
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

    let shared_id = file_id_for_path(&content, &shared);

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

    let mut model_solvers: Vec<SolverKind> =
        content.models.iter().map(|m| m.solver.clone()).collect();
    model_solvers.sort_by_key(|solver| match solver {
        SolverKind::Unknown => 0,
        SolverKind::Nastran => 1,
        SolverKind::OptiStruct => 2,
        SolverKind::Abaqus => 3,
    });

    assert_eq!(
        model_solvers,
        vec![
            SolverKind::Nastran,
            SolverKind::OptiStruct,
            SolverKind::Abaqus
        ]
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

#[test]
fn differentiates_model_and_non_model_files_and_relates_only_model_files() {
    let root = unique_temp_dir("file_category_filtering");
    fs::create_dir_all(&root).expect("create temp root");

    let main_model = root.join("main.fem");
    let extensionless_model = root.join("submodel");
    let non_model = root.join("notes.txt");

    fs::write(
        &main_model,
        "INCLUDE \"submodel\"\nINCLUDE \"notes.txt\"\nGRID    1       0       0.0     0.0     0.0\n",
    )
    .expect("write main model");
    fs::write(
        &extensionless_model,
        "GRID    2       0       1.0     0.0     0.0\n",
    )
    .expect("write extensionless model");
    fs::write(&non_model, "INCLUDE \"submodel\"\n").expect("write non model file");

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

    let main_id = file_id_for_path(&content, &main_model);
    let extensionless_id = file_id_for_path(&content, &extensionless_model);
    let non_model_id = file_id_for_path(&content, &non_model);

    let main_file = content
        .files
        .iter()
        .find(|file| file.id == main_id)
        .expect("main file should exist");
    let extensionless_file = content
        .files
        .iter()
        .find(|file| file.id == extensionless_id)
        .expect("extensionless file should exist");
    let non_model_file = content
        .files
        .iter()
        .find(|file| file.id == non_model_id)
        .expect("non model file should exist");

    assert_eq!(main_file.category, FileCategory::Model);
    assert_eq!(extensionless_file.category, FileCategory::Model);
    assert_eq!(non_model_file.category, FileCategory::NonModel);

    assert_eq!(content.dependencies.len(), 1);
    assert_eq!(content.dependencies[0].from_file_id, main_id);
    assert_eq!(content.dependencies[0].to_file_id, extensionless_id);

    assert!(
        content
            .models
            .iter()
            .all(|model| !model.file_ids.contains(&non_model_id))
    );

    assert!(
        content
            .unresolved_references
            .iter()
            .any(|item| item.contains("notes.txt"))
    );

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn can_delete_cache_file_after_reading() {
    let root = unique_temp_dir("delete_cache");
    fs::create_dir_all(&root).expect("create temp root");

    let cache_path = root.join(".femreader-cache");
    let mut cache = IncrementalCache::default();
    cache.update("file:dummy".to_string(), 1, 1);
    cache.save(&cache_path).expect("save cache");
    assert!(cache_path.exists());

    IncrementalCache::remove_file(&cache_path).expect("remove cache file");
    assert!(!cache_path.exists());

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn parses_optistruct_cards_with_positions_and_comments() {
    let root = unique_temp_dir("optistruct_cards");
    fs::create_dir_all(&root).expect("create temp root");

    let main_model = root.join("main.fem");
    let sub_model = root.join("submodel.fem");
    fs::write(
        &main_model,
        "\
BEGIN BULK
$HMNAME COMP 1\"COMPONENT_1\" MAIN
$HMMOVE        1
$              1THRU          98     100      99
GRID    100     0       1.0     2.0     3.0
CQUAD4  200     10      1       2       3       4
CTRIA3  201     20      1       3       4
FORCE   1       100     0       500.0   1.0     0.0     0.0
MOMENT  2       100     0       300.0   0.0     1.0     0.0
INCLUDE \"submodel.fem\" $ inline include
ENDDATA
",
    )
    .expect("write main model");
    fs::write(&sub_model, "GRID    101     0       9.0     9.0     9.0\n").expect("write include");

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

    assert!(!content.optistruct_items.is_empty());

    let main_id = file_id_for_path(&content, &main_model);

    let main_items = content
        .optistruct_items
        .iter()
        .filter(|item| match item {
            OptiStructItem::Comment(comment) => comment.location.file_id == main_id,
            OptiStructItem::Text(text) => text.location.file_id == main_id,
            OptiStructItem::Card(card) => card.location.file_id == main_id,
        })
        .collect::<Vec<_>>();

    assert!(
        main_items
            .iter()
            .any(|item| matches!(item, OptiStructItem::Comment(_)))
    );
    assert!(main_items.iter().any(|item| matches!(
        item,
        OptiStructItem::Comment(comment)
            if matches!(
                comment.kind,
                OptiStructCommentKind::HyperMeshComment {
                    ref tag,
                    fields: HyperMeshCommentFields::HMName(ref hmname)
                } if tag == "HMNAME"
                    && hmname.entity_type.as_deref() == Some("COMP")
                    && hmname.id.as_deref() == Some("1")
                    && hmname.name.as_deref() == Some("COMPONENT_1")
                    && hmname.id_pool.as_deref() == Some("MAIN")
            )
    )));
    assert!(main_items.iter().any(|item| matches!(
        item,
        OptiStructItem::Comment(comment)
            if matches!(
                comment.kind,
                OptiStructCommentKind::HyperMeshComment {
                    ref tag,
                    fields: HyperMeshCommentFields::HMMove(ref hmmove)
                } if tag == "HMMOVE"
                    && hmmove.collector_id.as_deref() == Some("1")
                    && hmmove.raw_fields
                        == vec![
                            "1".to_string(),
                            "1THRU".to_string(),
                            "98".to_string(),
                            "100".to_string(),
                            "99".to_string()
                        ]
            )
    )));
    assert!(main_items.iter().any(|item| {
        matches!(
            item,
            OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::BeginBulk(_))
        )
    }));
    assert!(
        main_items.iter().any(
            |item| matches!(item, OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::Grid(_)))
        )
    );
    assert!(
        main_items.iter().any(
            |item| matches!(item, OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::Cquad4(_)))
        )
    );
    assert!(
        main_items.iter().any(
            |item| matches!(item, OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::Ctria3(_)))
        )
    );
    assert!(
        main_items.iter().any(
            |item| matches!(item, OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::Force(_)))
        )
    );
    assert!(
        main_items.iter().any(
            |item| matches!(item, OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::Moment(_)))
        )
    );
    assert!(
        main_items.iter().any(
            |item| matches!(item, OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::Include(_)))
        )
    );
    assert!(main_items.iter().any(|item| {
        matches!(
            item,
            OptiStructItem::Card(card) if matches!(card.kind, OptiStructCardKind::EndData(_))
        )
    }));

    let include_card = main_items
        .iter()
        .find_map(|item| match item {
            OptiStructItem::Card(card) => match &card.kind {
                OptiStructCardKind::Include(include) => Some((card, include)),
                _ => None,
            },
            OptiStructItem::Comment(_) => None,
            OptiStructItem::Text(_) => None,
        })
        .expect("include card should exist");

    assert_eq!(include_card.0.location.start_line, 10);
    assert_eq!(include_card.1.target, "submodel.fem");

    assert!(
        content
            .dependencies
            .iter()
            .any(|edge| edge.from_file_id == main_id && edge.reference == "submodel.fem")
    );
    let main_summary = content
        .optistruct_file_summaries
        .iter()
        .find(|summary| summary.file_id == main_id)
        .expect("main file summary should exist");
    assert_eq!(main_summary.card_items, 8);
    assert_eq!(main_summary.comment_items, 3);
    assert_eq!(main_summary.hypermesh_comment_items, 2);
    assert_eq!(main_summary.text_items, 0);
    assert!(
        main_summary
            .card_counts
            .iter()
            .any(|entry| entry.card_name == "CQUAD4" && entry.count == 1)
    );

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn ignores_internal_cache_file_from_scan_and_models() {
    let root = unique_temp_dir("ignore_cache_file");
    fs::create_dir_all(&root).expect("create temp root");

    let model = root.join("model.fem");
    let cache_file = root.join(".femreader-cache");
    fs::write(&model, "GRID    1       0       0.0     0.0     0.0\n").expect("write model");
    fs::write(&cache_file, "dummy cache content").expect("write cache-like file");

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

    assert!(content.files.iter().all(
        |file| file.path.file_name().and_then(|name| name.to_str()) != Some(".femreader-cache")
    ));
    assert!(content.models.iter().all(|model_node| {
        model_node
            .file_ids
            .iter()
            .all(|id| !id.ends_with(".femreader-cache"))
    }));

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn generates_unique_ids_and_populates_name_fields() {
    let root = unique_temp_dir("unique_ids_names");
    fs::create_dir_all(&root).expect("create temp root");

    let main_model = root.join("main.fem");
    let include_model = root.join("include.fem");
    fs::write(
        &main_model,
        "BEGIN   BULK\n$HMNAME COMP 1 \"panel\" MAIN\nINCLUDE \"include.fem\"\nGRID    1       0       0.0     0.0     0.0\nENDDATA\n",
    )
    .expect("write main model");
    fs::write(
        &include_model,
        "GRID    2       0       1.0     0.0     0.0\n",
    )
    .expect("write include model");

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

    let mut directory_ids = std::collections::HashSet::new();
    for directory in &content.directories {
        assert!(directory_ids.insert(directory.id.clone()));
        assert!(!directory.id.is_empty());
        assert!(directory.id.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(
            directory.name,
            directory
                .path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| directory.path.display().to_string())
        );
    }

    let mut file_ids = std::collections::HashSet::new();
    for file in &content.files {
        assert!(file_ids.insert(file.id.clone()));
        assert!(!file.id.is_empty());
        assert!(file.id.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(
            file.name,
            file.path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| file.path.display().to_string())
        );
    }

    let mut dependency_ids = std::collections::HashSet::new();
    for dependency in &content.dependencies {
        assert!(dependency_ids.insert(dependency.id.clone()));
        assert!(!dependency.id.is_empty());
        assert!(dependency.id.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    let mut model_ids = std::collections::HashSet::new();
    for model in &content.models {
        assert!(model_ids.insert(model.id.clone()));
        assert!(!model.id.is_empty());
        assert!(model.id.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    let mut item_ids = std::collections::HashSet::new();
    for item in &content.optistruct_items {
        match item {
            OptiStructItem::Comment(comment) => {
                assert!(item_ids.insert(comment.id.clone()));
                assert!(comment.id.chars().all(|ch| ch.is_ascii_hexdigit()));
                assert_eq!(
                    comment.location.file_name,
                    comment
                        .location
                        .file_path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| comment.location.file_path.display().to_string())
                );
            }
            OptiStructItem::Text(text) => {
                assert!(item_ids.insert(text.id.clone()));
                assert!(text.id.chars().all(|ch| ch.is_ascii_hexdigit()));
                assert_eq!(
                    text.location.file_name,
                    text.location
                        .file_path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| text.location.file_path.display().to_string())
                );
            }
            OptiStructItem::Card(card) => {
                assert!(item_ids.insert(card.id.clone()));
                assert!(card.id.chars().all(|ch| ch.is_ascii_hexdigit()));
                assert_eq!(
                    card.location.file_name,
                    card.location
                        .file_path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| card.location.file_path.display().to_string())
                );
            }
        }
    }

    fs::remove_dir_all(&root).expect("cleanup temp root");
}

#[test]
fn preserves_unparsed_lines_and_builds_optistruct_file_summaries() {
    let root = unique_temp_dir("optistruct_text_and_summary");
    fs::create_dir_all(&root).expect("create temp root");
    let model_file = root.join("model.fem");
    fs::write(
        &model_file,
        "BEGIN BULK\nECHO = NONE\nGRID    1       0       0.0     0.0     0.0\n$ comentario\nENDDATA\n",
    )
    .expect("write model");

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
    let file_id = file_id_for_path(&content, &model_file);

    assert!(content.optistruct_items.iter().any(|item| matches!(
        item,
        OptiStructItem::Text(text)
            if text.location.file_id == file_id && text.text == "ECHO = NONE"
    )));

    let summary = content
        .optistruct_file_summaries
        .iter()
        .find(|entry| entry.file_id == file_id)
        .expect("summary should exist");
    assert_eq!(summary.total_items, 5);
    assert_eq!(summary.card_items, 3);
    assert_eq!(summary.comment_items, 1);
    assert_eq!(summary.text_items, 1);
    assert!(
        summary
            .card_counts
            .iter()
            .any(|entry| entry.card_name == "GRID" && entry.count == 1)
    );

    fs::remove_dir_all(&root).expect("cleanup temp root");
}
