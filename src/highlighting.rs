use crate::model::SolverKind;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Default,
    Comment,
    Operator,
    Keyword(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub text: String,
    pub kind: HighlightKind,
}

#[derive(Debug, Clone)]
pub struct HighlightStyles {
    pub default: RgbColor,
    pub comment: RgbColor,
    pub operator: RgbColor,
    pub keywords: [RgbColor; 8],
}

#[derive(Debug, Clone)]
pub struct SyntaxHighlighter {
    case_ignored: bool,
    comment_prefixes: Vec<String>,
    operator_chars: HashSet<u8>,
    keyword_sets: [HashSet<String>; 8],
    styles: HighlightStyles,
}

impl SyntaxHighlighter {
    pub fn from_notepadpp_xml(xml: &str) -> Result<Self, String> {
        let case_ignored = parse_case_ignored(xml);
        let keyword_lists = parse_keyword_lists(xml)?;
        let styles_map = parse_words_styles(xml)?;

        let comments = keyword_lists
            .get("Comments")
            .map(|value| split_udl_keywords(value))
            .unwrap_or_default();
        let comment_prefixes = derive_comment_prefixes(&comments);

        let operator_chars = keyword_lists
            .get("Operators1")
            .map(|value| {
                split_udl_keywords(value)
                    .into_iter()
                    .filter_map(|token| {
                        let mut chars = token.chars();
                        let first = chars.next()?;
                        if chars.next().is_none() {
                            Some(first as u8)
                        } else {
                            None
                        }
                    })
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        let mut keyword_sets = std::array::from_fn(|_| HashSet::<String>::new());
        for index in 0..8 {
            let key = format!("Keywords{}", index + 1);
            if let Some(raw) = keyword_lists.get(&key) {
                for token in split_udl_keywords(raw) {
                    let normalized = normalize_case(&token, case_ignored);
                    if !normalized.is_empty() {
                        keyword_sets[index].insert(normalized);
                    }
                }
            }
        }

        let default = style_color(&styles_map, "DEFAULT").unwrap_or(RgbColor::new(0, 0, 0));
        let comment = style_color(&styles_map, "LINE COMMENTS")
            .or_else(|| style_color(&styles_map, "COMMENTS"))
            .unwrap_or(RgbColor::new(0, 128, 0));
        let operator = style_color(&styles_map, "OPERATORS").unwrap_or(default);
        let keywords = std::array::from_fn(|index| {
            let style_name = format!("KEYWORDS{}", index + 1);
            style_color(&styles_map, &style_name).unwrap_or(default)
        });

        Ok(Self {
            case_ignored,
            comment_prefixes,
            operator_chars,
            keyword_sets,
            styles: HighlightStyles {
                default,
                comment,
                operator,
                keywords,
            },
        })
    }

    pub fn from_notepadpp_file(path: &Path) -> Result<Self, String> {
        let xml = fs::read_to_string(path)
            .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?;
        Self::from_notepadpp_xml(&xml)
    }

    pub fn styles(&self) -> &HighlightStyles {
        &self.styles
    }

    pub fn highlight_line(&self, line: &str) -> Vec<HighlightSpan> {
        if line.is_empty() {
            return vec![HighlightSpan {
                text: String::new(),
                kind: HighlightKind::Default,
            }];
        }

        let trimmed_start = line.trim_start();
        if self
            .comment_prefixes
            .iter()
            .any(|prefix| !prefix.is_empty() && trimmed_start.starts_with(prefix))
        {
            return vec![HighlightSpan {
                text: line.to_string(),
                kind: HighlightKind::Comment,
            }];
        }

        let bytes = line.as_bytes();
        let mut spans = Vec::<HighlightSpan>::new();
        let mut start = 0usize;

        while start < bytes.len() {
            let byte = bytes[start];
            if byte.is_ascii_whitespace() {
                let mut end = start + 1;
                while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                    end += 1;
                }
                push_span(&mut spans, &line[start..end], HighlightKind::Default);
                start = end;
                continue;
            }

            if self.operator_chars.contains(&byte) {
                push_span(&mut spans, &line[start..start + 1], HighlightKind::Operator);
                start += 1;
                continue;
            }

            let mut end = start + 1;
            while end < bytes.len()
                && !bytes[end].is_ascii_whitespace()
                && !self.operator_chars.contains(&bytes[end])
            {
                end += 1;
            }
            let token = &line[start..end];
            let token_kind = self.classify_token(token);
            push_span(&mut spans, token, token_kind);
            start = end;
        }

        spans
    }

    fn classify_token(&self, token: &str) -> HighlightKind {
        let normalized = normalize_case(token, self.case_ignored);
        for (index, keywords) in self.keyword_sets.iter().enumerate() {
            if keywords.contains(&normalized) {
                return HighlightKind::Keyword((index + 1) as u8);
            }
        }
        HighlightKind::Default
    }
}

pub fn highlighting_file_for_solver(solver: &SolverKind) -> Option<&'static str> {
    match solver {
        SolverKind::OptiStruct => Some("NASTRAN_mod.xml"),
        SolverKind::Nastran => Some("NASTRAN.xml"),
        SolverKind::Abaqus => Some("ABAQUS_6.13.xml"),
        SolverKind::Unknown => None,
    }
}

