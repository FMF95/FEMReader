use crate::ids::{leaf_name, stable_id};
use crate::model::{
    BeginBulkCard, Cquad4Card, Ctria3Card, EndDataCard, ForceCard, GridCard, HMDPrpCommentFields,
    HMMoveCommentFields, HMNameCommentFields, HWColorCommentFields, HyperMeshCommentFields,
    IncludeCard, MomentCard, OptiStructCardCount, OptiStructCardItem, OptiStructCardKind,
    OptiStructCommentItem, OptiStructCommentKind, OptiStructFileSummary, OptiStructItem,
    OptiStructLocation, OptiStructTextItem, UnknownCard,
};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawCardRecord {
    name: String,
    long_format: bool,
    fields: Vec<String>,
    text: String,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RawRecord {
    Comment {
        start_line: usize,
        end_line: usize,
        text: String,
    },
    Text {
        line_number: usize,
        text: String,
    },
    Card(RawCardRecord),
}

pub fn parse_file_items(file_id: &str, file_path: &Path, content: &str) -> Vec<OptiStructItem> {
    let records = parse_raw_records(content);
    let mut items = Vec::with_capacity(records.len());
    let file_name = leaf_name(file_path);

    for (index, record) in records.into_iter().enumerate() {
        let ordinal = index + 1;
        match record {
            RawRecord::Comment {
                start_line,
                end_line,
                text,
            } => {
                let location = OptiStructLocation {
                    file_id: file_id.to_string(),
                    file_name: file_name.clone(),
                    file_path: file_path.to_path_buf(),
                    start_line,
                    end_line,
                    ordinal,
                };
                let kind = classify_comment_kind(&text);
                let item_id = stable_id(
                    "osic",
                    &format!(
                        "{}|{}|{}|{}",
                        location.file_id, location.start_line, location.ordinal, text
                    ),
                );
                items.push(OptiStructItem::Comment(OptiStructCommentItem {
                    id: item_id,
                    location,
                    kind,
                    text,
                }));
            }
            RawRecord::Text { line_number, text } => {
                let location = OptiStructLocation {
                    file_id: file_id.to_string(),
                    file_name: file_name.clone(),
                    file_path: file_path.to_path_buf(),
                    start_line: line_number,
                    end_line: line_number,
                    ordinal,
                };
                let item_id = stable_id(
                    "osit",
                    &format!(
                        "{}|{}|{}|{}|text",
                        location.file_id, location.start_line, location.ordinal, text
                    ),
                );
                items.push(OptiStructItem::Text(OptiStructTextItem {
                    id: item_id,
                    location,
                    text,
                }));
            }
            RawRecord::Card(card) => {
                let kind = parse_card_kind(&card.name, &card.fields);
                let location = OptiStructLocation {
                    file_id: file_id.to_string(),
                    file_name: file_name.clone(),
                    file_path: file_path.to_path_buf(),
                    start_line: card.start_line,
                    end_line: card.end_line,
                    ordinal,
                };
                let item_id = stable_id(
                    "osit",
                    &format!(
                        "{}|{}|{}|{}|{}",
                        location.file_id,
                        location.start_line,
                        location.end_line,
                        location.ordinal,
                        card.name
                    ),
                );
                items.push(OptiStructItem::Card(OptiStructCardItem {
                    id: item_id,
                    location,
                    text: card.text,
                    card_name: card.name.clone(),
                    long_format: card.long_format,
                    fields: card.fields,
                    kind,
                }));
            }
        }
    }

    items
}

pub fn summarize_file_items(
    file_id: &str,
    file_path: &Path,
    content: &str,
    items: &[OptiStructItem],
) -> OptiStructFileSummary {
    let mut comment_items = 0usize;
    let mut hypermesh_comment_items = 0usize;
    let mut text_items = 0usize;
    let mut card_items = 0usize;
    let mut card_counts = BTreeMap::<String, usize>::new();

    for item in items {
        match item {
            OptiStructItem::Comment(comment) => {
                comment_items += 1;
                if matches!(comment.kind, OptiStructCommentKind::HyperMeshComment { .. }) {
                    hypermesh_comment_items += 1;
                }
            }
            OptiStructItem::Text(_) => {
                text_items += 1;
            }
            OptiStructItem::Card(card) => {
                card_items += 1;
                *card_counts.entry(card.card_name.clone()).or_insert(0) += 1;
            }
        }
    }

    let card_counts = card_counts
        .into_iter()
        .map(|(card_name, count)| OptiStructCardCount { card_name, count })
        .collect::<Vec<_>>();
    let total_non_empty_lines = count_non_empty_lines(content);
    let covered_non_empty_lines = count_covered_non_empty_lines(items);

    OptiStructFileSummary {
        file_id: file_id.to_string(),
        file_name: leaf_name(file_path),
        file_path: file_path.to_path_buf(),
        total_non_empty_lines,
        covered_non_empty_lines,
        line_coverage_ok: covered_non_empty_lines == total_non_empty_lines,
        total_items: items.len(),
        comment_items,
        hypermesh_comment_items,
        text_items,
        card_items,
        card_counts,
    }
}

pub fn extract_include_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();
    for record in parse_raw_records(content) {
        let RawRecord::Card(card) = record else {
            continue;
        };
        if card.name.eq_ignore_ascii_case("INCLUDE")
            && let Some(target) = parse_include_target(&card.fields)
        {
            references.push(target);
        }
    }
    references
}

