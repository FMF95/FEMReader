use crate::model::{DependencyEdge, FileNode, SolverKind};
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

    fn adapter_for_path(&self, path: &Path) -> Option<&(dyn SolverAdapter + Send + Sync)> {
        self.adapters
            .iter()
            .find(|adapter| adapter.supports_file(path))
            .map(|adapter| adapter.as_ref())
    }
}

impl DependencyResolver for DefaultDependencyResolver {
    fn resolve(&self, scan_result: &ScanResult) -> DependencyGraph {
        let mut files = scan_result.files.clone();
        let mut edges = Vec::new();
        let mut unresolved_references = Vec::new();

        let mut by_path: HashMap<PathBuf, String> = HashMap::new();
        for file in &files {
            by_path.insert(file.path.clone(), file.id.clone());
        }

        for file in &mut files {
            let Some(adapter) = self.adapter_for_path(&file.path) else {
                continue;
            };

            file.solver_hint = adapter.solver_kind();

            let content = match fs::read_to_string(&file.path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for reference in adapter.extract_references(&content) {
                let target = resolve_reference_path(&file.path, &reference);
                if let Some(target_id) = by_path.get(&target) {
                    edges.push(DependencyEdge {
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
        }
    }
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

#[allow(dead_code)]
fn _solver_name(kind: &SolverKind) -> &'static str {
    match kind {
        SolverKind::Unknown => "Unknown",
        SolverKind::Nastran => "NASTRAN",
        SolverKind::OptiStruct => "OptiStruct",
        SolverKind::Abaqus => "ABAQUS",
    }
}