pub fn embedded_highlighting_xml(file_name: &str) -> Option<&'static str> {
    match file_name {
        "ABAQUS_6.13.xml" => Some(include_str!("../highlighting/ABAQUS_6.13.xml")),
        "ABAQUS.xml" => Some(include_str!("../highlighting/ABAQUS.xml")),
        "NASTRAN_mod.xml" => Some(include_str!("../highlighting/NASTRAN_mod.xml")),
        "NASTRAN.xml" => Some(include_str!("../highlighting/NASTRAN.xml")),
        "OptiStruct.xml" => Some(include_str!("../highlighting/OptiStruct.xml")),
        _ => None,
    }
}

fn push_span(spans: &mut Vec<HighlightSpan>, text: &str, kind: HighlightKind) {
    if let Some(last) = spans.last_mut()
        && last.kind == kind
    {
        last.text.push_str(text);
        return;
    }
    spans.push(HighlightSpan {
        text: text.to_string(),
        kind,
    });
}

fn parse_case_ignored(xml: &str) -> bool {
    if let Some(attributes) = parse_first_tag_attributes(xml, "Global")
        && let Some(value) = attributes.get("caseIgnored")
    {
        return value.eq_ignore_ascii_case("yes");
    }
    false
}

fn parse_keyword_lists(xml: &str) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::<String, String>::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = xml[cursor..].find("<Keywords") {
        let start = cursor + start_rel;
        let open_end = xml[start..]
            .find('>')
            .ok_or_else(|| "Etiqueta <Keywords> mal formada.".to_string())?
            + start;
        let open_tag = &xml[start..=open_end];
        let attributes = parse_tag_attributes(open_tag);
        let Some(name) = attributes.get("name").cloned() else {
            cursor = open_end + 1;
            continue;
        };
        let close_start = xml[open_end + 1..]
            .find("</Keywords>")
            .ok_or_else(|| "Falta cierre </Keywords> en XML de highlighting.".to_string())?
            + open_end
            + 1;
        let content = &xml[open_end + 1..close_start];
        result.insert(name, decode_xml_entities(content));
        cursor = close_start + "</Keywords>".len();
    }
    Ok(result)
}

fn parse_words_styles(xml: &str) -> Result<HashMap<String, HashMap<String, String>>, String> {
    let mut result = HashMap::<String, HashMap<String, String>>::new();
    let mut cursor = 0usize;
    while let Some(start_rel) = xml[cursor..].find("<WordsStyle") {
        let start = cursor + start_rel;
        let end = xml[start..]
            .find('>')
            .ok_or_else(|| "Etiqueta <WordsStyle> mal formada.".to_string())?
            + start;
        let tag = &xml[start..=end];
        let attributes = parse_tag_attributes(tag);
        if let Some(name) = attributes.get("name").cloned() {
            result.insert(name, attributes);
        }
        cursor = end + 1;
    }
    Ok(result)
}

fn parse_first_tag_attributes(xml: &str, tag_name: &str) -> Option<HashMap<String, String>> {
    let marker = format!("<{tag_name}");
    let start = xml.find(&marker)?;
    let end = xml[start..].find('>')? + start;
    Some(parse_tag_attributes(&xml[start..=end]))
}

