use crate::model::{
    BeginBulkCard, CbushCard, Cquad4Card, Ctria3Card, DependencyEdge, EndDataCard, FileCategory,
    FileNode, ForceCard, GridCard, HMDPrpCommentFields, HMMoveCommentFields, HMNameCommentFields,
    HWColorCommentFields, HyperMeshCommentFields, IncludeCard, Mat1Card, Model, ModelContent,
    MomentCard, OptiStructCardCount, OptiStructCardItem, OptiStructCardKind, OptiStructCommentItem,
    OptiStructCommentKind, OptiStructFileSummary, OptiStructItem, OptiStructLocation,
    OptiStructModelSummary, OptiStructTextItem, PbushCard, PshellCard, Rbe2Card, Rbe3Card,
    SolverKind, UnknownCard,
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

// Contract marker used by tests to force an explicit schema-version review when JSON shape changes.
#[cfg(test)]
const SCHEMA_CONTRACT_MARKER: &str = "card-text-key:text|begin-bulk:no-fields|enddata:no-fields|model-summaries:yes|items-array:ordered";

#[cfg(test)]
fn schema_contract_marker() -> &'static str {
    SCHEMA_CONTRACT_MARKER
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
    out.push_str("\"json_guide\":");
    out.push_str(&serialize_json_guide_compact());
    out.push(',');
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
    out.push_str("],\"optistruct_file_summaries\":[");
    for (i, summary) in content.optistruct_file_summaries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_optistruct_file_summary_compact(summary));
    }
    out.push_str("],\"optistruct_model_summaries\":[");
    for (i, summary) in content.optistruct_model_summaries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serialize_optistruct_model_summary_compact(summary));
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
        OptiStructItem::Text(text) => serialize_optistruct_text_compact(text),
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
        "{{\"id\":\"{}\",\"type\":\"Card\",\"location\":{},\"text\":\"{}\",\"card_name\":\"{}\",\"long_format\":{},\"fields\":{},\"attributes\":{}}}",
        escape(&card.id),
        serialize_optistruct_location_compact(&card.location),
        escape(&card.text),
        escape(&card.card_name),
        if card.long_format { "true" } else { "false" },
        fields,
        serialize_optistruct_card_attributes(&card.kind)
    )
}

fn serialize_optistruct_text_compact(text: &OptiStructTextItem) -> String {
    format!(
        "{{\"id\":\"{}\",\"type\":\"Text\",\"location\":{},\"text\":\"{}\"}}",
        escape(&text.id),
        serialize_optistruct_location_compact(&text.location),
        escape(&text.text)
    )
}

fn serialize_optistruct_file_summary_compact(summary: &OptiStructFileSummary) -> String {
    format!(
        "{{\"file_id\":\"{}\",\"file_name\":\"{}\",\"file_path\":\"{}\",\"total_non_empty_lines\":{},\"covered_non_empty_lines\":{},\"line_coverage_ok\":{},\"total_items\":{},\"comment_items\":{},\"hypermesh_comment_items\":{},\"text_items\":{},\"card_items\":{},\"card_counts\":{}}}",
        escape(&summary.file_id),
        escape(&summary.file_name),
        escape(&summary.file_path.display().to_string()),
        summary.total_non_empty_lines,
        summary.covered_non_empty_lines,
        if summary.line_coverage_ok {
            "true"
        } else {
            "false"
        },
        summary.total_items,
        summary.comment_items,
        summary.hypermesh_comment_items,
        summary.text_items,
        summary.card_items,
        serialize_optistruct_card_counts(&summary.card_counts)
    )
}

fn serialize_optistruct_card_counts(values: &[OptiStructCardCount]) -> String {
    let mut out = String::from("[");
    for (index, entry) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"card_name\":\"{}\",\"count\":{}}}",
            escape(&entry.card_name),
            entry.count
        ));
    }
    out.push(']');
    out
}

