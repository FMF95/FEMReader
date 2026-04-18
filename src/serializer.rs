use crate::model::{
    BeginBulkCard, Cquad4Card, Ctria3Card, DependencyEdge, EndDataCard, FileCategory, FileNode,
    ForceCard, GridCard, HMDPrpCommentFields, HMMoveCommentFields, HMNameCommentFields,
    HWColorCommentFields, HyperMeshCommentFields, IncludeCard, Model, ModelContent, MomentCard,
    OptiStructCardItem, OptiStructCardKind, OptiStructCommentItem, OptiStructCommentKind,
    OptiStructItem, OptiStructLocation, SolverKind, UnknownCard,
};

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
            "{{\"id\":\"{}\",\"name\":\"{}\",\"path\":\"{}\"}}",
            escape(&dir.id),
            escape(&dir.name),
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
    out.push_str("],\"optistruct_items\":[");
    for (i, item) in content.optistruct_items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_optistruct_item_compact(item));
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

fn serialize_optistruct_item_compact(item: &OptiStructItem) -> String {
    match item {
        OptiStructItem::Comment(comment) => serialize_optistruct_comment_compact(comment),
        OptiStructItem::Card(card) => serialize_optistruct_card_compact(card),
    }
}

fn serialize_optistruct_comment_compact(comment: &OptiStructCommentItem) -> String {
    let (item_type, comment_kind, hypermesh_tag, hypermesh_fields) = match &comment.kind {
        OptiStructCommentKind::Comment => ("Comment", "Comment", None, None),
        OptiStructCommentKind::HyperMeshComment { tag, fields } => (
            "HyperMesh Comment",
            "HyperMeshComment",
            Some(tag.as_str()),
            Some(serialize_hypermesh_comment_fields(fields)),
        ),
    };

    format!(
        "{{\"id\":\"{}\",\"type\":\"{}\",\"comment_kind\":\"{}\",\"hypermesh_tag\":{},\"hypermesh_fields\":{},\"location\":{},\"text\":\"{}\"}}",
        escape(&comment.id),
        item_type,
        comment_kind,
        serialize_opt_string(hypermesh_tag),
        hypermesh_fields.unwrap_or_else(|| "null".to_string()),
        serialize_optistruct_location_compact(&comment.location),
        escape(&comment.text)
    )
}

fn serialize_optistruct_card_compact(card: &OptiStructCardItem) -> String {
    let mut fields = String::new();
    fields.push('[');
    for (index, field) in card.fields.iter().enumerate() {
        if index > 0 {
            fields.push(',');
        }
        fields.push_str(&format!("\"{}\"", escape(field)));
    }
    fields.push(']');

    format!(
        "{{\"id\":\"{}\",\"type\":\"Card\",\"location\":{},\"card_name\":\"{}\",\"long_format\":{},\"fields\":{},\"kind\":{},\"attributes\":{}}}",
        escape(&card.id),
        serialize_optistruct_location_compact(&card.location),
        escape(&card.card_name),
        if card.long_format { "true" } else { "false" },
        fields,
        serialize_optistruct_kind_label(&card.kind),
        serialize_optistruct_card_attributes(&card.kind)
    )
}

fn serialize_hypermesh_comment_fields(fields: &HyperMeshCommentFields) -> String {
    match fields {
        HyperMeshCommentFields::HMName(HMNameCommentFields {
            entity_type,
            id,
            name,
            id_pool,
            raw_fields,
        }) => format!(
            "{{\"kind\":\"HMNAME\",\"entity_type\":{},\"id\":{},\"name\":{},\"id_pool\":{},\"raw_fields\":{}}}",
            serialize_opt_string(entity_type.as_deref()),
            serialize_opt_string(id.as_deref()),
            serialize_opt_string(name.as_deref()),
            serialize_opt_string(id_pool.as_deref()),
            serialize_string_array(raw_fields)
        ),
        HyperMeshCommentFields::HWColor(HWColorCommentFields {
            entity_type,
            id,
            color_id,
            raw_fields,
        }) => format!(
            "{{\"kind\":\"HWCOLOR\",\"entity_type\":{},\"id\":{},\"color_id\":{},\"raw_fields\":{}}}",
            serialize_opt_string(entity_type.as_deref()),
            serialize_opt_string(id.as_deref()),
            serialize_opt_string(color_id.as_deref()),
            serialize_string_array(raw_fields)
        ),
        HyperMeshCommentFields::HMMove(HMMoveCommentFields {
            collector_id,
            raw_fields,
        }) => format!(
            "{{\"kind\":\"HMMOVE\",\"collector_id\":{},\"raw_fields\":{}}}",
            serialize_opt_string(collector_id.as_deref()),
            serialize_string_array(raw_fields)
        ),
        HyperMeshCommentFields::HMDPrp(HMDPrpCommentFields {
            elements,
            ids,
            raw_fields,
        }) => format!(
            "{{\"kind\":\"HMDPRP\",\"elements\":{},\"ids\":{},\"raw_fields\":{}}}",
            serialize_opt_string(elements.as_deref()),
            serialize_string_array(ids),
            serialize_string_array(raw_fields)
        ),
    }
}

