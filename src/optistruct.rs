use crate::model::{
    Cquad4Card, Ctria3Card, ForceCard, GridCard, IncludeCard, MomentCard, OptiStructCardItem,
    OptiStructCardKind, OptiStructCommentItem, OptiStructItem, OptiStructLocation, UnknownCard,
};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawCardRecord {
    name: String,
    long_format: bool,
    fields: Vec<String>,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RawRecord {
    Comment { line: usize, text: String },
    Card(RawCardRecord),
}

pub fn parse_file_items(file_id: &str, file_path: &Path, content: &str) -> Vec<OptiStructItem> {
    let records = parse_raw_records(content);
    let mut items = Vec::with_capacity(records.len());

    for (index, record) in records.into_iter().enumerate() {
        let ordinal = index + 1;
        match record {
            RawRecord::Comment { line, text } => {
                items.push(OptiStructItem::Comment(OptiStructCommentItem {
                    location: OptiStructLocation {
                        file_id: file_id.to_string(),
                        file_path: file_path.to_path_buf(),
                        start_line: line,
                        end_line: line,
                        ordinal,
                    },
                    text,
                }));
            }
            RawRecord::Card(card) => {
                let kind = parse_card_kind(&card.name, &card.fields);
                items.push(OptiStructItem::Card(OptiStructCardItem {
                    location: OptiStructLocation {
                        file_id: file_id.to_string(),
                        file_path: file_path.to_path_buf(),
                        start_line: card.start_line,
                        end_line: card.end_line,
                        ordinal,
                    },
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
                records.push(RawRecord::Comment {
                    line: line_number,
                    text: comment,
                });
            }
            continue;
        }

        if is_continuation_line(payload) {
            if let Some(card) = current_card.as_mut() {
                extend_card_with_continuation(card, payload, line_number);
            }
            if let Some(comment) = inline_comment {
                records.push(RawRecord::Comment {
                    line: line_number,
                    text: comment,
                });
            }
            continue;
        }

        flush_pending_card(&mut current_card, &mut records);
        current_card = parse_card_start(payload, line_number);

        if let Some(comment) = inline_comment {
            records.push(RawRecord::Comment {
                line: line_number,
                text: comment,
            });
        }
    }

    flush_pending_card(&mut current_card, &mut records);
    records.sort_by_key(|record| match record {
        RawRecord::Card(card) => (card.start_line, 0usize),
        RawRecord::Comment { line, .. } => (*line, 1usize),
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
        _ => OptiStructCardKind::Unknown(UnknownCard {
            card_name: card_name.to_string(),
        }),
    }
}

fn field_string(fields: &[String], index: usize) -> Option<String> {
    fields.get(index).and_then(|value| {
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
    let candidate = fields.join("");
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
    let comment = line[index..].trim().to_string();
    (payload, Some(comment))
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
    let first_line_slots = split_fixed_width_fields(line, 8, Some(10));
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

    let mut fields = if long_format {
        if line.len() > 8 {
            split_fixed_width_fields(&line[8..], 16, None)
        } else {
            Vec::new()
        }
    } else {
        first_line_slots.into_iter().skip(1).collect()
    };
    trim_trailing_empty_fields(&mut fields);

    Some(RawCardRecord {
        name: card_name.to_string(),
        long_format,
        fields,
        start_line: line_number,
        end_line: line_number,
    })
}

fn extend_card_with_continuation(card: &mut RawCardRecord, line: &str, line_number: usize) {
    let payload = line.trim_start().trim_start_matches('*');
    let width = if card.long_format { 16 } else { 8 };
    let mut continuation_fields = split_fixed_width_fields(payload, width, None);
    trim_trailing_empty_fields(&mut continuation_fields);
    card.fields.extend(continuation_fields);
    card.end_line = line_number;
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
    use super::parse_file_items;
    use crate::model::{OptiStructCardKind, OptiStructItem};
    use std::path::Path;

    #[test]
    fn parses_grid_cquad4_ctria3_force_moment_and_include_cards() {
        let content = "\
$ Inicio
GRID    100     0       1.0     2.0     3.0
CQUAD4  200     10      1       2       3       4
CTRIA3  201     10      1       3       4
FORCE   1       100     0       500.0   1.0     0.0     0.0
MOMENT  2       100     0       300.0   0.0     1.0     0.0
INCLUDE \"submodel.fem\"
";
        let items = parse_file_items("file:demo", Path::new("demo.fem"), content);
        let cards = items
            .iter()
            .filter_map(|item| match item {
                OptiStructItem::Card(card) => Some(card),
                OptiStructItem::Comment(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(cards.len(), 6);
        assert!(matches!(cards[0].kind, OptiStructCardKind::Grid(_)));
        assert!(matches!(cards[1].kind, OptiStructCardKind::Cquad4(_)));
        assert!(matches!(cards[2].kind, OptiStructCardKind::Ctria3(_)));
        assert!(matches!(cards[3].kind, OptiStructCardKind::Force(_)));
        assert!(matches!(cards[4].kind, OptiStructCardKind::Moment(_)));
        assert!(matches!(cards[5].kind, OptiStructCardKind::Include(_)));

        if let OptiStructCardKind::Grid(grid) = &cards[0].kind {
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
                assert_eq!(comment.text, "$ inline");
            }
            _ => panic!("expected inline comment in line 2"),
        }

        match &items[3] {
            OptiStructItem::Comment(comment) => {
                assert_eq!(comment.location.start_line, 3);
                assert_eq!(comment.text, "$ Final");
            }
            _ => panic!("expected final comment in line 3"),
        }
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
            })
            .expect("INCLUDE card should exist");

        assert_eq!(include.target, "folder/submodel.inc");
    }
}
