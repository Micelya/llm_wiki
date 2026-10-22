//! Links inside the text of a document's page to the pages of the
//! entities it names (P-102): without the model, and only where linking
//! wrong is not possible.
//!
//! The page of a document is written by the wiki; the app adds, besides
//! its block of entities at the end (D-198), a link on the first place
//! the text writes one of the document's names **exactly as the document
//! writes it** (D-221). A name is linked only when:
//!
//! - its entity has a page, and the name has two words or more and is not
//!   generic (a name of one word is often an ordinary word);
//! - no other entity of the same document is written the same way, or
//!   with a name that contains it (an organization written without its
//!   legal form when another of the document has it: which one it is,
//!   the text does not say);
//! - the place is running text: not the header, a heading, a code span or
//!   block, nor inside another link; and the name is whole there: what
//!   comes next is not a letter, a digit or another capitalized word
//!   (`Ejemplo Uno` in `Ejemplo Uno LLC` is not linked).
//!
//! Every link the app adds carries a mark (`APP_LINK_MARK`), so the app
//! can take them all away and write them again, and so they never count
//! as links a person or the wiki made.

/// What follows each link the app adds to a document's page.
pub const APP_LINK_MARK: &str = "<!--micelya-->";

/// The page without the links the app added: each one back to its text.
#[cfg(test)]
pub fn strip_app_links(text: &str) -> String {
    let close = format!("]]{APP_LINK_MARK}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(end) = rest.find(&close) {
        let before = &rest[..end];
        match before.rfind("[[") {
            Some(start) => {
                let inner = &before[start + 2..];
                let shown = inner.split_once('|').map(|(_, s)| s).unwrap_or(inner);
                out.push_str(&before[..start]);
                out.push_str(shown);
            }
            None => out.push_str(before),
        }
        rest = &rest[end + close.len()..];
    }
    out.push_str(rest);
    out
}

/// A name of a document to link: how the document writes it, and the
/// page (its file name without `.md`) of its entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkForm {
    pub written: String,
    pub stem: String,
    pub entity: String,
}

/// The names that can be linked without doubt, from every `(entity,
/// written, has a page)` of one document's mentions: see the module.
pub fn safe_forms(mentions: &[(String, String, Option<String>)], is_generic: impl Fn(&str) -> bool) -> Vec<LinkForm> {
    let key = |w: &str| w.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let mut out: Vec<LinkForm> = Vec::new();
    for (entity, written, stem) in mentions {
        let Some(stem) = stem else { continue };
        let written = written.split_whitespace().collect::<Vec<_>>().join(" ");
        let words = written.split(' ').filter(|w| w.chars().any(char::is_alphanumeric)).count();
        if words < 2 || is_generic(&key(&written)) || written.contains(['[', ']', '|', '\n']) {
            continue;
        }
        // Another entity of the document written the same way, or with a
        // name that contains this one: not linked.
        let clash = mentions.iter().any(|(other, other_written, _)| {
            other != entity && {
                let (a, b) = (key(other_written), key(&written));
                a == b || a.contains(&b) || b.contains(&a)
            }
        });
        if clash || out.iter().any(|f| f.entity == *entity && key(&f.written) == key(&written)) {
            continue;
        }
        out.push(LinkForm { written, stem: stem.clone(), entity: entity.clone() });
    }
    // The longest names first: a longer name is linked before a shorter
    // one inside it.
    out.sort_by(|a, b| b.written.len().cmp(&a.written.len()).then_with(|| a.written.cmp(&b.written)));
    out
}

/// Byte ranges of the text where nothing is linked: the header, headings,
/// code, existing links, and the notes of the page (`[^1]: «…»`, D-441):
/// a quote stays as the document writes it.
fn closed_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut closed = Vec::new();
    let mut at = 0;
    let mut in_header = text.starts_with("---\n");
    let mut in_fence = false;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let start = at;
        at += line.len();
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if in_header {
            closed.push((start, at));
            if index > 0 && trimmed == "---" {
                in_header = false;
            }
            continue;
        }
        if trimmed.trim_start().starts_with("```") {
            in_fence = !in_fence;
            closed.push((start, at));
            continue;
        }
        let opening = trimmed.trim_start();
        if in_fence || opening.starts_with('#') || opening.starts_with("<!--") || opening.starts_with("[^") {
            closed.push((start, at));
            continue;
        }
        // Inline code, wiki links and markdown links on the line.
        let bytes = trimmed.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let rest = &trimmed[i..];
            let span = if rest.starts_with('`') {
                rest[1..].find('`').map(|e| e + 2)
            } else if rest.starts_with("[[") {
                rest.find("]]").map(|e| e + 2)
            } else if rest.starts_with('[') {
                rest.find(')').filter(|e| rest[..*e].contains("](")).map(|e| e + 1)
            } else if rest.starts_with("http://") || rest.starts_with("https://") {
                Some(rest.find(char::is_whitespace).unwrap_or(rest.len()))
            } else {
                None
            };
            match span {
                Some(length) => {
                    closed.push((start + i, start + i + length));
                    i += length;
                }
                None => i += rest.chars().next().map(char::len_utf8).unwrap_or(1),
            }
        }
    }
    closed
}

