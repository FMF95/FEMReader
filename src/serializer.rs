use crate::model::{DependencyEdge, FileNode, Model, ModelContent, SolverKind};

pub trait Serializer {
    fn serialize(&self, content: &ModelContent, pretty: bool) -> String;
}

#[derive(Debug, Default)]
pub struct JsonSerializer;

impl Serializer for JsonSerializer {
    fn serialize(&self, content: &ModelContent, pretty: bool) -> String {
        if pretty {
            serialize_pretty(content)
        } else {
            serialize_compact(content)
        }
    }
}

fn serialize_compact(content: &ModelContent) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str(&format!(
        "\"schema_version\":\"{}\",",
        escape(&content.schema_version)
    ));
    out.push_str(&format!(
        "\"root_directory\":\"{}\",",
        escape(&content.root_directory.display().to_string())
    ));
    out.push_str("\"directories\":[");
    for (i, dir) in content.directories.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"id\":\"{}\",\"path\":\"{}\"}}",
            escape(&dir.id),
            escape(&dir.path.display().to_string())
        ));
    }
    out.push_str("],\"files\":[");
    for (i, file) in content.files.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_file_compact(file));
    }
    out.push_str("],\"dependencies\":[");
    for (i, dep) in content.dependencies.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_dep_compact(dep));
    }
    out.push_str("],\"unresolved_references\":[");
    for (i, unresolved) in content.unresolved_references.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\"{}\"", escape(unresolved)));
    }
    out.push_str("],\"models\":[");
    for (i, model) in content.models.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_model_compact(model));
    }
    out.push_str("]}");
    out
}

fn serialize_file_compact(file: &FileNode) -> String {
    format!(
        "{{\"id\":\"{}\",\"path\":\"{}\",\"extension\":{},\"size\":{},\"modified_unix_secs\":{},\"changed_since_last_scan\":{},\"solver_hint\":\"{}\"}}",
        escape(&file.id),
        escape(&file.path.display().to_string()),
        file.extension
            .as_ref()
            .map(|v| format!("\"{}\"", escape(v)))
            .unwrap_or_else(|| "null".to_string()),
        file.size,
        file.modified_unix_secs,
        if file.changed_since_last_scan {
            "true"
        } else {
            "false"
        },
        solver_label(&file.solver_hint)
    )
}

fn serialize_dep_compact(dep: &DependencyEdge) -> String {
    format!(
        "{{\"from_file_id\":\"{}\",\"to_file_id\":\"{}\",\"reference\":\"{}\"}}",
        escape(&dep.from_file_id),
        escape(&dep.to_file_id),
        escape(&dep.reference)
    )
}

fn serialize_model_compact(model: &Model) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{{\"id\":\"{}\",\"name\":\"{}\",\"root_file_id\":\"{}\",\"solver\":\"{}\",\"file_ids\":[",
        escape(&model.id),
        escape(&model.name),
        escape(&model.root_file_id),
        solver_label(&model.solver)
    ));
    for (i, file_id) in model.file_ids.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\"{}\"", escape(file_id)));
    }
    out.push_str("]}");
    out
}

fn serialize_pretty(content: &ModelContent) -> String {
    let compact = serialize_compact(content);
    pretty_json_like(&compact)
}

fn pretty_json_like(input: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                out.push(ch);
                out.push('\n');
                indent += 2;
                out.push_str(&" ".repeat(indent));
            }
            '}' | ']' => {
                out.push('\n');
                indent = indent.saturating_sub(2);
                out.push_str(&" ".repeat(indent));
                out.push(ch);
            }
            ',' => {
                out.push(ch);
                out.push('\n');
                out.push_str(&" ".repeat(indent));
            }
            ':' => {
                out.push(':');
                out.push(' ');
            }
            _ => out.push(ch),
        }
    }

    out
}

fn solver_label(kind: &SolverKind) -> &'static str {
    match kind {
        SolverKind::Unknown => "Unknown",
        SolverKind::Nastran => "NASTRAN",
        SolverKind::OptiStruct => "OptiStruct",
        SolverKind::Abaqus => "ABAQUS",
    }
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
