use crate::ids::stable_id;
use crate::model::{DependencyEdge, FileCategory, FileNode, OptiStructItem, SolverKind};
use crate::optistruct;
use crate::scanner::ScanResult;
use crate::solver::SolverAdapter;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub files: Vec<FileNode>,
    pub edges: Vec<DependencyEdge>,
    pub unresolved_references: Vec<String>,
    pub optistruct_items: Vec<OptiStructItem>,
}

pub trait DependencyResolver {
    fn resolve(&self, scan_result: &ScanResult) -> DependencyGraph;
}

pub struct DefaultDependencyResolver {
    adapters: Vec<Box<dyn SolverAdapter + Send + Sync>>,
}

impl DefaultDependencyResolver {
    pub fn new(adapters: Vec<Box<dyn SolverAdapter + Send + Sync>>) -> Self {
        Self { adapters }
    }

    fn adapter_for_file(&self, file: &FileNode) -> Option<&(dyn SolverAdapter + Send + Sync)> {
        let mut matching: Vec<&(dyn SolverAdapter + Send + Sync)> = self
            .adapters
            .iter()
            .filter(|adapter| adapter.supports_file(&file.path))
            .map(|adapter| adapter.as_ref())
            .collect();

        if matching.is_empty() {
            return None;
        }

        if file.solver_hint != SolverKind::Unknown
            && let Some(preferred) = matching
                .iter()
                .copied()
                .find(|adapter| adapter.solver_kind() == file.solver_hint)
        {
            return Some(preferred);
        }

        Some(matching.remove(0))
    }
}

impl DependencyResolver for DefaultDependencyResolver {
    fn resolve(&self, scan_result: &ScanResult) -> DependencyGraph {
        let mut files = scan_result.files.clone();
        let mut edges = Vec::new();
        let mut unresolved_references = Vec::new();
        let mut optistruct_items = Vec::new();
        let directory_solver_hints = build_directory_solver_hints(scan_result);

        let mut by_path: HashMap<PathBuf, String> = HashMap::new();
        for file in &files {
            if file.category == FileCategory::Model {
                by_path.insert(file.path.clone(), file.id.clone());
            }
        }

        for file in &mut files {
            if file.category == FileCategory::Model {
                file.solver_hint = file
                    .path
                    .parent()
                    .and_then(|parent| directory_solver_hints.get(parent).cloned())
                    .unwrap_or(crate::model::SolverKind::Unknown);
            } else {
                file.solver_hint = SolverKind::Unknown;
            }

            if file.category != FileCategory::Model {
                continue;
            }

            let Some(adapter) = self.adapter_for_file(file) else {
                continue;
            };

            let content = match fs::read_to_string(&file.path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            if adapter.solver_kind() == SolverKind::OptiStruct {
                optistruct_items
                    .extend(optistruct::parse_file_items(&file.id, &file.path, &content));
            }

            for reference in adapter.extract_references(&content) {
                let target = resolve_reference_path(&file.path, &reference);
                if let Some(target_id) = by_path.get(&target) {
                    let edge_source = format!(
                        "{}|{}|{}|{}",
                        file.id,
                        target_id,
                        reference,
                        edges.len() + 1
                    );
                    edges.push(DependencyEdge {
                        id: stable_id("dep", &edge_source),
                        from_file_id: file.id.clone(),
                        to_file_id: target_id.clone(),
                        reference,
                    });
                } else {
                    unresolved_references.push(format!("{} -> {}", file.path.display(), reference));
                }
            }
        }

        DependencyGraph {
            files,
            edges,
            unresolved_references,
            optistruct_items,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct DirectorySolverPresence {
    has_nastran: bool,
    has_optistruct: bool,
    has_abaqus: bool,
}

impl DirectorySolverPresence {
    fn register_extension(&mut self, extension: Option<&str>) {
        match extension {
            Some("bdf") => self.has_nastran = true,
            Some("fem") => self.has_optistruct = true,
            Some("inp") => self.has_abaqus = true,
            _ => {}
        }
    }

    fn to_solver_kind(self) -> crate::model::SolverKind {
        let mut total = 0;
        if self.has_nastran {
            total += 1;
        }
        if self.has_optistruct {
            total += 1;
        }
        if self.has_abaqus {
            total += 1;
        }

        if total != 1 {
            return crate::model::SolverKind::Unknown;
        }

        if self.has_nastran {
            crate::model::SolverKind::Nastran
        } else if self.has_optistruct {
            crate::model::SolverKind::OptiStruct
        } else {
            crate::model::SolverKind::Abaqus
        }
    }
}

fn build_directory_solver_hints(
    scan_result: &ScanResult,
) -> HashMap<PathBuf, crate::model::SolverKind> {
    let mut by_directory = HashMap::<PathBuf, DirectorySolverPresence>::new();

    for file in &scan_result.files {
        if file.category != FileCategory::Model {
            continue;
        }
        let parent = file.path.parent().unwrap_or(&scan_result.root_directory);
        let entry = by_directory.entry(parent.to_path_buf()).or_default();
        entry.register_extension(file.extension.as_deref());
    }

    by_directory
        .into_iter()
        .map(|(directory, presence)| (directory, presence.to_solver_kind()))
        .collect()
}

fn resolve_reference_path(base_file: &Path, reference: &str) -> PathBuf {
    let reference_path = PathBuf::from(reference);

    let candidate = if reference_path.is_absolute() {
        reference_path
    } else {
        base_file
            .parent()
            .map(|parent| parent.join(reference_path))
            .unwrap_or_else(|| PathBuf::from(reference))
    };

    fs::canonicalize(&candidate).unwrap_or(candidate)
}