fn parse_raw_records(content: &str) -> Vec<RawRecord> {
    let mut records = Vec::new();
    let mut current_card: Option<RawCardRecord> = None;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_number = idx + 1;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            flush_pending_card(&mut current_card, &mut records);
            continue;
        }

        let (payload, inline_comment) = split_comment(line);
        if payload.trim().is_empty() {
            flush_pending_card(&mut current_card, &mut records);
            if let Some(comment) = inline_comment {
                if should_merge_comment_continuation(&comment, &records) {
                    merge_comment_continuation(&comment, line_number, &mut records);
                    continue;
                }
                records.push(RawRecord::Comment {
                    start_line: line_number,
                    end_line: line_number,
                    text: comment,
                });
            }
            continue;
        }

        if is_continuation_line(payload) {
            if let Some(card) = current_card.as_mut() {
                extend_card_with_continuation(card, payload, line_number);
            } else {
                records.push(RawRecord::Text {
                    line_number,
                    text: payload.to_string(),
                });
            }
            if let Some(comment) = inline_comment {
                records.push(RawRecord::Comment {
                    start_line: line_number,
                    end_line: line_number,
                    text: comment,
                });
            }
            continue;
        }

        flush_pending_card(&mut current_card, &mut records);
        if let Some(card) = parse_card_start(payload, line_number) {
            current_card = Some(card);
        } else {
            records.push(RawRecord::Text {
                line_number,
                text: payload.to_string(),
            });
            current_card = None;
        }

        if let Some(comment) = inline_comment {
            records.push(RawRecord::Comment {
                start_line: line_number,
                end_line: line_number,
                text: comment,
            });
        }
    }

    flush_pending_card(&mut current_card, &mut records);
    records.sort_by_key(|record| match record {
        RawRecord::Card(card) => (card.start_line, 0usize),
        RawRecord::Text { line_number, .. } => (*line_number, 0usize),
        RawRecord::Comment { start_line, .. } => (*start_line, 1usize),
    });
    records
}

