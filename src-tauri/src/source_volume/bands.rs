//! The text of a document with its bands (D-668, E1 of 2 nov): what a
//! reader read with low confidence stays in the text, marked line by line,
//! and what it read with less than the low threshold does not enter.
//!
//! - A firm line is plain text: the only text that gives data (names,
//!   numbers of a document, dates, amounts, domiciles).
//! - An approximate line starts with `[aproximado]`: it is there to
//!   understand the document, and nothing that comes only from it is kept
//!   as a datum. The signature of the similar documents, the type and the
//!   search may use it, as a signal (`unmarked`); the extraction, the data
//!   of the type, the identities, the matters and the pages read only the
//!   firm text (`firm`), and the questions see the marked lines with a note
//!   (`NOTE`).
//! - A discarded line is not in the text at all.
//!
//! The mark goes on each line, so it survives any cut of the text: by
//! pages (the parts of a PDF), by fragments of the extraction, by the
//! excerpt of a question.

/// What starts a line read with low confidence.
pub const APPROXIMATE: &str = "[aproximado]";

/// What a question says of the marked lines, when its text has one.
pub const NOTE: &str = "Los renglones que empiezan con «[aproximado]» se leyeron con poca seguridad: sirven solo para entender \
el documento. No copies de ellos nombres, números, fechas, importes ni domicilios, ni los uses como cita: lo que salga solo de \
esos renglones se descarta.";

/// A line read with low confidence, with its mark.
pub fn approximate_line(text: &str) -> String {
    format!("{APPROXIMATE} {}", text.trim())
}

/// Whether a line is marked as read with low confidence.
pub fn is_approximate(line: &str) -> bool {
    line.trim_start().starts_with(APPROXIMATE)
}

/// The line without its mark (the same line when it has none): what the
/// signals see of it.
pub fn without_mark(line: &str) -> &str {
    match line.trim_start().strip_prefix(APPROXIMATE) {
        Some(rest) => rest.trim_start(),
        None => line,
    }
}

/// Whether a text has a line read with low confidence.
pub fn has_approximate(text: &str) -> bool {
    text.lines().any(is_approximate)
}

/// The firm text: the lines read with low confidence go away. What gives
/// data is checked against this.
pub fn firm(text: &str) -> String {
    if !has_approximate(text) {
        return text.to_string();
    }
    text.lines().filter(|line| !is_approximate(line)).collect::<Vec<_>>().join("\n")
}

/// The text with the lines read with low confidence without their mark:
/// what a signal (the signature, the search) sees.
pub fn unmarked(text: &str) -> String {
    if !has_approximate(text) {
        return text.to_string();
    }
    text.lines().map(without_mark).collect::<Vec<_>>().join("\n")
}

/// The note for a question whose text has marked lines, with its line
/// break; empty when it has none.
pub fn note_for(text: &str) -> String {
    if has_approximate(text) {
        format!("{NOTE}\n\n")
    } else {
        String::new()
    }
}

/// The marked text of a page from its lines, each with its band
/// (`firme`, `aproximada` or `descartada`), in reading order. A page with
/// lines but none firm or approximate is unreadable (`[ilegible]`), so the
/// screens offer it to another engine; one without lines is blank.
pub fn marked_text<'a>(lines: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let (mut out, mut any) = (Vec::new(), false);
    for (text, band) in lines {
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        any = true;
        match band {
            "firme" => out.push(text.to_string()),
            "aproximada" => out.push(approximate_line(text)),
            _ => {}
        }
    }
    if out.is_empty() && any {
        return super::recognition::UNREADABLE.to_string();
    }
    out.join("\n")
}

/// The band of a line by its confidence alone, for a reader that gives no
/// zones (Micelya Core): firm from the high threshold, approximate from
/// the low one, discarded below it. A line without confidence is firm.
pub fn band_of(confidence: Option<f64>, low: f64, high: f64) -> &'static str {
    match confidence {
        None => "firme",
        Some(c) if c >= high => "firme",
        Some(c) if c >= low => "aproximada",
        Some(_) => "descartada",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_firm_text_drops_the_marked_lines_and_the_signals_keep_their_words() {
        let text = marked_text([("ACTA DE NACIMIENTO", "firme"), ("Hijo de Persona Uno", "aproximada"), ("garabato", "descartada")]);
        assert_eq!(text, "ACTA DE NACIMIENTO\n[aproximado] Hijo de Persona Uno");
        assert_eq!(firm(&text), "ACTA DE NACIMIENTO");
        assert_eq!(unmarked(&text), "ACTA DE NACIMIENTO\nHijo de Persona Uno");
        assert!(has_approximate(&text) && !has_approximate(&firm(&text)));
        assert!(note_for(&text).contains("[aproximado]") && note_for("solo firme").is_empty());
        // Lines, none firm or approximate: unreadable; no line: blank.
        assert_eq!(marked_text([("garabato", "descartada")]), super::super::recognition::UNREADABLE);
        assert_eq!(marked_text(std::iter::empty::<(&str, &str)>()), "");
    }

    #[test]
    fn a_line_without_zones_takes_its_band_by_its_confidence() {
        assert_eq!(band_of(Some(0.99), 0.90, 0.97), "firme");
        assert_eq!(band_of(Some(0.93), 0.90, 0.97), "aproximada");
        assert_eq!(band_of(Some(0.93), 0.95, 0.97), "descartada");
        assert_eq!(band_of(None, 0.90, 0.97), "firme");
    }
}