fn parse_tag_attributes(tag: &str) -> HashMap<String, String> {
    let mut attributes = HashMap::<String, String>::new();
    let bytes = tag.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() && bytes[i] != b'<' {
        i += 1;
    }
    while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' && bytes[i] != b'>' {
        i += 1;
    }

    while i < bytes.len() {
        while i < bytes.len()
            && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/' || bytes[i] == b'>')
        {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'>' {
            break;
        }

        let key_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'>'
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key = &tag[key_start..i];

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        if bytes[i] == b'"' {
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i <= bytes.len() {
                let value = decode_xml_entities(&tag[value_start..i]);
                attributes.insert(key.to_string(), value);
            }
            if i < bytes.len() && bytes[i] == b'"' {
                i += 1;
            }
        } else {
            let value_start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'>'
                && bytes[i] != b'/'
            {
                i += 1;
            }
            let value = decode_xml_entities(&tag[value_start..i]);
            attributes.insert(key.to_string(), value);
        }
    }

    attributes
}

fn style_color(
    styles: &HashMap<String, HashMap<String, String>>,
    style_name: &str,
) -> Option<RgbColor> {
    let style = styles.get(style_name)?;
    let fg = style.get("fgColor")?;
    parse_color_hex(fg)
}

fn parse_color_hex(value: &str) -> Option<RgbColor> {
    if value.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&value[0..2], 16).ok()?;
    let g = u8::from_str_radix(&value[2..4], 16).ok()?;
    let b = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(RgbColor::new(r, g, b))
}

