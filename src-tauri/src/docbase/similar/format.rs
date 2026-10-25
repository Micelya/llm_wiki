//! What a file format adds to a document's text that is not the
//! document, taken out before comparing it with another (D-521).
//!
//! The same document read from a Word file and from its PDF must compare
//! as the same text. Each rule here is about what one way of reading
//! writes, and does nothing to a text that does not have it:
//!
//! - **The marks of Markdown.** The text of a Word file (and of the other
//!   office formats) is kept as Markdown: bold and italics (`**`, `*`),
//!   headings (`#`), list bullets (`- `), links (`[text](address)`, which
//!   writes an address twice when the text is the address itself), code
//!   spans, escapes (`\.`), table bars and footnote marks. None of them
//!   is text of the document. The words do not change; the tokens a
//!   difference is told apart by do (a sentence that ends in bold ends a
//!   sentence), and so do words split by a mark inside them.
//! - **Carriage returns**, which the PDF reader writes at the end of each
//!   line.
//! - **The hyphen at the end of a line** that the PDF reader writes as the
//!   character 0x02, joining the two lines: the two parts are one word
//!   when that word is elsewhere in the same text ("contra-to" where the
//!   text also says "contrato"); otherwise it is a hyphen of a compound
//!   word ("real-world"), as the Word file writes it.
//! - **The page number lines** of a PDF: at the top or the bottom of a
//!   page, a line that is only the number of the page ("3", "- 3 -",
//!   "Página 3 de 10", "3/10"), when at least two pages have one that
//!   follows the pages the same way. A Word file has no pages, but one
//!   made from a PDF may keep its page numbers as paragraphs of their
//!   own: they go when at least three of them grow in order.
//! - **A link** is its text: the address it points to is not written,
//!   the same link repeated right after itself is written once, and an
//!   address is never glued to the word before or after it.
//!
//! The header a signing service repeats on every page is left out by the
//! catalog (`dropLines` of its signal), with the other lines that are not
//! content. The numbering of paragraphs ("4.2.") stays in the text, but
//! counts as wording and not as data (`text::Raw::numbering`): the Word
//! file may not write its automatic numbering, and its PDF does.
//!
//! The page headings (`## Page N`), the form fields and the recognition
//! markers stay as they are, since the comparison reads them.

use std::collections::HashSet;

use crate::commands::extract_images::FORM_FIELDS_HEADING;
use crate::source_volume::catalog::{is_page_heading, RECOGNIZED_MARKER_START};

use super::text;

/// The character the PDF reader writes for a hyphen at the end of a line.
const LINE_END_HYPHEN: char = '\u{2}';
/// Lines at each edge of a page where its number may be.
const PAGE_EDGE_LINES: usize = 2;
/// Pages that must have a number line for those lines to be numbers.
const NUMBERED_PAGES_MIN: usize = 2;
/// Paragraphs that must be page numbers, growing, in a text without pages.
const PARAGRAPH_NUMBERS_MIN: usize = 3;
/// Digits of a page number.
const PAGE_NUMBER_DIGITS: usize = 4;
/// Words that may come with a page number, without accents.
const PAGE_WORDS: &[&str] = &["pagina", "pag", "page", "p", "hoja"];
/// Words between the page and the number of pages ("3 de 10").
const OF_WORDS: &[&str] = &["de", "of"];

/// The text of a document as it is compared: without what its format
/// adds. Every reading of the text for the comparison (the signature, the
/// pair, the template) goes through here, so their tokens are the same.
pub fn comparable(body: &str) -> String {
    let body = join_line_end_hyphens(&body.replace('\r', ""));
    let mut lines: Vec<String> = body
        .split('\n')
        .map(|line| if keeps_as_is(line.trim()) { line.to_string() } else { without_markdown(line) })
        .collect();
    let numbers = if lines.iter().any(|line| is_page_heading(line.trim())) {
        page_number_lines(&lines)
    } else {
        paragraph_page_numbers(&lines)
    };
    for index in numbers {
        lines[index].clear();
    }
    lines.join("\n")
}