fn serialize_optistruct_model_summary_compact(summary: &OptiStructModelSummary) -> String {
    format!(
        "{{\"model_id\":\"{}\",\"model_name\":\"{}\",\"total_non_empty_lines\":{},\"covered_non_empty_lines\":{},\"line_coverage_ok\":{},\"total_items\":{},\"comment_items\":{},\"hypermesh_comment_items\":{},\"text_items\":{},\"card_items\":{},\"card_counts\":{}}}",
        escape(&summary.model_id),
        escape(&summary.model_name),
        summary.total_non_empty_lines,
        summary.covered_non_empty_lines,
        if summary.line_coverage_ok {
            "true"
        } else {
            "false"
        },
        summary.total_items,
        summary.comment_items,
        summary.hypermesh_comment_items,
        summary.text_items,
        summary.card_items,
        serialize_optistruct_card_counts(&summary.card_counts)
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

fn serialize_optistruct_card_attributes(kind: &OptiStructCardKind) -> String {
    match kind {
        OptiStructCardKind::Grid(card) => serialize_grid_card(card),
        OptiStructCardKind::Cquad4(card) => serialize_cquad4_card(card),
        OptiStructCardKind::Ctria3(card) => serialize_ctria3_card(card),
        OptiStructCardKind::Force(card) => serialize_force_card(card),
        OptiStructCardKind::Moment(card) => serialize_moment_card(card),
        OptiStructCardKind::Cbush(card) => serialize_cbush_card(card),
        OptiStructCardKind::Pbush(card) => serialize_pbush_card(card),
        OptiStructCardKind::Pshell(card) => serialize_pshell_card(card),
        OptiStructCardKind::Mat1(card) => serialize_mat1_card(card),
        OptiStructCardKind::Rbe2(card) => serialize_rbe2_card(card),
        OptiStructCardKind::Rbe3(card) => serialize_rbe3_card(card),
        OptiStructCardKind::Include(card) => serialize_include_card(card),
        OptiStructCardKind::BeginBulk(card) => serialize_begin_bulk_card(card),
        OptiStructCardKind::EndData(card) => serialize_enddata_card(card),
        OptiStructCardKind::Unknown(card) => serialize_unknown_card(card),
    }
}

fn serialize_json_guide_compact() -> String {
    "{\
\"notes\":[\
\"json_guide es una ayuda de lectura para ubicar secciones del JSON\",\
\"optistruct_items se mantiene como lista para preservar orden de lectura; el id permite indexación externa\",\
\"optistruct_items contiene el contenido leído de cada archivo (Comment/Text/Card)\",\
\"optistruct_file_summaries resume conteos por archivo\",\
\"optistruct_model_summaries resume conteos agregados por modelo\"\
],\
\"card_field_templates\":{\
\"GRID\":[\"F1:card_name\",\"F2:id\",\"F3:cp\",\"F4:x1\",\"F5:x2\",\"F6:x3\",\"F7:cd\",\"F8:ps\",\"F9:seid\"],\
\"CQUAD4\":[\"F1:card_name\",\"F2:id\",\"F3:pid\",\"F4:g1\",\"F5:g2\",\"F6:g3\",\"F7:g4\",\"F8:theta_or_mcid\",\"F9:zoffs\",\"F10:tflag\"],\
\"CTRIA3\":[\"F1:card_name\",\"F2:id\",\"F3:pid\",\"F4:g1\",\"F5:g2\",\"F6:g3\",\"F7:theta_or_mcid\",\"F8:zoffs\",\"F9:tflag\",\"F10:t1\"],\
\"FORCE\":[\"F1:card_name\",\"F2:sid\",\"F3:g\",\"F4:cid\",\"F5:f\",\"F6:n1\",\"F7:n2\",\"F8:n3\"],\
\"MOMENT\":[\"F1:card_name\",\"F2:sid\",\"F3:g\",\"F4:cid\",\"F5:m\",\"F6:n1\",\"F7:n2\",\"F8:n3\"],\
\"CBUSH\":[\"F1:card_name\",\"F2:eid\",\"F3:pid\",\"F4:ga\",\"F5:gb\",\"F6:g0/x1\",\"F7:x2\",\"F8:x3\",\"F9:cid\",\"F10:s\",\"F11:ocid\",\"F12:s1\",\"F13:s2\",\"F14:s3\"],\
\"PBUSH\":[\"F1:card_name\",\"F2:pid\",\"F3:k1\",\"F4:k2\",\"F5:k3\",\"F6:k4\",\"F7:k5\",\"F8:k6\",\"F9:b1\",\"F10:b2\",\"F11:b3\",\"F12:b4\",\"F13:b5\",\"F14:b6\",\"F15:ge1\",\"F16:ge2\",\"F17:ge3\",\"F18:ge4\",\"F19:ge5\",\"F20:ge6\",\"F21:sa\",\"F22:sb\"],\
\"PSHELL\":[\"F1:card_name\",\"F2:pid\",\"F3:mid1\",\"F4:t\",\"F5:mid2\",\"F6:twelve_i_over_t3\",\"F7:mid3\",\"F8:ts_over_t\",\"F9:nsm\",\"F10:z1\",\"F11:z2\",\"F12:mid4\"],\
\"MAT1\":[\"F1:card_name\",\"F2:mid\",\"F3:e\",\"F4:g\",\"F5:nu\",\"F6:rho\",\"F7:a\",\"F8:tref\",\"F9:ge\",\"F10:st\",\"F11:sc\",\"F12:ss\",\"F13:mcsid\"],\
\"RBE2\":[\"F1:card_name\",\"F2:eid\",\"F3:gn\",\"F4:cm\",\"F5..F(n-1):gm\",\"F(n):alpha (opcional)\"],\
\"RBE3\":[\"F1:card_name\",\"F2:eid\",\"F3:blank\",\"F4:refgrid\",\"F5:refc\",\"F6..F(n-2):remaining_fields (grupos de peso/componentes/nodos y UM)\",\"F(n-1):ALPHA (marcador opcional)\",\"F(n):alpha (valor opcional)\"],\
\"INCLUDE\":[\"F1:card_name\",\"F2:target\"],\
\"BEGIN BULK\":[],\
\"ENDDATA\":[]\
}\
}"
    .to_string()
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

fn serialize_cbush_card(card: &CbushCard) -> String {
    format!(
        "{{\"eid\":{},\"pid\":{},\"ga\":{},\"gb\":{},\"g0\":{},\"x1\":{},\"x2\":{},\"x3\":{},\"cid\":{},\"s\":{},\"ocid\":{},\"s1\":{},\"s2\":{},\"s3\":{}}}",
        serialize_opt_i64(card.eid),
        serialize_opt_i64(card.pid),
        serialize_opt_i64(card.ga),
        serialize_opt_i64(card.gb),
        serialize_opt_i64(card.g0),
        serialize_opt_f64(card.x1),
        serialize_opt_f64(card.x2),
        serialize_opt_f64(card.x3),
        serialize_opt_i64(card.cid),
        serialize_opt_f64(card.s),
        serialize_opt_i64(card.ocid),
        serialize_opt_f64(card.s1),
        serialize_opt_f64(card.s2),
        serialize_opt_f64(card.s3)
    )
}

fn serialize_pbush_card(card: &PbushCard) -> String {
    format!(
        "{{\"pid\":{},\"k1\":{},\"k2\":{},\"k3\":{},\"k4\":{},\"k5\":{},\"k6\":{},\"b1\":{},\"b2\":{},\"b3\":{},\"b4\":{},\"b5\":{},\"b6\":{},\"ge1\":{},\"ge2\":{},\"ge3\":{},\"ge4\":{},\"ge5\":{},\"ge6\":{},\"sa\":{},\"sb\":{}}}",
        serialize_opt_i64(card.pid),
        serialize_opt_f64(card.k1),
        serialize_opt_f64(card.k2),
        serialize_opt_f64(card.k3),
        serialize_opt_f64(card.k4),
        serialize_opt_f64(card.k5),
        serialize_opt_f64(card.k6),
        serialize_opt_f64(card.b1),
        serialize_opt_f64(card.b2),
        serialize_opt_f64(card.b3),
        serialize_opt_f64(card.b4),
        serialize_opt_f64(card.b5),
        serialize_opt_f64(card.b6),
        serialize_opt_f64(card.ge1),
        serialize_opt_f64(card.ge2),
        serialize_opt_f64(card.ge3),
        serialize_opt_f64(card.ge4),
        serialize_opt_f64(card.ge5),
        serialize_opt_f64(card.ge6),
        serialize_opt_f64(card.sa),
        serialize_opt_f64(card.sb)
    )
}

fn serialize_pshell_card(card: &PshellCard) -> String {
    format!(
        "{{\"pid\":{},\"mid1\":{},\"t\":{},\"mid2\":{},\"twelve_i_over_t3\":{},\"mid3\":{},\"ts_over_t\":{},\"nsm\":{},\"z1\":{},\"z2\":{},\"mid4\":{}}}",
        serialize_opt_i64(card.pid),
        serialize_opt_i64(card.mid1),
        serialize_opt_f64(card.t),
        serialize_opt_i64(card.mid2),
        serialize_opt_f64(card.twelve_i_over_t3),
        serialize_opt_i64(card.mid3),
        serialize_opt_f64(card.ts_over_t),
        serialize_opt_f64(card.nsm),
        serialize_opt_f64(card.z1),
        serialize_opt_f64(card.z2),
        serialize_opt_i64(card.mid4)
    )
}

fn serialize_mat1_card(card: &Mat1Card) -> String {
    format!(
        "{{\"mid\":{},\"e\":{},\"g\":{},\"nu\":{},\"rho\":{},\"a\":{},\"tref\":{},\"ge\":{},\"st\":{},\"sc\":{},\"ss\":{},\"mcsid\":{}}}",
        serialize_opt_i64(card.mid),
        serialize_opt_f64(card.e),
        serialize_opt_f64(card.g),
        serialize_opt_f64(card.nu),
        serialize_opt_f64(card.rho),
        serialize_opt_f64(card.a),
        serialize_opt_f64(card.tref),
        serialize_opt_f64(card.ge),
        serialize_opt_f64(card.st),
        serialize_opt_f64(card.sc),
        serialize_opt_f64(card.ss),
        serialize_opt_i64(card.mcsid)
    )
}

fn serialize_rbe2_card(card: &Rbe2Card) -> String {
    let gm = card
        .gm
        .iter()
        .map(|value| serialize_opt_i64(*value))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"eid\":{},\"gn\":{},\"cm\":{},\"gm\":[{}],\"alpha\":{}}}",
        serialize_opt_i64(card.eid),
        serialize_opt_i64(card.gn),
        serialize_opt_string(card.cm.as_deref()),
        gm,
        serialize_opt_f64(card.alpha)
    )
}

fn serialize_rbe3_card(card: &Rbe3Card) -> String {
    format!(
        "{{\"eid\":{},\"refgrid\":{},\"refc\":{},\"remaining_fields\":{},\"alpha\":{}}}",
        serialize_opt_i64(card.eid),
        serialize_opt_i64(card.refgrid),
        serialize_opt_string(card.refc.as_deref()),
        serialize_string_array(&card.remaining_fields),
        serialize_opt_f64(card.alpha)
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

#[cfg(test)]
mod tests {
    use super::{
        SCHEMA_CONTRACT_MARKER, schema_contract_marker, serialize_compact,
        serialize_optistruct_item_compact,
    };
    use crate::model::{
        BeginBulkCard, Model, ModelContent, OptiStructCardCount, OptiStructCardItem,
        OptiStructCardKind, OptiStructFileSummary, OptiStructItem, OptiStructLocation,
        OptiStructModelSummary, SCHEMA_VERSION, SolverKind,
    };
    use std::path::PathBuf;

    fn short_card(name: &str, values: &[&str]) -> String {
        let mut lines = Vec::new();
        for (line_index, chunk) in values.chunks(9).enumerate() {
            let mut line = if line_index == 0 {
                format!("{name:<8}")
            } else {
                "*       ".to_string()
            };
            for value in chunk {
                line.push_str(&format!("{value:<8}"));
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }

    fn sample_content() -> ModelContent {
        let location = OptiStructLocation {
            file_id: "file-1".to_string(),
            file_name: "main.fem".to_string(),
            file_path: PathBuf::from("main.fem"),
            start_line: 1,
            end_line: 1,
            ordinal: 1,
        };

        ModelContent {
            schema_version: SCHEMA_VERSION.to_string(),
            root_directory: PathBuf::from("C:\\models"),
            directories: Vec::new(),
            files: Vec::new(),
            dependencies: Vec::new(),
            unresolved_references: Vec::new(),
            optistruct_items: vec![OptiStructItem::Card(OptiStructCardItem {
                id: "item-1".to_string(),
                location: location.clone(),
                text: "BEGIN BULK".to_string(),
                card_name: "BEGIN BULK".to_string(),
                long_format: false,
                fields: Vec::new(),
                kind: OptiStructCardKind::BeginBulk(BeginBulkCard),
            })],
            optistruct_file_summaries: vec![OptiStructFileSummary {
                file_id: "file-1".to_string(),
                file_name: "main.fem".to_string(),
                file_path: PathBuf::from("main.fem"),
                total_non_empty_lines: 1,
                covered_non_empty_lines: 1,
                line_coverage_ok: true,
                total_items: 1,
                comment_items: 0,
                hypermesh_comment_items: 0,
                text_items: 0,
                card_items: 1,
                card_counts: vec![OptiStructCardCount {
                    card_name: "BEGIN BULK".to_string(),
                    count: 1,
                }],
            }],
            optistruct_model_summaries: vec![OptiStructModelSummary {
                model_id: "model-1".to_string(),
                model_name: "main.fem".to_string(),
                total_non_empty_lines: 1,
                covered_non_empty_lines: 1,
                line_coverage_ok: true,
                total_items: 1,
                comment_items: 0,
                hypermesh_comment_items: 0,
                text_items: 0,
                card_items: 1,
                card_counts: vec![OptiStructCardCount {
                    card_name: "BEGIN BULK".to_string(),
                    count: 1,
                }],
            }],
            models: vec![Model {
                id: "model-1".to_string(),
                name: "main.fem".to_string(),
                root_file_id: "file-1".to_string(),
                file_ids: vec!["file-1".to_string()],
                solver: SolverKind::OptiStruct,
            }],
        }
    }

    #[test]
    fn card_items_use_text_key_and_begin_bulk_has_no_fields() {
        let json = serialize_compact(&sample_content());
        assert!(json.contains("\"type\":\"Card\""));
        assert!(json.contains("\"text\":\"BEGIN BULK\""));
        assert!(json.contains("\"card_name\":\"BEGIN BULK\""));
        assert!(json.contains("\"fields\":[]"));
        assert!(!json.contains("\"raw_text\""));
        assert!(!json.contains("\"kind\":"));
    }

    #[test]
    fn new_optistruct_cards_serialize_typed_attributes_inside_card_items() {
        let source = [
            short_card(
                "CBUSH",
                &["601", "3", "604", "263", ".5773503", ".5773503", ".5773503"],
            ),
            short_card(
                "PBUSH",
                &[
                    "30", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13",
                    "14", "15", "16", "17", "18", "19", "20",
                ],
            ),
            short_card(
                "PSHELL",
                &[
                    "40", "50", "0.25", "51", "0.083", "52", "0.1", "0.01", "-0.125", "0.125", "53",
                ],
            ),
            short_card(
                "MAT1",
                &[
                    "60", "210000", "80000", "0.3", "7.8e-9", "1.2e-5", "20", "0.02", "100", "200",
                    "300", "4",
                ],
            ),
            short_card("RBE2", &["70", "1", "123", "2", "3", "4", "5", "0.01"]),
            short_card(
                "RBE3",
                &[
                    "80", "", "1", "123", "1.0", "123", "2", "3", "2.0", "456", "4", "5", "UM",
                    "7", "123", "ALPHA", "1.45e-5",
                ],
            ),
        ]
        .join("\n");
        let items = crate::optistruct::parse_file_items(
            "file:serialized-cards",
            &PathBuf::from("cards.fem"),
            &source,
        );
        let serialized = items
            .iter()
            .map(serialize_optistruct_item_compact)
            .collect::<Vec<_>>();

        assert!(serialized[0].contains("\"type\":\"Card\""));
        assert!(serialized[0].contains("\"card_name\":\"CBUSH\""));
        assert!(serialized[0].contains("\"attributes\":{\"eid\":601,\"pid\":3"));
        assert!(serialized[0].contains("\"x1\":0.5773503"));
        assert!(serialized[0].contains("\"x2\":0.5773503"));
        assert!(serialized[0].contains("\"x3\":0.5773503"));
        assert!(serialized[1].contains("\"card_name\":\"PBUSH\""));
        assert!(serialized[1].contains("\"k1\":1,"));
        assert!(serialized[1].contains("\"sb\":20}"));
        assert!(serialized[2].contains("\"card_name\":\"PSHELL\""));
        assert!(serialized[2].contains("\"twelve_i_over_t3\":0.083"));
        assert!(serialized[3].contains("\"card_name\":\"MAT1\""));
        assert!(serialized[3].contains("\"mcsid\":4"));
        assert!(serialized[4].contains("\"card_name\":\"RBE2\""));
        assert!(serialized[4].contains("\"gm\":[2,3,4,5]"));
        assert!(serialized[4].contains("\"alpha\":0.01"));
        assert!(serialized[5].contains("\"card_name\":\"RBE3\""));
        assert!(serialized[5].contains("\"remaining_fields\":[\"1.0\",\"123\""));
        assert!(serialized[5].contains("\"UM\",\"7\",\"123\"]"));
        assert!(serialized[5].contains("\"alpha\":0.0000145"));
    }

    #[test]
    fn schema_version_is_reviewed_with_contract_marker() {
        let approved_versions = [("1.1.0", SCHEMA_CONTRACT_MARKER)];
        assert!(
            approved_versions
                .iter()
                .any(|(version, marker)| *version == SCHEMA_VERSION
                    && *marker == schema_contract_marker()),
            "Cambió el contrato JSON. Actualiza SCHEMA_VERSION y el marcador de contrato aprobado."
        );
    }
}