fn parse_card_kind(card_name: &str, fields: &[String]) -> OptiStructCardKind {
    match card_name.to_ascii_uppercase().as_str() {
        "GRID" => OptiStructCardKind::Grid(GridCard {
            id: field_i64(fields, 0),
            cp: field_i64(fields, 1),
            x1: field_f64(fields, 2),
            x2: field_f64(fields, 3),
            x3: field_f64(fields, 4),
            cd: field_i64(fields, 5),
            ps: field_string(fields, 6),
            seid: field_i64(fields, 7),
        }),
        "CQUAD4" => OptiStructCardKind::Cquad4(Cquad4Card {
            id: field_i64(fields, 0),
            pid: field_i64(fields, 1),
            g1: field_i64(fields, 2),
            g2: field_i64(fields, 3),
            g3: field_i64(fields, 4),
            g4: field_i64(fields, 5),
            theta_or_mcid: field_string(fields, 6),
            zoffs: field_f64(fields, 7),
            tflag: field_i64(fields, 8),
            t1: field_f64(fields, 9),
            t2: field_f64(fields, 10),
            t3: field_f64(fields, 11),
            t4: field_f64(fields, 12),
        }),
        "CTRIA3" => OptiStructCardKind::Ctria3(Ctria3Card {
            id: field_i64(fields, 0),
            pid: field_i64(fields, 1),
            g1: field_i64(fields, 2),
            g2: field_i64(fields, 3),
            g3: field_i64(fields, 4),
            theta_or_mcid: field_string(fields, 5),
            zoffs: field_f64(fields, 6),
            tflag: field_i64(fields, 7),
            t1: field_f64(fields, 8),
            t2: field_f64(fields, 9),
            t3: field_f64(fields, 10),
        }),
        "FORCE" => OptiStructCardKind::Force(ForceCard {
            sid: field_i64(fields, 0),
            g: field_i64(fields, 1),
            cid: field_i64(fields, 2),
            f: field_f64(fields, 3),
            n1: field_f64(fields, 4),
            n2: field_f64(fields, 5),
            n3: field_f64(fields, 6),
        }),
        "MOMENT" => OptiStructCardKind::Moment(MomentCard {
            sid: field_i64(fields, 0),
            g: field_i64(fields, 1),
            cid: field_i64(fields, 2),
            m: field_f64(fields, 3),
            n1: field_f64(fields, 4),
            n2: field_f64(fields, 5),
            n3: field_f64(fields, 6),
        }),
        "INCLUDE" => OptiStructCardKind::Include(IncludeCard {
            target: parse_include_target(fields).unwrap_or_default(),
        }),
        "BEGIN BULK" => OptiStructCardKind::BeginBulk(BeginBulkCard),
        "ENDDATA" => OptiStructCardKind::EndData(EndDataCard),
        _ => OptiStructCardKind::Unknown(UnknownCard {
            card_name: card_name.to_string(),
        }),
    }
}

fn classify_comment_kind(text: &str) -> OptiStructCommentKind {
    const HYPERMESH_TAGS: [&str; 4] = ["HMNAME", "HWCOLOR", "HMMOVE", "HMDPRP"];
    let normalized = normalize_comment_for_tokenization(text);
    let tokens = tokenize_hypermesh_fields(&normalized);
    let Some(tag) = tokens.first() else {
        return OptiStructCommentKind::Comment;
    };
    let upper_tag = tag.to_ascii_uppercase();

    if !HYPERMESH_TAGS.iter().any(|known| *known == upper_tag) {
        return OptiStructCommentKind::Comment;
    }

    let field_tokens = tokens.into_iter().skip(1).collect::<Vec<_>>();
    let fields = parse_hypermesh_comment_fields(&upper_tag, field_tokens);
    OptiStructCommentKind::HyperMeshComment {
        tag: upper_tag,
        fields,
    }
}

fn parse_hypermesh_comment_fields(tag: &str, tokens: Vec<String>) -> HyperMeshCommentFields {
    match tag {
        "HMNAME" => HyperMeshCommentFields::HMName(HMNameCommentFields {
            entity_type: tokens.first().cloned(),
            id: tokens.get(1).cloned(),
            name: tokens.get(2).cloned(),
            id_pool: tokens.get(3).cloned(),
            raw_fields: tokens,
        }),
        "HWCOLOR" => HyperMeshCommentFields::HWColor(HWColorCommentFields {
            entity_type: tokens.first().cloned(),
            id: tokens.get(1).cloned(),
            color_id: tokens.get(2).cloned(),
            raw_fields: tokens,
        }),
        "HMMOVE" => HyperMeshCommentFields::HMMove(HMMoveCommentFields {
            collector_id: tokens.first().cloned(),
            raw_fields: tokens,
        }),
        "HMDPRP" => {
            let first = tokens.first().cloned();
            let first_is_keyword = first
                .as_ref()
                .is_some_and(|value| value.chars().all(|ch| ch.is_ascii_alphabetic()));
            let (elements, ids) = if first_is_keyword {
                (first, tokens.iter().skip(1).cloned().collect())
            } else {
                (None, tokens.clone())
            };
            HyperMeshCommentFields::HMDPrp(HMDPrpCommentFields {
                elements,
                ids,
                raw_fields: tokens,
            })
        }
        _ => HyperMeshCommentFields::HMMove(HMMoveCommentFields {
            collector_id: None,
            raw_fields: tokens,
        }),
    }
}