fn split_udl_keywords(raw: &str) -> Vec<String> {
    let normalized = decode_xml_entities(raw)
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ");
    let mut tokens = Vec::<String>::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in normalized.chars() {
        if in_quote {
            if ch == '\'' {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                }
                current.clear();
                in_quote = false;
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch == '\'' {
            if !current.trim().is_empty() {
                tokens.push(current.trim().to_string());
            }
            current.clear();
            in_quote = true;
            continue;
        }

        if ch.is_whitespace() {
            if !current.trim().is_empty() {
                tokens.push(current.trim().to_string());
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

fn derive_comment_prefixes(tokens: &[String]) -> Vec<String> {
    let mut prefixes = HashSet::<String>::new();
    for token in tokens {
        if token.len() < 3 || !token.starts_with("00") {
            continue;
        }
        let marker = token[2..].trim().to_string();
        if marker.is_empty() {
            continue;
        }
        prefixes.insert(marker.clone());
        if marker.starts_with('$') && marker.len() > 1 {
            prefixes.insert("$".to_string());
        }
        if marker.starts_with('#') && marker.len() > 1 {
            prefixes.insert("#".to_string());
        }
    }

    let mut ordered = prefixes.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    ordered
}

fn normalize_case(value: &str, case_ignored: bool) -> String {
    if case_ignored {
        value.to_ascii_uppercase()
    } else {
        value.to_string()
    }
}

fn decode_xml_entities(value: &str) -> String {
    let mut out = String::new();
    let chars = value.chars().collect::<Vec<_>>();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] != '&' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < chars.len() && chars[j] != ';' && j - i <= 12 {
            j += 1;
        }
        if j >= chars.len() || chars[j] != ';' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        let entity = chars[i + 1..j].iter().collect::<String>();
        match entity.as_str() {
            "apos" => out.push('\''),
            "quot" => out.push('"'),
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                if let Ok(code) = u32::from_str_radix(&entity[2..], 16)
                    && let Some(ch) = char::from_u32(code)
                {
                    out.push(ch);
                }
            }
            _ if entity.starts_with('#') => {
                if let Ok(code) = entity[1..].parse::<u32>()
                    && let Some(ch) = char::from_u32(code)
                {
                    out.push(ch);
                }
            }
            _ => {
                out.push('&');
                out.push_str(&entity);
                out.push(';');
            }
        }
        i = j + 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_notepadpp_theme_and_highlights_keywords() {
        let xml = r#"
<NotepadPlus>
  <UserLang name="X" ext="fem" udlVersion="2.1">
    <Settings>
      <Global caseIgnored="yes" />
    </Settings>
    <KeywordLists>
      <Keywords name="Comments">00$$ 00$?</Keywords>
      <Keywords name="Operators1">, =</Keywords>
      <Keywords name="Keywords1">GRID INCLUDE</Keywords>
    </KeywordLists>
    <Styles>
      <WordsStyle name="DEFAULT" fgColor="000000" />
      <WordsStyle name="LINE COMMENTS" fgColor="008000" />
      <WordsStyle name="OPERATORS" fgColor="FF0000" />
      <WordsStyle name="KEYWORDS1" fgColor="0000FF" />
    </Styles>
  </UserLang>
</NotepadPlus>
"#;

        let highlighter = SyntaxHighlighter::from_notepadpp_xml(xml)
            .expect("parser should read test XML correctly");
        let line = "grid, 1";
        let spans = highlighter.highlight_line(line);

        assert!(
            spans
                .iter()
                .any(|span| span.kind == HighlightKind::Keyword(1) && span.text == "grid")
        );
        assert!(
            spans
                .iter()
                .any(|span| span.kind == HighlightKind::Operator && span.text == ",")
        );
    }

    #[test]
    fn detects_comment_line_from_nastran_mod_comment_pattern() {
        let xml = r#"
<NotepadPlus>
  <UserLang name="X" ext="fem" udlVersion="2.1">
    <Settings><Global caseIgnored="no" /></Settings>
    <KeywordLists>
      <Keywords name="Comments">00$$ 00$? 00$#</Keywords>
    </KeywordLists>
    <Styles>
      <WordsStyle name="DEFAULT" fgColor="000000" />
      <WordsStyle name="LINE COMMENTS" fgColor="008000" />
    </Styles>
  </UserLang>
</NotepadPlus>
"#;
        let highlighter = SyntaxHighlighter::from_notepadpp_xml(xml)
            .expect("parser should read comment patterns");
        let spans = highlighter.highlight_line("$HMNAME PROP 1\"Plate\"");

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].kind, HighlightKind::Comment);
    }

    #[test]
    fn abaqus_double_asterisk_comment_does_not_mark_keywords_as_comment() {
        let xml = r#"
<NotepadPlus>
  <UserLang name="X" ext="inp" udlVersion="2.1">
    <Settings><Global caseIgnored="yes" /></Settings>
    <KeywordLists>
      <Keywords name="Comments">00**</Keywords>
      <Keywords name="Keywords1">'*NODE'</Keywords>
    </KeywordLists>
    <Styles>
      <WordsStyle name="DEFAULT" fgColor="000000" />
      <WordsStyle name="LINE COMMENTS" fgColor="008000" />
      <WordsStyle name="KEYWORDS1" fgColor="0000FF" />
    </Styles>
  </UserLang>
</NotepadPlus>
"#;
        let highlighter = SyntaxHighlighter::from_notepadpp_xml(xml)
            .expect("parser should read Abaqus comment marker");
        let keyword_line = "*NODE";
        let comment_line = "** heading";

        let keyword_spans = highlighter.highlight_line(keyword_line);
        let comment_spans = highlighter.highlight_line(comment_line);

        assert!(
            keyword_spans
                .iter()
                .any(|span| span.kind == HighlightKind::Keyword(1))
        );
        assert_eq!(comment_spans.len(), 1);
        assert_eq!(comment_spans[0].kind, HighlightKind::Comment);
    }

    #[test]
    fn solver_to_highlighting_file_mapping_is_expected() {
        assert_eq!(
            highlighting_file_for_solver(&SolverKind::OptiStruct),
            Some("NASTRAN_mod.xml")
        );
        assert_eq!(
            highlighting_file_for_solver(&SolverKind::Nastran),
            Some("NASTRAN.xml")
        );
        assert_eq!(
            highlighting_file_for_solver(&SolverKind::Abaqus),
            Some("ABAQUS_6.13.xml")
        );
        assert_eq!(highlighting_file_for_solver(&SolverKind::Unknown), None);
    }

    #[test]
    fn embedded_solver_highlighting_files_are_parseable() {
        for solver in [SolverKind::OptiStruct, SolverKind::Nastran, SolverKind::Abaqus] {
            let file_name = highlighting_file_for_solver(&solver).expect("solver file mapping");
            let xml = embedded_highlighting_xml(file_name).expect("embedded highlighting XML");
            SyntaxHighlighter::from_notepadpp_xml(xml)
                .expect("embedded highlighting XML should parse");
        }
    }
}