/// Whether the name found at `start..end` is whole: not inside a word,
/// and not followed by another capitalized word (a longer name).
fn whole_at(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    if before.is_some_and(|c| c.is_alphanumeric()) {
        return false;
    }
    let mut after = text[end..].chars();
    match after.next() {
        Some(c) if c.is_alphanumeric() => false,
        Some(' ') => !after.next().is_some_and(|c| c.is_uppercase() || c.is_ascii_digit()),
        _ => true,
    }
}

/// The page (already without the app's links) with a link on the first
/// whole place each name is written, one per entity.
pub fn with_links(text: &str, forms: &[LinkForm]) -> String {
    if forms.is_empty() {
        return text.to_string();
    }
    let mut closed = closed_ranges(text);
    let mut chosen: Vec<(usize, usize, &LinkForm)> = Vec::new();
    let mut linked: Vec<&str> = Vec::new();
    for form in forms {
        if linked.contains(&form.entity.as_str()) {
            continue;
        }
        let mut from = 0;
        while let Some(found) = text[from..].find(&form.written) {
            let (start, end) = (from + found, from + found + form.written.len());
            from = start + text[start..].chars().next().map(char::len_utf8).unwrap_or(1);
            let free = !closed.iter().any(|(a, b)| start < *b && end > *a);
            if free && whole_at(text, start, end) {
                chosen.push((start, end, form));
                closed.push((start, end));
                linked.push(&form.entity);
                break;
            }
        }
    }
    chosen.sort_by_key(|(start, _, _)| *start);
    let mut out = String::with_capacity(text.len() + chosen.len() * 40);
    let mut at = 0;
    for (start, end, form) in chosen {
        out.push_str(&text[at..start]);
        out.push_str(&format!("[[{}|{}]]{APP_LINK_MARK}", form.stem, &text[start..end]));
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(written: &str, stem: &str, entity: &str) -> LinkForm {
        LinkForm { written: written.into(), stem: stem.into(), entity: entity.into() }
    }

    const PAGE: &str = "---\ntype: source\ntitle: Persona Dos y Ejemplo Uno S.A.\n---\n\n# Carta de Persona Dos\n\n\
Persona Dos le escribe a Ejemplo Uno S.A. por la deuda. Persona Dos firma.\n\nVer [[otra|Ejemplo Uno S.A.]] y `Persona Dos`.\n";

    #[test]
    fn the_first_whole_place_in_running_text_is_linked_once() {
        let forms = vec![form("Ejemplo Uno S.A.", "ejemplo-uno-s-a-11", "e1"), form("Persona Dos", "persona-dos-22", "p2")];
        let linked = with_links(PAGE, &forms);
        // Not in the header, the heading, another link or a code span.
        assert!(linked.starts_with("---\ntype: source\ntitle: Persona Dos y Ejemplo Uno S.A.\n---\n\n# Carta de Persona Dos\n"));
        assert!(linked.contains("[[persona-dos-22|Persona Dos]]<!--micelya--> le escribe a [[ejemplo-uno-s-a-11|Ejemplo Uno S.A.]]<!--micelya--> por"));
        assert!(linked.contains("deuda. Persona Dos firma."), "only the first place");
        assert!(linked.contains("Ver [[otra|Ejemplo Uno S.A.]] y `Persona Dos`."));
        // Taken away, the page is as it was; written again, the same.
        assert_eq!(strip_app_links(&linked), PAGE);
        assert_eq!(with_links(&strip_app_links(&linked), &forms), linked);
    }

    #[test]
    fn a_name_inside_a_longer_name_is_not_linked() {
        let text = "Ejemplo Uno LLC le escribe a Ejemplo Uno, y Ejemplo Unos no.\n";
        let linked = with_links(text, &[form("Ejemplo Uno", "ejemplo-uno-1", "e1")]);
        assert_eq!(linked, "Ejemplo Uno LLC le escribe a [[ejemplo-uno-1|Ejemplo Uno]]<!--micelya-->, y Ejemplo Unos no.\n");
    }

    #[test]
    fn only_names_that_cannot_be_another_entity_of_the_document_are_used() {
        let mentions = vec![
            ("e1".to_string(), "Ejemplo Uno S.A.".to_string(), Some("ejemplo-uno-s-a-1".to_string())),
            ("e2".to_string(), "Ejemplo Uno".to_string(), Some("ejemplo-uno-2".to_string())),
            ("p1".to_string(), "Persona  Dos".to_string(), Some("persona-dos-3".to_string())),
            ("p1".to_string(), "Persona Dos".to_string(), Some("persona-dos-3".to_string())),
            ("p4".to_string(), "Cuatro".to_string(), Some("cuatro-4".to_string())),
            ("p5".to_string(), "Persona Cinco".to_string(), None),
            ("g".to_string(), "El Cliente".to_string(), Some("el-cliente-6".to_string())),
        ];
        let forms = safe_forms(&mentions, |name| name == "el cliente");
        // One contains the other: neither. One word, no page, generic: no.
        assert_eq!(forms, vec![form("Persona Dos", "persona-dos-3", "p1")]);
    }
}