fn keeps_as_is(trimmed: &str) -> bool {
    is_page_heading(trimmed) || trimmed == FORM_FIELDS_HEADING || trimmed.starts_with(RECOGNIZED_MARKER_START)
}

// --- The hyphen at the end of a line ---

fn join_line_end_hyphens(body: &str) -> String {
    if !body.contains(LINE_END_HYPHEN) {
        return body.to_string();
    }
    // The words of the text, with the mark as a separator.
    let words: HashSet<String> = text::normalized(&body.replace(LINE_END_HYPHEN, " "))
        .split(' ')
        .map(str::to_string)
        .collect();
    let chars: Vec<char> = body.chars().collect();
    let mut out = String::with_capacity(body.len());
    for (index, c) in chars.iter().enumerate() {
        if *c != LINE_END_HYPHEN {
            out.push(*c);
            continue;
        }
        let left: String = chars[..index].iter().rev().take_while(|c| c.is_alphanumeric()).collect::<Vec<_>>().into_iter().rev().collect();
        let right: String = chars[index + 1..].iter().take_while(|c| c.is_alphanumeric()).collect();
        let joined = text::normalized(&format!("{left}{right}"));
        if left.is_empty() || right.is_empty() || !words.contains(&joined) {
            out.push('-');
        }
    }
    out
}

// --- The marks of Markdown ---

/// A line without the marks of Markdown. A line that is only a rule
/// (`---`) or the separator of a table is left empty.
fn without_markdown(line: &str) -> String {
    let mut rest = line.trim_start();
    if is_rule(rest) {
        return String::new();
    }
    // Block marks at the start: quotes, headings, bullets, a footnote's
    // label. An ordered list's number is text of the document.
    loop {
        let before = rest;
        if let Some(after) = rest.strip_prefix('>') {
            rest = after.trim_start();
        }
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && rest[hashes..].starts_with(' ') {
            rest = rest[hashes..].trim_start();
        }
        for bullet in ["- ", "* ", "+ "] {
            if let Some(after) = rest.strip_prefix(bullet) {
                rest = after.trim_start();
            }
        }
        if rest.starts_with("[^") {
            if let Some(end) = rest.find("]:") {
                rest = rest[end + 2..].trim_start();
            }
        }
        if rest == before {
            break;
        }
    }
    let mut out = String::with_capacity(rest.len());
    inline(rest, &mut out);
    if line.trim_start().starts_with('|') {
        // The bars of a table separate its cells.
        out = out.replace('|', " ");
    }
    out
}

fn is_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.len() >= 3 {
        for mark in ['-', '*', '_'] {
            if trimmed.chars().all(|c| c == mark || c == ' ') && trimmed.chars().filter(|c| *c == mark).count() >= 3 {
                return true;
            }
        }
    }
    // A table's separator row: bars, dashes, colons.
    trimmed.starts_with('|') && trimmed.contains('-') && trimmed.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Writes `text` without its inline marks.