fn serialize_optistruct_location_compact(location: &OptiStructLocation) -> String {
    format!(
        "{{\"file_id\":\"{}\",\"file_name\":\"{}\",\"file_path\":\"{}\",\"start_line\":{},\"end_line\":{},\"ordinal\":{}}}",
        escape(&location.file_id),
        escape(&location.file_name),
        escape(&location.file_path.display().to_string()),
        location.start_line,
        location.end_line,
        location.ordinal
    )
}

fn serialize_optistruct_kind_label(kind: &OptiStructCardKind) -> String {
    let label = match kind {
        OptiStructCardKind::Grid(_) => "GRID",
        OptiStructCardKind::Cquad4(_) => "CQUAD4",
        OptiStructCardKind::Ctria3(_) => "CTRIA3",
        OptiStructCardKind::Force(_) => "FORCE",
        OptiStructCardKind::Moment(_) => "MOMENT",
        OptiStructCardKind::Include(_) => "INCLUDE",
        OptiStructCardKind::BeginBulk(_) => "BEGIN BULK",
        OptiStructCardKind::EndData(_) => "ENDDATA",
        OptiStructCardKind::Unknown(_) => "UNKNOWN",
    };
    format!("\"{label}\"")
}

fn serialize_optistruct_card_attributes(kind: &OptiStructCardKind) -> String {
    match kind {
        OptiStructCardKind::Grid(card) => serialize_grid_card(card),
        OptiStructCardKind::Cquad4(card) => serialize_cquad4_card(card),
        OptiStructCardKind::Ctria3(card) => serialize_ctria3_card(card),
        OptiStructCardKind::Force(card) => serialize_force_card(card),
        OptiStructCardKind::Moment(card) => serialize_moment_card(card),
        OptiStructCardKind::Include(card) => serialize_include_card(card),
        OptiStructCardKind::BeginBulk(card) => serialize_begin_bulk_card(card),
        OptiStructCardKind::EndData(card) => serialize_enddata_card(card),
        OptiStructCardKind::Unknown(card) => serialize_unknown_card(card),
    }
}

fn serialize_grid_card(card: &GridCard) -> String {
    format!(
        "{{\"id\":{},\"cp\":{},\"x1\":{},\"x2\":{},\"x3\":{},\"cd\":{},\"ps\":{},\"seid\":{}}}",
        serialize_opt_i64(card.id),
        serialize_opt_i64(card.cp),
        serialize_opt_f64(card.x1),
        serialize_opt_f64(card.x2),
        serialize_opt_f64(card.x3),
        serialize_opt_i64(card.cd),
        serialize_opt_string(card.ps.as_deref()),
        serialize_opt_i64(card.seid)
    )
}

fn serialize_cquad4_card(card: &Cquad4Card) -> String {
    format!(
        "{{\"id\":{},\"pid\":{},\"g1\":{},\"g2\":{},\"g3\":{},\"g4\":{},\"theta_or_mcid\":{},\"zoffs\":{},\"tflag\":{},\"t1\":{},\"t2\":{},\"t3\":{},\"t4\":{}}}",
        serialize_opt_i64(card.id),
        serialize_opt_i64(card.pid),
        serialize_opt_i64(card.g1),
        serialize_opt_i64(card.g2),
        serialize_opt_i64(card.g3),
        serialize_opt_i64(card.g4),
        serialize_opt_string(card.theta_or_mcid.as_deref()),
        serialize_opt_f64(card.zoffs),
        serialize_opt_i64(card.tflag),
        serialize_opt_f64(card.t1),
        serialize_opt_f64(card.t2),
        serialize_opt_f64(card.t3),
        serialize_opt_f64(card.t4)
    )
}

