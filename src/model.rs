use std::path::PathBuf;

// JSON contract version. Bump this when output schema changes (keys, structure, semantics).
pub const SCHEMA_VERSION: &str = "1.1.0";

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
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: String,
    pub name: String,
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
    pub id: String,
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
    pub file_name: String,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub ordinal: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptiStructCommentItem {
    pub id: String,
    pub location: OptiStructLocation,
    pub kind: OptiStructCommentKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptiStructTextItem {
    pub id: String,
    pub location: OptiStructLocation,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HMNameCommentFields {
    pub entity_type: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub id_pool: Option<String>,
    pub raw_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HWColorCommentFields {
    pub entity_type: Option<String>,
    pub id: Option<String>,
    pub color_id: Option<String>,
    pub raw_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HMMoveCommentFields {
    pub collector_id: Option<String>,
    pub raw_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HMDPrpCommentFields {
    pub elements: Option<String>,
    pub ids: Vec<String>,
    pub raw_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperMeshCommentFields {
    HMName(HMNameCommentFields),
    HWColor(HWColorCommentFields),
    HMMove(HMMoveCommentFields),
    HMDPrp(HMDPrpCommentFields),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptiStructCommentKind {
    Comment,
    HyperMeshComment {
        tag: String,
        fields: HyperMeshCommentFields,
    },
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

#[derive(Debug, Clone, PartialEq)]
pub struct CbushCard {
    pub eid: Option<i64>,
    pub pid: Option<i64>,
    pub ga: Option<i64>,
    pub gb: Option<i64>,
    pub cid: Option<i64>,
    pub s: Option<f64>,
    pub ocid: Option<i64>,
    pub s1: Option<f64>,
    pub s2: Option<f64>,
    pub s3: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PbushCard {
    pub pid: Option<i64>,
    pub k1: Option<f64>,
    pub k2: Option<f64>,
    pub k3: Option<f64>,
    pub k4: Option<f64>,
    pub k5: Option<f64>,
    pub k6: Option<f64>,
    pub b1: Option<f64>,
    pub b2: Option<f64>,
    pub b3: Option<f64>,
    pub b4: Option<f64>,
    pub b5: Option<f64>,
    pub b6: Option<f64>,
    pub ge1: Option<f64>,
    pub ge2: Option<f64>,
    pub ge3: Option<f64>,
    pub ge4: Option<f64>,
    pub ge5: Option<f64>,
    pub ge6: Option<f64>,
    pub sa: Option<f64>,
    pub sb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PshellCard {
    pub pid: Option<i64>,
    pub mid1: Option<i64>,
    pub t: Option<f64>,
    pub mid2: Option<i64>,
    pub twelve_i_over_t3: Option<f64>,
    pub mid3: Option<i64>,
    pub ts_over_t: Option<f64>,
    pub nsm: Option<f64>,
    pub z1: Option<f64>,
    pub z2: Option<f64>,
    pub mid4: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mat1Card {
    pub mid: Option<i64>,
    pub e: Option<f64>,
    pub g: Option<f64>,
    pub nu: Option<f64>,
    pub rho: Option<f64>,
    pub a: Option<f64>,
    pub tref: Option<f64>,
    pub ge: Option<f64>,
    pub st: Option<f64>,
    pub sc: Option<f64>,
    pub ss: Option<f64>,
    pub mcsid: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeCard {
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeginBulkCard;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndDataCard;

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
    Cbush(CbushCard),
    Pbush(PbushCard),
    Pshell(PshellCard),
    Mat1(Mat1Card),
    Include(IncludeCard),
    BeginBulk(BeginBulkCard),
    EndData(EndDataCard),
    Unknown(UnknownCard),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptiStructCardItem {
    pub id: String,
    pub location: OptiStructLocation,
    pub text: String,
    pub card_name: String,
    pub long_format: bool,
    pub fields: Vec<String>,
    pub kind: OptiStructCardKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptiStructItem {
    Comment(OptiStructCommentItem),
    Text(OptiStructTextItem),
    Card(OptiStructCardItem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptiStructCardCount {
    pub card_name: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptiStructFileSummary {
    pub file_id: String,
    pub file_name: String,
    pub file_path: PathBuf,
    pub total_non_empty_lines: usize,
    pub covered_non_empty_lines: usize,
    pub line_coverage_ok: bool,
    pub total_items: usize,
    pub comment_items: usize,
    pub hypermesh_comment_items: usize,
    pub text_items: usize,
    pub card_items: usize,
    pub card_counts: Vec<OptiStructCardCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptiStructModelSummary {
    pub model_id: String,
    pub model_name: String,
    pub total_non_empty_lines: usize,
    pub covered_non_empty_lines: usize,
    pub line_coverage_ok: bool,
    pub total_items: usize,
    pub comment_items: usize,
    pub hypermesh_comment_items: usize,
    pub text_items: usize,
    pub card_items: usize,
    pub card_counts: Vec<OptiStructCardCount>,
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
    pub optistruct_file_summaries: Vec<OptiStructFileSummary>,
    pub optistruct_model_summaries: Vec<OptiStructModelSummary>,
    pub models: Vec<Model>,
}