fn inline(text: &str, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1).is_some_and(|n| n.is_ascii_punctuation()) => {
                out.push(chars[i + 1]);
                i += 2;
            }
            '*' | '`' => i += 1,
            '~' if chars.get(i + 1) == Some(&'~') => i += 2,
            '!' if chars.get(i + 1) == Some(&'[') => i += 1,
            '[' => match link(&chars, i) {
                Some(Link::Footnote { end }) => i = end,
                Some(Link::Labeled { label, end }) => {
                    let written: String = chars[i..end].iter().collect();
                    let label: String = chars[label].iter().collect();
                    // An address is never part of a word around it.
                    let address = is_address(&label);
                    if address && out.chars().last().is_some_and(char::is_alphanumeric) {
                        out.push(' ');
                    }
                    inline(&label, out);
                    i = end;
                    // The same link written again right after itself (a
                    // field's code and its result) is written once.
                    let mut again: String = chars[i..(i + written.chars().count()).min(chars.len())].iter().collect();
                    while again == written {
                        i += written.chars().count();
                        again = chars[i..(i + written.chars().count()).min(chars.len())].iter().collect();
                    }
                    if address && chars.get(i).is_some_and(|c| c.is_alphanumeric()) {
                        out.push(' ');
                    }
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            },
            '<' => match autolink_or_break(&chars, i) {
                Some((inner, end)) => {
                    let inner: String = chars[inner].iter().collect();
                    out.push_str(if inner.is_empty() { " " } else { &inner });
                    i = end;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            },
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
}

enum Link {
    /// A footnote's mark (`[^1]`), up to `end`.
    Footnote { end: usize },
    /// `[label](address)` up to `end`.
    Labeled { label: std::ops::Range<usize>, end: usize },
}

fn link(chars: &[char], open: usize) -> Option<Link> {
    let close = open + 1 + chars[open + 1..].iter().position(|c| *c == ']')?;
    if chars.get(open + 1) == Some(&'^') {
        return Some(Link::Footnote { end: close + 1 });
    }
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = close + 2 + chars[close + 2..].iter().position(|c| *c == ')')?;
    Some(Link::Labeled { label: open + 1..close, end: end + 1 })
}

fn is_address(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    ["http://", "https://", "www.", "mailto:"].iter().any(|start| lower.starts_with(start))
}

/// `<https://…>` writes its address; `<br>` is a space.
fn autolink_or_break(chars: &[char], open: usize) -> Option<(std::ops::Range<usize>, usize)> {
    let close = open + 1 + chars[open + 1..].iter().position(|c| *c == '>')?;
    let inner: String = chars[open + 1..close].iter().collect();
    let lower = inner.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") {
        return Some((open + 1..close, close + 1));
    }
    if matches!(lower.trim_end_matches('/').trim(), "br") {
        return Some((open..open, close + 1));
    }
    None
}

// --- The page number lines ---

/// The lines that are a page's number, when there are pages: at an edge
/// of its page, and following the pages as most of them do (a lone "15"
/// at the bottom of the second page is not its number when the others
/// are "1", "2", "3").
fn page_number_lines(lines: &[String]) -> Vec<usize> {
    // The non-empty lines of each page, with the page's number.
    let mut pages: Vec<(i64, Vec<usize>)> = Vec::new();
    let mut on_page = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_page_heading(trimmed) {
            let number = trimmed.trim_start_matches("## Page ").parse::<i64>().unwrap_or_default();
            pages.push((number, Vec::new()));
            on_page = true;
        } else if trimmed == FORM_FIELDS_HEADING {
            // The form fields are not on the page.
            on_page = false;
        } else if on_page && !trimmed.is_empty() && !trimmed.starts_with(RECOGNIZED_MARKER_START) {
            if let Some((_, page)) = pages.last_mut() {
                page.push(index);
            }
        }
    }
    if pages.len() < NUMBERED_PAGES_MIN {
        return Vec::new();
    }
    // Each candidate, by how far its number is from the page's.
    let mut by_offset: std::collections::BTreeMap<i64, Vec<(usize, i64)>> = std::collections::BTreeMap::new();
    for (page, indices) in &pages {
        let edges = indices.iter().take(PAGE_EDGE_LINES).chain(indices.iter().rev().take(PAGE_EDGE_LINES));
        let mut seen = Vec::new();
        for index in edges.copied() {
            if seen.contains(&index) {
                continue;
            }
            seen.push(index);
            if let Some(number) = page_number(&lines[index]) {
                by_offset.entry(i64::from(number) - page).or_default().push((index, *page));
            }
        }
    }
    let pages_of = |found: &Vec<(usize, i64)>| {
        let mut pages: Vec<i64> = found.iter().map(|(_, page)| *page).collect();
        pages.dedup();
        pages.len()
    };
    let best = by_offset
        .iter()
        .max_by(|(oa, a), (ob, b)| pages_of(a).cmp(&pages_of(b)).then_with(|| ob.abs().cmp(&oa.abs())));
    match best {
        Some((_, found)) if pages_of(found) >= NUMBERED_PAGES_MIN => found.iter().map(|(index, _)| *index).collect(),
        _ => Vec::new(),
    }
}