fn serialize_ctria3_card(card: &Ctria3Card) -> String {
    format!(
        "{{\"id\":{},\"pid\":{},\"g1\":{},\"g2\":{},\"g3\":{},\"theta_or_mcid\":{},\"zoffs\":{},\"tflag\":{},\"t1\":{},\"t2\":{},\"t3\":{}}}",
        serialize_opt_i64(card.id),
        serialize_opt_i64(card.pid),
        serialize_opt_i64(card.g1),
        serialize_opt_i64(card.g2),
        serialize_opt_i64(card.g3),
        serialize_opt_string(card.theta_or_mcid.as_deref()),
        serialize_opt_f64(card.zoffs),
        serialize_opt_i64(card.tflag),
        serialize_opt_f64(card.t1),
        serialize_opt_f64(card.t2),
        serialize_opt_f64(card.t3)
    )
}

fn serialize_force_card(card: &ForceCard) -> String {
    format!(
        "{{\"sid\":{},\"g\":{},\"cid\":{},\"f\":{},\"n1\":{},\"n2\":{},\"n3\":{}}}",
        serialize_opt_i64(card.sid),
        serialize_opt_i64(card.g),
        serialize_opt_i64(card.cid),
        serialize_opt_f64(card.f),
        serialize_opt_f64(card.n1),
        serialize_opt_f64(card.n2),
        serialize_opt_f64(card.n3)
    )
}

fn serialize_moment_card(card: &MomentCard) -> String {
    format!(
        "{{\"sid\":{},\"g\":{},\"cid\":{},\"m\":{},\"n1\":{},\"n2\":{},\"n3\":{}}}",
        serialize_opt_i64(card.sid),
        serialize_opt_i64(card.g),
        serialize_opt_i64(card.cid),
        serialize_opt_f64(card.m),
        serialize_opt_f64(card.n1),
        serialize_opt_f64(card.n2),
        serialize_opt_f64(card.n3)
    )
}

fn serialize_include_card(card: &IncludeCard) -> String {
    format!("{{\"target\":\"{}\"}}", escape(&card.target))
}

fn serialize_begin_bulk_card(_: &BeginBulkCard) -> String {
    "{}".to_string()
}

fn serialize_enddata_card(_: &EndDataCard) -> String {
    "{}".to_string()
}

fn serialize_unknown_card(card: &UnknownCard) -> String {
    format!("{{\"card_name\":\"{}\"}}", escape(&card.card_name))
}

fn serialize_opt_i64(value: Option<i64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn serialize_opt_f64(value: Option<f64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn serialize_opt_string(value: Option<&str>) -> String {
    value
        .map(|v| format!("\"{}\"", escape(v)))
        .unwrap_or_else(|| "null".to_string())
}

fn serialize_string_array(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&escape(value));
        out.push('"');
    }
    out.push(']');
    out
}

fn serialize_file_compact(file: &FileNode) -> String {
    format!(
        "{{\"id\":\"{}\",\"name\":\"{}\",\"path\":\"{}\",\"extension\":{},\"size\":{},\"modified_unix_secs\":{},\"changed_since_last_scan\":{},\"category\":\"{}\",\"solver_hint\":\"{}\"}}",
        escape(&file.id),
        escape(&file.name),
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
        file_category_label(&file.category),
        solver_label(&file.solver_hint)
    )
}

fn serialize_dep_compact(dep: &DependencyEdge) -> String {
    format!(
        "{{\"id\":\"{}\",\"from_file_id\":\"{}\",\"to_file_id\":\"{}\",\"reference\":\"{}\"}}",
        escape(&dep.id),
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

fn file_category_label(category: &FileCategory) -> &'static str {
    match category {
        FileCategory::Model => "Model",
        FileCategory::NonModel => "NonModel",
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
