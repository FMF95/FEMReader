use crate::model::SolverKind;
use crate::optistruct;
use std::collections::HashSet;
use std::path::Path;

pub trait SolverAdapter {
    fn name(&self) -> &'static str;
    fn solver_kind(&self) -> SolverKind;
    fn supports_file(&self, path: &Path) -> bool;
    fn extract_references(&self, content: &str) -> Vec<String>;
}

#[derive(Debug, Default)]
pub struct NastranAdapter;
#[derive(Debug, Default)]
pub struct OptiStructAdapter;
#[derive(Debug, Default)]
pub struct AbaqusAdapter;

fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|v| v.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn is_model_file_path(path: &Path) -> bool {
    if path.extension().is_none() {
        return true;
    }

    matches!(
        extension_of(path).as_str(),
        "bdf" | "fem" | "inp" | "dat" | "inc" | "nas" | "aba"
    )
}

fn parse_include_like_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let upper = line.to_ascii_uppercase();
        if (upper.starts_with("INCLUDE")
            || upper.starts_with("*INCLUDE")
            || upper.contains("INCLUDE="))
            && let Some(reference) = extract_reference(line)
        {
            references.push(reference);
        }
    }

    references
}

fn dedupe_preserving_order(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }

    deduped
}

fn extract_reference(line: &str) -> Option<String> {
    if let Some(start) = line.find('"') {
        let rest = &line[start + 1..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].trim().to_string());
        }
    }

    if let Some(start) = line.find('=') {
        let candidate = line[start + 1..]
            .trim()
            .trim_matches('\'')
            .trim_matches('"');
        if !candidate.is_empty() {
            return Some(candidate.to_string());
        }
    }

    let mut parts = line.split_whitespace();
    let _ = parts.next();
    if let Some(candidate) = parts.next() {
        let cleaned = candidate.trim_matches('\'').trim_matches('"').trim();
        if !cleaned.is_empty() {
            return Some(cleaned.to_string());
        }
    }

    None
}

impl SolverAdapter for NastranAdapter {
    fn name(&self) -> &'static str {
        "NASTRAN"
    }

    fn solver_kind(&self) -> SolverKind {
        SolverKind::Nastran
    }

    fn supports_file(&self, path: &Path) -> bool {
        matches!(extension_of(path).as_str(), "bdf" | "dat" | "nas")
    }

    fn extract_references(&self, content: &str) -> Vec<String> {
        parse_include_like_references(content)
    }
}

impl SolverAdapter for OptiStructAdapter {
    fn name(&self) -> &'static str {
        "OptiStruct"
    }

    fn solver_kind(&self) -> SolverKind {
        SolverKind::OptiStruct
    }

    fn supports_file(&self, path: &Path) -> bool {
        path.extension().is_none() || matches!(extension_of(path).as_str(), "fem" | "dat" | "inc")
    }

    fn extract_references(&self, content: &str) -> Vec<String> {
        let mut references = optistruct::extract_include_references(content);
        references.extend(parse_include_like_references(content));
        dedupe_preserving_order(references)
    }
}

impl SolverAdapter for AbaqusAdapter {
    fn name(&self) -> &'static str {
        "ABAQUS"
    }

    fn solver_kind(&self) -> SolverKind {
        SolverKind::Abaqus
    }

    fn supports_file(&self, path: &Path) -> bool {
        matches!(extension_of(path).as_str(), "inp" | "inc" | "aba")
    }

    fn extract_references(&self, content: &str) -> Vec<String> {
        parse_include_like_references(content)
    }
}