/// In a text without pages, the paragraphs that are only a page number:
/// a Word file made from a PDF keeps them as paragraphs of their own.
/// They go when at least `PARAGRAPH_NUMBERS_MIN` of them follow one
/// another upwards ("2", "3", "5"…), the most that do.
fn paragraph_page_numbers(lines: &[String]) -> Vec<usize> {
    let blank = |index: usize| lines.get(index).is_none_or(|line| line.trim().is_empty());
    let candidates: Vec<(usize, u32)> = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| (*index == 0 || blank(index - 1)) && blank(index + 1))
        .filter_map(|(index, line)| page_number(line).map(|number| (index, number)))
        .collect();
    if candidates.len() < PARAGRAPH_NUMBERS_MIN {
        return Vec::new();
    }
    // The longest run of numbers that grows, in the order of the text.
    let mut length = vec![1usize; candidates.len()];
    let mut previous: Vec<Option<usize>> = vec![None; candidates.len()];
    for j in 0..candidates.len() {
        for i in 0..j {
            if candidates[i].1 < candidates[j].1 && length[i] + 1 > length[j] {
                length[j] = length[i] + 1;
                previous[j] = Some(i);
            }
        }
    }
    let Some((mut last, longest)) = length.iter().copied().enumerate().max_by_key(|(_, n)| *n) else {
        return Vec::new();
    };
    if longest < PARAGRAPH_NUMBERS_MIN {
        return Vec::new();
    }
    let mut found = vec![candidates[last].0];
    while let Some(before) = previous[last] {
        found.push(candidates[before].0);
        last = before;
    }
    found
}

