use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverKind {
    Unknown,
    Nastran,
    OptiStruct,
    Abaqus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileCategory {
    Model,
    NonModel,
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
    pub category: FileCategory,
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

#[derive(Debug, Clone, PartialEq)]
pub struct OptiStructLocation {
    pub file_id: String,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub ordinal: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptiStructCommentItem {
    pub location: OptiStructLocation,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridCard {
    pub id: Option<i64>,
    pub cp: Option<i64>,
    pub x1: Option<f64>,
    pub x2: Option<f64>,
    pub x3: Option<f64>,
    pub cd: Option<i64>,
    pub ps: Option<String>,
    pub seid: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cquad4Card {
    pub id: Option<i64>,
    pub pid: Option<i64>,
    pub g1: Option<i64>,
    pub g2: Option<i64>,
    pub g3: Option<i64>,
    pub g4: Option<i64>,
    pub theta_or_mcid: Option<String>,
    pub zoffs: Option<f64>,
    pub tflag: Option<i64>,
    pub t1: Option<f64>,
    pub t2: Option<f64>,
    pub t3: Option<f64>,
    pub t4: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ctria3Card {
    pub id: Option<i64>,
    pub pid: Option<i64>,
    pub g1: Option<i64>,
    pub g2: Option<i64>,
    pub g3: Option<i64>,
    pub theta_or_mcid: Option<String>,
    pub zoffs: Option<f64>,
    pub tflag: Option<i64>,
    pub t1: Option<f64>,
    pub t2: Option<f64>,
    pub t3: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForceCard {
    pub sid: Option<i64>,
    pub g: Option<i64>,
    pub cid: Option<i64>,
    pub f: Option<f64>,
    pub n1: Option<f64>,
    pub n2: Option<f64>,
    pub n3: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MomentCard {
    pub sid: Option<i64>,
    pub g: Option<i64>,
    pub cid: Option<i64>,
    pub m: Option<f64>,
    pub n1: Option<f64>,
    pub n2: Option<f64>,
    pub n3: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeCard {
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCard {
    pub card_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptiStructCardKind {
    Grid(GridCard),
    Cquad4(Cquad4Card),
    Ctria3(Ctria3Card),
    Force(ForceCard),
    Moment(MomentCard),
    Include(IncludeCard),
    Unknown(UnknownCard),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptiStructCardItem {
    pub location: OptiStructLocation,
    pub card_name: String,
    pub long_format: bool,
    pub fields: Vec<String>,
    pub kind: OptiStructCardKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptiStructItem {
    Comment(OptiStructCommentItem),
    Card(OptiStructCardItem),
}

#[derive(Debug, Clone)]
pub struct ModelContent {
    pub schema_version: String,
    pub root_directory: PathBuf,
    pub directories: Vec<DirectoryNode>,
    pub files: Vec<FileNode>,
    pub dependencies: Vec<DependencyEdge>,
    pub unresolved_references: Vec<String>,
    pub optistruct_items: Vec<OptiStructItem>,
    pub models: Vec<Model>,
}
