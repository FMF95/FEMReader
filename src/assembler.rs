use crate::dependency::DependencyGraph;
use crate::ids::stable_id;
use crate::model::{FileCategory, Model, ModelContent, SolverKind};
use crate::scanner::ScanResult;
use std::collections::{HashMap, HashSet};

pub trait ModelAssembler {
    fn assemble(&self, scan_result: &ScanResult, graph: &DependencyGraph) -> ModelContent;
}

#[derive(Debug, Default)]
pub struct DefaultModelAssembler;

impl ModelAssembler for DefaultModelAssembler {
    fn assemble(&self, scan_result: &ScanResult, graph: &DependencyGraph) -> ModelContent {
        let mut incoming = HashMap::<String, usize>::new();
        let mut adjacency = HashMap::<String, Vec<String>>::new();
        let mut id_to_solver = HashMap::<String, SolverKind>::new();
        let mut id_to_name = HashMap::<String, String>::new();
        let model_files: Vec<_> = graph
            .files
            .iter()
            .filter(|file| file.category == FileCategory::Model)
            .collect();

        for file in &model_files {
            incoming.entry(file.id.clone()).or_insert(0);
            adjacency.entry(file.id.clone()).or_default();
            id_to_solver.insert(file.id.clone(), file.solver_hint.clone());
            id_to_name.insert(file.id.clone(), file.name.clone());
        }

        for edge in &graph.edges {
            if incoming.contains_key(&edge.from_file_id) && incoming.contains_key(&edge.to_file_id)
            {
                *incoming.entry(edge.to_file_id.clone()).or_insert(0) += 1;
                adjacency
                    .entry(edge.from_file_id.clone())
                    .or_default()
                    .push(edge.to_file_id.clone());
            }
        }

        let mut roots: Vec<String> = incoming
            .iter()
            .filter_map(|(file_id, count)| {
                if *count == 0 {
                    Some(file_id.clone())
                } else {
                    None
                }
            })
            .collect();

        if roots.is_empty() {
            roots = model_files.iter().map(|file| file.id.clone()).collect();
        }

        roots.sort();
        roots.dedup();

        let mut models = Vec::new();

        for root_file_id in roots {
            let mut visited = HashSet::new();
            let mut stack = vec![root_file_id.clone()];

            while let Some(current) = stack.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }

                if let Some(neighbors) = adjacency.get(&current) {
                    for next in neighbors {
                        stack.push(next.clone());
                    }
                }
            }

            let mut file_ids: Vec<String> = visited.into_iter().collect();
            file_ids.sort();

            let solver = select_solver(&file_ids, &id_to_solver);
            let model_index = models.len() + 1;
            let root_name = id_to_name
                .get(&root_file_id)
                .cloned()
                .unwrap_or_else(|| format!("model_{model_index}"));
            let model_source = format!("{}|{}", root_file_id, file_ids.join("|"));

            models.push(Model {
                id: stable_id("model", &model_source),
                name: root_name,
                root_file_id,
                file_ids,
                solver,
            });
        }

        ModelContent {
            schema_version: "1.0.0".to_string(),
            root_directory: scan_result.root_directory.clone(),
            directories: scan_result.directories.clone(),
            files: graph.files.clone(),
            dependencies: graph.edges.clone(),
            unresolved_references: graph.unresolved_references.clone(),
            optistruct_items: graph.optistruct_items.clone(),
            optistruct_file_summaries: graph.optistruct_file_summaries.clone(),
            models,
        }
    }
}

fn select_solver(file_ids: &[String], id_to_solver: &HashMap<String, SolverKind>) -> SolverKind {
    let mut detected: Option<SolverKind> = None;

    for file_id in file_ids {
        if let Some(kind) = id_to_solver.get(file_id) {
            match kind {
                SolverKind::Unknown => {}
                _ => match &detected {
                    None => detected = Some(kind.clone()),
                    Some(previous) if previous == kind => {}
                    Some(_) => return SolverKind::Unknown,
                },
            }
        }
    }

    detected.unwrap_or(SolverKind::Unknown)
}