fn tokenize_hypermesh_fields(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in text.chars() {
        match ch {
            '"' => {
                if in_quotes {
                    if !current.trim().is_empty() {
                        tokens.push(current.trim().to_string());
                    }
                    current.clear();
                    in_quotes = false;
                } else {
                    if !current.trim().is_empty() {
                        tokens.push(current.trim().to_string());
                    }
                    current.clear();
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                }
                current.clear();
            }
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

fn field_string(fields: &[String], index: usize) -> Option<String> {
    fields.get(index + 1).and_then(|value| {
        let normalized = value.trim();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.to_string())
        }
    })
}

fn field_i64(fields: &[String], index: usize) -> Option<i64> {
    field_string(fields, index).and_then(|value| value.parse::<i64>().ok())
}

fn field_f64(fields: &[String], index: usize) -> Option<f64> {
    field_string(fields, index).and_then(|value| value.parse::<f64>().ok())
}

fn parse_include_target(fields: &[String]) -> Option<String> {
    let candidate = fields.iter().skip(1).cloned().collect::<Vec<_>>().join("");
    let normalized = candidate.trim().trim_matches('\'').trim_matches('"');
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn split_comment(line: &str) -> (&str, Option<String>) {
    let Some(index) = line.find('$') else {
        return (line, None);
    };
    let payload = &line[..index];
    let comment = line[index..].to_string();
    (payload, Some(comment))
}

fn should_merge_comment_continuation(comment: &str, records: &[RawRecord]) -> bool {
    if !has_hypermesh_continuation_prefix(comment) {
        return false;
    }
    if extract_comment_continuation_fields(comment).is_empty() {
        return false;
    }

    matches!(
        records.last(),
        Some(RawRecord::Comment { text, .. })
            if matches!(
                extract_hypermesh_tag(text).as_deref(),
                Some("HMMOVE" | "HMDPRP")
            )
    )
}

fn merge_comment_continuation(comment: &str, line_number: usize, records: &mut [RawRecord]) {
    if let Some(RawRecord::Comment { end_line, text, .. }) = records.last_mut() {
        text.push('\n');
        text.push_str(comment);
        *end_line = line_number;
    }
}

fn normalize_comment_for_tokenization(text: &str) -> String {
    text.lines()
        .map(|line| line.trim().trim_start_matches('$').trim_start().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_continuation_line(line: &str) -> bool {
    line.trim_start().starts_with('*')
}

fn flush_pending_card(card: &mut Option<RawCardRecord>, records: &mut Vec<RawRecord>) {
    if let Some(card) = card.take() {
        records.push(RawRecord::Card(card));
    }
}

fn parse_card_start(line: &str, line_number: usize) -> Option<RawCardRecord> {
    let trimmed = line.trim();
    let compact = trimmed
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    if compact.eq_ignore_ascii_case("BEGINBULK") {
        return Some(begin_bulk_record(line_number, line));
    }

    let first_line_slots = split_fixed_width_fields(line, 8, Some(10));
    if is_begin_bulk_slots(&first_line_slots) {
        return Some(begin_bulk_record(line_number, line));
    }
    let first_field = first_line_slots.first()?.clone();
    let first_field = first_field.trim();
    if first_field.is_empty() {
        return None;
    }

    let long_format = first_field.ends_with('*');
    let card_name = first_field.trim_end_matches('*').trim();
    if card_name.is_empty() {
        return None;
    }
    if !is_supported_card_name(card_name) {
        return None;
    }
    if card_name.eq_ignore_ascii_case("ENDDATA") {
        return Some(RawCardRecord {
            name: "ENDDATA".to_string(),
            long_format: false,
            fields: Vec::new(),
            text: line.to_string(),
            start_line: line_number,
            end_line: line_number,
        });
    }

    let mut fields = if long_format {
        if line.len() > 8 {
            let mut fields = vec![first_field.to_string()];
            fields.extend(split_fixed_width_fields(&line[8..], 16, None));
            fields
        } else {
            vec![first_field.to_string()]
        }
    } else {
        first_line_slots
    };
    trim_trailing_empty_fields(&mut fields);

    Some(RawCardRecord {
        name: card_name.to_string(),
        long_format,
        fields,
        text: line.to_string(),
        start_line: line_number,
        end_line: line_number,
    })
}

fn extend_card_with_continuation(card: &mut RawCardRecord, line: &str, line_number: usize) {
    card.text.push('\n');
    card.text.push_str(line);
    let payload = line.trim_start().trim_start_matches('*');
    let width = if card.long_format { 16 } else { 8 };
    let mut continuation_fields = split_fixed_width_fields(payload, width, None);
    trim_trailing_empty_fields(&mut continuation_fields);
    card.fields.extend(continuation_fields);
    card.end_line = line_number;
}

fn begin_bulk_record(line_number: usize, raw_line: &str) -> RawCardRecord {
    RawCardRecord {
        name: "BEGIN BULK".to_string(),
        long_format: false,
        fields: Vec::new(),
        text: raw_line.to_string(),
        start_line: line_number,
        end_line: line_number,
    }
}

fn is_begin_bulk_slots(slots: &[String]) -> bool {
    let combined = slots
        .iter()
        .take(2)
        .map(|value| {
            value
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>()
        })
        .collect::<String>()
        .to_ascii_uppercase();
    combined == "BEGINBULK"
}

fn is_supported_card_name(card_name: &str) -> bool {
    matches!(
        card_name.to_ascii_uppercase().as_str(),
        "GRID" | "CQUAD4" | "CTRIA3" | "FORCE" | "MOMENT" | "INCLUDE" | "ENDDATA"
    )
}

fn count_non_empty_lines(content: &str) -> usize {
    content
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn count_covered_non_empty_lines(items: &[OptiStructItem]) -> usize {
    let mut covered = std::collections::BTreeSet::<usize>::new();
    for item in items {
        let (start, end) = match item {
            OptiStructItem::Comment(comment) => {
                (comment.location.start_line, comment.location.end_line)
            }
            OptiStructItem::Text(text) => (text.location.start_line, text.location.end_line),
            OptiStructItem::Card(card) => (card.location.start_line, card.location.end_line),
        };
        for line in start..=end {
            covered.insert(line);
        }
    }
    covered.len()
}

fn extract_hypermesh_tag(text: &str) -> Option<String> {
    let normalized = normalize_comment_for_tokenization(text);
    let tokens = tokenize_hypermesh_fields(&normalized);
    let tag = tokens.first()?.to_ascii_uppercase();
    matches!(tag.as_str(), "HMNAME" | "HWCOLOR" | "HMMOVE" | "HMDPRP").then_some(tag)
}

fn has_hypermesh_continuation_prefix(comment: &str) -> bool {
    let Some(payload) = comment.strip_prefix('$') else {
        return false;
    };
    let leading_whitespace = payload.chars().take_while(|ch| ch.is_whitespace()).count();
    leading_whitespace >= 8
}

fn extract_comment_continuation_fields(comment: &str) -> Vec<String> {
    let payload = comment.strip_prefix('$').unwrap_or(comment);
    payload
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>()
}

fn split_fixed_width_fields(line: &str, width: usize, max_fields: Option<usize>) -> Vec<String> {
    let mut fields = Vec::new();
    let mut start = 0usize;

    while start < line.len() {
        if let Some(limit) = max_fields
            && fields.len() >= limit
        {
            break;
        }

        let end = (start + width).min(line.len());
        let slice = &line[start..end];
        fields.push(slice.trim().to_string());
        start += width;
    }

    fields
}

fn trim_trailing_empty_fields(fields: &mut Vec<String>) {
    while fields.last().is_some_and(String::is_empty) {
        fields.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_file_items, summarize_file_items};
    use crate::model::{
        HMDPrpCommentFields, HMMoveCommentFields, HMNameCommentFields, HWColorCommentFields,
        HyperMeshCommentFields, OptiStructCardKind, OptiStructCommentKind, OptiStructItem,
    };
    use std::path::Path;

    #[test]
    fn parses_grid_cquad4_ctria3_force_moment_include_begin_and_end_cards() {
        let content = "\
$ Inicio
BEGIN   BULK
GRID    100     0       1.0     2.0     3.0
CQUAD4  200     10      1       2       3       4
CTRIA3  201     10      1       3       4
FORCE   1       100     0       500.0   1.0     0.0     0.0
MOMENT  2       100     0       300.0   0.0     1.0     0.0
INCLUDE \"submodel.fem\"
ENDDATA
";
        let items = parse_file_items("file:demo", Path::new("demo.fem"), content);
        let cards = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Card(card) => Some(card),
                OptiStructItem::Comment(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(cards.len(), 8);
        assert!(matches!(cards[0].kind, OptiStructCardKind::BeginBulk(_)));
        assert!(matches!(cards[1].kind, OptiStructCardKind::Grid(_)));
        assert!(matches!(cards[2].kind, OptiStructCardKind::Cquad4(_)));
        assert!(matches!(cards[3].kind, OptiStructCardKind::Ctria3(_)));
        assert!(matches!(cards[4].kind, OptiStructCardKind::Force(_)));
        assert!(matches!(cards[5].kind, OptiStructCardKind::Moment(_)));
        assert!(matches!(cards[6].kind, OptiStructCardKind::Include(_)));
        assert!(matches!(cards[7].kind, OptiStructCardKind::EndData(_)));
        assert_eq!(cards[1].fields.first().map(String::as_str), Some("GRID"));

        if let OptiStructCardKind::Grid(grid) = &cards[1].kind {
            assert_eq!(grid.id, Some(100));
            assert_eq!(grid.x1, Some(1.0));
            assert_eq!(grid.x2, Some(2.0));
            assert_eq!(grid.x3, Some(3.0));
        } else {
            panic!("GRID card expected");
        }
    }

    #[test]
    fn keeps_comment_positions_and_card_file_metadata() {
        let content =
            "$ Encabezado\nGRID    1       0       0.0     0.0     0.0 $ inline\n$ Final\n";
        let items = parse_file_items("file:meta", Path::new("model.fem"), content);

        assert_eq!(items.len(), 4);
        match &items[0] {
            OptiStructItem::Comment(comment) => {
                assert_eq!(comment.location.start_line, 1);
                assert_eq!(comment.kind, OptiStructCommentKind::Comment);
                assert_eq!(comment.text, "$ Encabezado");
            }
            _ => panic!("expected comment in line 1"),
        }

        match &items[1] {
            OptiStructItem::Card(card) => {
                assert_eq!(card.location.file_id, "file:meta");
                assert_eq!(card.location.start_line, 2);
                assert_eq!(card.location.end_line, 2);
            }
            _ => panic!("expected card in line 2"),
        }

        match &items[2] {
            OptiStructItem::Comment(comment) => {
                assert_eq!(comment.location.start_line, 2);
                assert_eq!(comment.kind, OptiStructCommentKind::Comment);
                assert_eq!(comment.text, "$ inline");
            }
            _ => panic!("expected inline comment in line 2"),
        }

        match &items[3] {
            OptiStructItem::Comment(comment) => {
                assert_eq!(comment.location.start_line, 3);
                assert_eq!(comment.kind, OptiStructCommentKind::Comment);
                assert_eq!(comment.text, "$ Final");
            }
            _ => panic!("expected final comment in line 3"),
        }
    }

    #[test]
    fn classifies_hypermesh_comments() {
        let content = "$HMNAME COMP 1\"panel\" MAIN\n$HWCOLOR COMP 1 5\n$ texto normal\n";
        let items = parse_file_items("file:hm", Path::new("hm.fem"), content);
        let comments = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Comment(comment) => Some(comment),
                OptiStructItem::Card(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(comments.len(), 3);
        assert!(matches!(
            comments[0].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMName(HMNameCommentFields {
                    entity_type: Some(ref entity_type),
                    id: Some(ref id),
                    name: Some(ref name),
                    id_pool: Some(ref id_pool),
                    ..
                })
                } if tag == "HMNAME"
                    && entity_type == "COMP"
                    && id == "1"
                && name == "panel"
                && id_pool == "MAIN"
        ));
        assert!(matches!(
            comments[1].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HWColor(HWColorCommentFields {
                    entity_type: Some(ref entity_type),
                    id: Some(ref id),
                    color_id: Some(ref color_id),
                    ..
                })
            } if tag == "HWCOLOR"
                && entity_type == "COMP"
                && id == "1"
                && color_id == "5"
        ));
        assert_eq!(comments[2].kind, OptiStructCommentKind::Comment);
    }

    #[test]
    fn parses_hmmove_and_hmdprp_fields() {
        let content =
            "$HMMOVE        1\n$              1THRU         100\n$HMDPRP elements 10 11 12\n";
        let items = parse_file_items("file:hm2", Path::new("hm2.fem"), content);
        let comments = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Comment(comment) => Some(comment),
                OptiStructItem::Card(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(comments.len(), 2);
        assert!(matches!(
            comments[0].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMMove(HMMoveCommentFields {
                    collector_id: Some(ref collector_id),
                    ref raw_fields,
                    ..
                })
            } if tag == "HMMOVE"
                && collector_id == "1"
                && raw_fields == &vec!["1".to_string(), "1THRU".to_string(), "100".to_string()]
        ));

        assert!(matches!(
            comments[1].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMDPrp(HMDPrpCommentFields {
                    elements: Some(ref elements),
                    ref ids,
                    ..
                })
            } if tag == "HMDPRP" && elements == "elements" && ids == &vec!["10".to_string(), "11".to_string(), "12".to_string()]
        ));
    }

    #[test]
    fn parses_hypermesh_examples_with_fixed_width_continuations() {
        let content = "\
$HMMOVE        1
$              1THRU          98     100      99
$HMDPRP
$              1
";
        let items = parse_file_items("file:hm3", Path::new("hm3.fem"), content);
        let comments = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Comment(comment) => Some(comment),
                OptiStructItem::Card(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(comments.len(), 2);
        assert!(matches!(
            comments[0].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMMove(HMMoveCommentFields {
                    collector_id: Some(ref collector_id),
                    ref raw_fields,
                    ..
                })
            } if tag == "HMMOVE"
                && collector_id == "1"
                && raw_fields
                    == &vec![
                        "1".to_string(),
                        "1THRU".to_string(),
                        "98".to_string(),
                        "100".to_string(),
                        "99".to_string()
                    ]
        ));
        assert!(matches!(
            comments[1].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMDPrp(HMDPrpCommentFields {
                    elements: None,
                    ref ids,
                    ..
                })
            } if tag == "HMDPRP" && ids == &vec!["1".to_string()]
        ));
    }

    #[test]
    fn parses_hmname_examples_with_quoted_name_attached_to_id() {
        let content = "\
$HMNAME PROP                   1\"PCOMPP\" 4
$HMNAME COMP                   1\"Plate\"
";
        let items = parse_file_items("file:hm4", Path::new("hm4.fem"), content);
        let comments = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Comment(comment) => Some(comment),
                OptiStructItem::Card(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(comments.len(), 2);
        assert!(matches!(
            comments[0].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMName(HMNameCommentFields {
                    entity_type: Some(ref entity_type),
                    id: Some(ref id),
                    name: Some(ref name),
                    id_pool: Some(ref id_pool),
                    ..
                })
            } if tag == "HMNAME"
                && entity_type == "PROP"
                && id == "1"
                && name == "PCOMPP"
                && id_pool == "4"
        ));
        assert!(matches!(
            comments[1].kind,
            OptiStructCommentKind::HyperMeshComment {
                ref tag,
                fields: HyperMeshCommentFields::HMName(HMNameCommentFields {
                    entity_type: Some(ref entity_type),
                    id: Some(ref id),
                    name: Some(ref name),
                    id_pool: None,
                    ..
                })
            } if tag == "HMNAME"
                && entity_type == "COMP"
                && id == "1"
                && name == "Plate"
        ));
    }

    #[test]
    fn parses_long_include_with_continuation() {
        let content = "INCLUDE*\"folder/submod\n*       el.inc\"\n";
        let items = parse_file_items("file:inc", Path::new("model.fem"), content);
        let include = items
            .iter()
            .find_map(|item| match item {
                OptiStructItem::Card(card) => match &card.kind {
                    OptiStructCardKind::Include(include) => Some(include),
                    _ => None,
                },
                OptiStructItem::Comment(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .expect("INCLUDE card should exist");

        assert_eq!(include.target, "folder/submodel.inc");
    }

    #[test]
    fn parses_begin_bulk_with_single_space_separator() {
        let content = "BEGIN BULK\nGRID    1       0       0.0     0.0     0.0\nENDDATA\n";
        let items = parse_file_items("file:bulk", Path::new("bulk.fem"), content);
        let cards = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Card(card) => Some(card),
                OptiStructItem::Comment(_) => None,
                OptiStructItem::Text(_) => None,
            })
            .collect::<Vec<_>>();

        assert!(matches!(cards[0].kind, OptiStructCardKind::BeginBulk(_)));
        assert_eq!(cards[0].card_name, "BEGIN BULK");
        assert!(cards[0].fields.is_empty());
        assert!(matches!(cards[2].kind, OptiStructCardKind::EndData(_)));
        assert!(cards[2].fields.is_empty());
    }

    #[test]
    fn parses_begin_bulk_when_name_is_split_by_short_fields() {
        let content = "BEGIN BULK2\nGRID    1       0       0.0     0.0     0.0\nENDDATA\n";
        let items = parse_file_items("file:bulk2", Path::new("bulk2.fem"), content);
        assert!(matches!(&items[0], OptiStructItem::Text(text) if text.text == "BEGIN BULK2"));
    }

    #[test]
    fn keeps_unknown_non_comment_lines_as_text_items() {
        let content = "\
PARAM,POST,-1
GRID    1       0       0.0     0.0     0.0
UNPARSED CARD LINE
* orphan continuation
";
        let items = parse_file_items("file:text", Path::new("text.fem"), content);

        assert!(matches!(&items[0], OptiStructItem::Text(text) if text.text == "PARAM,POST,-1"));
        assert!(matches!(&items[1], OptiStructItem::Card(_)));
        assert!(
            matches!(&items[2], OptiStructItem::Text(text) if text.text == "UNPARSED CARD LINE")
        );
        assert!(
            matches!(&items[3], OptiStructItem::Text(text) if text.text == "* orphan continuation")
        );
    }

    #[test]
    fn summarizes_items_per_file() {
        let content = "\
$ comentario
$HWCOLOR COMP 1 5
GRID    1       0       0.0     0.0     0.0
UNPARSED
CQUAD4  10      5       1       2       3       4
";
        let items = parse_file_items("file:sum", Path::new("sum.fem"), content);
        let summary = summarize_file_items("file:sum", Path::new("sum.fem"), content, &items);

        assert_eq!(summary.total_items, 5);
        assert_eq!(summary.total_non_empty_lines, 5);
        assert_eq!(summary.covered_non_empty_lines, 5);
        assert!(summary.line_coverage_ok);
        assert_eq!(summary.comment_items, 2);
        assert_eq!(summary.hypermesh_comment_items, 1);
        assert_eq!(summary.text_items, 1);
        assert_eq!(summary.card_items, 2);
        assert_eq!(summary.card_counts.len(), 2);
        assert!(
            summary
                .card_counts
                .iter()
                .any(|entry| entry.card_name == "GRID" && entry.count == 1)
        );
        assert!(
            summary
                .card_counts
                .iter()
                .any(|entry| entry.card_name == "CQUAD4" && entry.count == 1)
        );
    }
}