/// The number of a line that is only a page number: "3", "- 3 -",
/// "Página 3", "Pág. 3 de 10", "3/10", "Page 3 of 10".
pub(crate) fn page_number(line: &str) -> Option<u32> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || !trimmed.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || matches!(c, '-' | '–' | '—' | '.' | '/' | '|' | '·' | '(' | ')' | ':'))
    {
        return None;
    }
    let words = text::normalized(trimmed);
    let words: Vec<&str> = words.split(' ').collect();
    let number = |word: &str| -> Option<u32> {
        if word.len() > PAGE_NUMBER_DIGITS || !word.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        word.parse::<u32>().ok().filter(|n| *n > 0)
    };
    let rest = match words.as_slice() {
        [first, rest @ ..] if PAGE_WORDS.contains(first) => rest,
        all => all,
    };
    let of_total = |n: &str, total: &str| number(n).zip(number(total)).filter(|(n, total)| n <= total).map(|(n, _)| n);
    match rest {
        [n] => number(n),
        [n, of, total] if OF_WORDS.contains(of) => of_total(n, total),
        [n, total] if trimmed.contains('/') => of_total(n, total),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marks_of_markdown_are_not_text() {
        let body = "# 1. Objeto\n\n**El Locador** cede *el inmueble*.\n\n- a. Que el ***plazo*** es de \\[dos\\] años.\n\n> Ver [https://ejemplo.org/x](https://ejemplo.org/x) y <https://ejemplo.org/y>.<br>Fin~~tachado~~[^1]\n\n[^1]: Una nota.\n\n---\n\n| Nombre | Monto |\n| --- | --- |\n| Ana | $ 10 |";
        let out = comparable(body);
        assert!(out.contains("1. Objeto"));
        assert!(out.contains("El Locador cede el inmueble."));
        assert!(out.contains("a. Que el plazo es de [dos] años."));
        assert!(out.contains("Ver https://ejemplo.org/x y https://ejemplo.org/y. Fintachado"));
        assert!(out.contains("Una nota."));
        assert!(!out.contains('*') && !out.contains('#') && !out.contains("---") && !out.contains('|'));
        // The words are the same, but the address is written once.
        assert_eq!(text::normalized(&out).matches("ejemplo org x").count(), 1);
        assert_eq!(text::normalized("**pala**bra"), "pala bra");
        assert_eq!(text::normalized(&comparable("**pala**bra")), "palabra");
    }

    #[test]
    fn what_the_comparison_reads_stays() {
        let body = "## Page 1\n\nTexto\n\n### Form fields\n\n- Nombre: Ana\n\n<!-- texto-reconocido motor -->";
        let out = comparable(body);
        assert!(out.contains("## Page 1\n"));
        assert!(out.contains("### Form fields\n"));
        assert!(out.contains("<!-- texto-reconocido motor -->"));
    }

    #[test]
    fn a_line_end_hyphen_joins_a_word_the_text_knows_and_is_a_hyphen_otherwise() {
        let body = "## Page 1\r\n\r\nEl contra\u{2}to de uso real\u{2}world.\r\nEl contrato sigue.";
        let out = comparable(body);
        assert!(!out.contains('\r') && !out.contains('\u{2}'));
        assert!(out.contains("El contrato de uso real-world."));
    }

    #[test]
    fn page_number_lines_go_when_several_pages_follow_them() {
        let body = "## Page 1\n\nCLÁUSULA PRIMERA\nEl precio es 15\n- 1 -\n\n## Page 2\n\n2\nsigue el texto\n15\n\n## Page 3\n\nPágina 3 de 3\nfin del texto";
        let out = comparable(body);
        let lines: Vec<&str> = out.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        // "15" at the bottom of the second page is not its number.
        assert_eq!(lines, ["## Page 1", "CLÁUSULA PRIMERA", "El precio es 15", "## Page 2", "sigue el texto", "15", "## Page 3", "fin del texto"]);
        // Numbered from the second page on, as a cover page leaves them.
        let covered = "## Page 1\n\nPortada\n\n## Page 2\n\nTexto\n1\n\n## Page 3\n\nMás texto\n2";
        assert!(!comparable(covered).lines().any(|l| l == "1" || l == "2"));
        // One page alone with a number: nothing says it is a page number.
        let one = "## Page 1\n\nTexto\n\n## Page 2\n\nOtro texto\n7";
        assert_eq!(comparable(one), one);
        // A Word file has no pages.
        assert_eq!(comparable("Texto\n\n3"), "Texto\n\n3");
    }

    #[test]
    fn a_word_file_made_from_a_pdf_loses_its_page_number_paragraphs() {
        let body = "Primera parte del texto.\n\n2\n\nSegunda parte, con 15 días.\n\n7\n\nTercera parte.\n\n3\n\nCuarta parte.\n\n4\n\nFin.";
        let out = comparable(body);
        let kept: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
        // "7" does not follow the others.
        assert_eq!(kept, ["Primera parte del texto.", "Segunda parte, con 15 días.", "7", "Tercera parte.", "Cuarta parte.", "Fin."]);
        // Two alone are not enough.
        assert_eq!(comparable("Texto.\n\n2\n\nMás.\n\n3"), "Texto.\n\n2\n\nMás.\n\n3");
    }

    #[test]
    fn a_link_is_its_text_once() {
        let out = comparable("Ver[https://ejemplo.org/a](https://ejemplo.org/a)[https://ejemplo.org/a](https://ejemplo.org/a)y [el sitio](https://ejemplo.org/b).");
        assert_eq!(out, "Ver https://ejemplo.org/a y el sitio.");
    }

    #[test]
    fn page_number_shapes() {
        for (line, number) in [("3", 3), ("- 3 -", 3), ("Página 3", 3), ("Pág. 3 de 10", 3), ("3/10", 3), ("Page 3 of 10", 3), ("| 12 |", 12)] {
            assert_eq!(page_number(line), Some(number), "{line}");
        }
        for no in ["$ 3", "3,50", "0", "12345", "3 10", "Página tres", "fs. 3", "4 de 2", "15%"] {
            assert_eq!(page_number(no), None, "{no}");
        }
    }
}
