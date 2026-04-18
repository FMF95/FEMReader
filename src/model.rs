use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverKind {
    Unknown,
    Nastran,
    OptiStruct,
    Abaqus,
}

#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: String,
    pub path: PathBuf,
    pub extension: Option<String>,
    pub size: u64,
    pub modified_unix_secs: u64,
    pub changed_since_last_scan: bool,
    pub solver_hint: SolverKind,
}

#[derive(Debug, Clone)]
pub struct DependencyEdge {
    pub from_file_id: String,
    pub to_file_id: String,
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub root_file_id: String,
    pub file_ids: Vec<String>,
    pub solver: SolverKind,
}

#[derive(Debug, Clone)]
pub struct ModelContent {
    pub schema_version: String,
    pub root_directory: PathBuf,
    pub directories: Vec<DirectoryNode>,
    pub files: Vec<FileNode>,
    pub dependencies: Vec<DependencyEdge>,
    pub unresolved_references: Vec<String>,
    pub models: Vec<Model>,
}
