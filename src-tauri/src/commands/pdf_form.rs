//! What was filled into a fillable PDF form, as (field, value) pairs.
//!
//! Text, combo and list fields say what they hold. Tick boxes and option
//! buttons do not: a ticked box only says "on", and an option group such
//! as "yes / no" is one field with several buttons. So for each ticked
//! box or chosen option this module says *which* option it is, taking
//! the best name it can find, in this order:
//!
//! 1. the name the form gives the chosen button (its appearance state,
//!    e.g. "No"), when it is a real word and not a placeholder such as
//!    "Choice2" or "1";
//! 2. the text printed next to the button on the page (its label), on
//!    the side where the labels of that row are;
//! 3. its position: "opción 2 de 3".
//!
//! When the field itself has a placeholder name ("Check Box12"), the
//! text printed on the same row before the buttons (the question) is used
//! as the field name instead. Unticked boxes and unchosen options are not
//! written: the label of the chosen one, plus the question, already says
//! which answer was given.

use pdfium_render::prelude::*;

/// Heading of the list of filled-in form fields in the extracted text.
pub const FORM_FIELDS_HEADING: &str = "### Form fields";

/// Farthest a label can be from its button, in points.
const MAX_LABEL_GAP: f32 = 36.0;
/// Most words taken as the label of a button.
const MAX_LABEL_WORDS: usize = 8;
/// Longest question used as the name of a field, in characters.
const MAX_QUESTION_CHARS: usize = 200;

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
}

impl Rect {
    fn from_pdf(rect: PdfRect) -> Self {
        Rect { left: rect.left().value, right: rect.right().value, bottom: rect.bottom().value, top: rect.top().value }
    }
    fn center_y(&self) -> f32 {
        (self.bottom + self.top) / 2.0
    }
    fn height(&self) -> f32 {
        (self.top - self.bottom).abs().max(1.0)
    }
    fn spans_y(&self, y: f32) -> bool {
        self.bottom <= y && y <= self.top
    }
}

/// One word printed on the page and where it is.
#[derive(Debug)]
struct Word {
    text: String,
    rect: Rect,
}

/// One tick box or option button.
struct Button {
    /// Field it belongs to; buttons of one option group share it.
    name: String,
    on: bool,
    /// Its appearance state while on (the name of the option).
    state: Option<String>,
    rect: Rect,
}

enum Item {
    Value(String, String),
    /// A button field; holds the index of its first button.
    Buttons(String),
}

/// The filled-in fields of one page of a fillable form, as (name,
/// value), in page order. Empty fields, unticked boxes, unchosen options,
/// push buttons and signatures are left out.
pub(crate) fn page_form_fields(page: &PdfPage<'_>) -> Vec<(String, String)> {
    let mut items: Vec<Item> = Vec::new();
    let mut buttons: Vec<Button> = Vec::new();
    for annotation in page.annotations().iter() {
        let Some(field) = annotation.as_form_field() else {
            continue;
        };
        let name = field.name().map(|name| name.trim().to_string()).unwrap_or_default();
        let on = match field {
            PdfFormField::Checkbox(button) => Some(is_on(button.appearance_stream(), || button.is_checked())),
            PdfFormField::RadioButton(button) => Some(is_on(button.appearance_stream(), || button.is_checked())),
            _ => None,
        };
        if let Some(on) = on {
            let state = field.appearance_stream().map(|state| clean(&state)).filter(|state| !state.is_empty());
            let rect = annotation.bounds().map(Rect::from_pdf).unwrap_or(Rect { left: 0.0, right: 0.0, bottom: 0.0, top: 0.0 });
            if !items.iter().any(|item| matches!(item, Item::Buttons(other) if *other == name)) {
                items.push(Item::Buttons(name.clone()));
            }
            buttons.push(Button { name, on, state, rect });
            continue;
        }
        let value = match field {
            PdfFormField::Text(field) => field.value(),
            PdfFormField::ComboBox(field) => field.value(),
            PdfFormField::ListBox(field) => field.value(),
            _ => None,
        };
        let value = value.map(|value| clean(&value)).unwrap_or_default();
        if !value.is_empty() {
            items.push(Item::Value(if name.is_empty() { "-".to_string() } else { name }, value));
        }
    }

    // Reading the words of the page costs something: only when a box is ticked.
    let words = if buttons.iter().any(|button| button.on) { page_words(page) } else { Vec::new() };
    let mut fields = Vec::new();
    for item in items {
        match item {
            Item::Value(name, value) => fields.push((name, value)),
            Item::Buttons(name) => {
                if let Some(field) = describe_buttons(&name, &buttons, &words) {
                    fields.push(field);
                }
            }
        }
    }
    fields
}

/// The filled-in fields of a fillable PDF form, page after page; empty
/// for a PDF that is not a form or was left blank.
///
/// Takes the global pdfium lock, like
/// [`crate::commands::extract_images::extract_pdf_markdown`].
pub fn pdf_form_fields(path: &str) -> Result<Vec<(String, String)>, String> {
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to open PDF '{path}': {e}"))?;
    if doc.form().is_none() {
        return Ok(Vec::new());
    }
    let mut fields = Vec::new();
    for page in doc.pages().iter() {
        fields.extend(page_form_fields(&page));
    }
    Ok(fields)
}

/// A button is on when its appearance state is anything but "Off" (pdfium
/// only takes "Yes" as on for a tick box, and forms in other languages
/// use other words). Without an appearance state, pdfium decides.
fn is_on(state: Option<String>, checked: impl Fn() -> Result<bool, PdfiumError>) -> bool {
    match state.map(|state| clean(&state)) {
        Some(state) if !state.is_empty() => !state.eq_ignore_ascii_case("off"),
        _ => checked().unwrap_or(false),
    }
}

/// One line for a button field (a single tick box or an option group),
/// or none when nothing in it is ticked.
fn describe_buttons(name: &str, buttons: &[Button], words: &[Word]) -> Option<(String, String)> {
    let group: Vec<&Button> = buttons.iter().filter(|button| button.name == name).collect();
    let chosen: Vec<(usize, &Button)> = group.iter().copied().enumerate().filter(|(_, button)| button.on).collect();
    let (_, first) = *chosen.first()?;
    let (label, question) = label_and_question(first, buttons, words);
    let field_name = if is_placeholder_name(name) {
        question.clone().filter(|question| !question.is_empty())
    } else {
        None
    }
    .unwrap_or_else(|| if name.is_empty() { "-".to_string() } else { name.to_string() });

    if group.len() == 1 {
        // A single tick box: its own name says what it is, unless it is a
        // placeholder; then the printed label does.
        if !is_placeholder_name(name) {
            return Some((field_name, "marcado".to_string()));
        }
        return Some(match (question.filter(|question| !question.is_empty()), label) {
            (Some(question), Some(label)) => (question, label),
            (None, Some(label)) => (label, "marcado".to_string()),
            (Some(question), None) => (question, "marcado".to_string()),
            (None, None) => (field_name, "marcado".to_string()),
        });
    }

    let values: Vec<String> = chosen
        .iter()
        .map(|(index, button)| {
            if let Some(state) = button.state.as_deref().filter(|state| is_option_name(state)) {
                return state.to_string();
            }
            let (label, _) = label_and_question(button, buttons, words);
            label.unwrap_or_else(|| format!("opción {} de {}", index + 1, group.len()))
        })
        .collect();
    Some((field_name, values.join(", ")))
}

/// The label printed next to a button, and the question printed before
/// the buttons of its row.
fn label_and_question(button: &Button, buttons: &[Button], words: &[Word]) -> (Option<String>, Option<String>) {
    let y = button.rect.center_y();
    let mut row: Vec<&Button> = buttons.iter().filter(|other| other.rect.spans_y(y)).collect();
    row.sort_by(|a, b| a.rect.left.total_cmp(&b.rect.left));
    let left: Vec<Option<Label>> = (0..row.len()).map(|i| label_beside(&row, i, y, words, Side::Left)).collect();
    let right: Vec<Option<Label>> = (0..row.len()).map(|i| label_beside(&row, i, y, words, Side::Right)).collect();
    // The labels of a row are all on the same side of their buttons: the
    // side where every button has one; if both, the closer one.
    let all_left = left.iter().all(Option::is_some);
    let all_right = right.iter().all(Option::is_some);
    let gaps = |labels: &[Option<Label>]| labels.iter().flatten().map(|label| label.gap).sum::<f32>();
    let side = match (all_left, all_right) {
        (true, false) => Some(Side::Left),
        (false, true) => Some(Side::Right),
        (true, true) if gaps(&left) < gaps(&right) => Some(Side::Left),
        (true, true) => Some(Side::Right),
        (false, false) => None,
    };
    let index = row.iter().position(|other| std::ptr::eq(*other, button)).unwrap_or(0);
    let label = match side {
        Some(Side::Left) => left[index].clone(),
        Some(Side::Right) => right[index].clone(),
        None => match (&left[index], &right[index]) {
            (Some(l), Some(r)) => Some(if l.gap < r.gap { l.clone() } else { r.clone() }),
            (l, r) => l.clone().or_else(|| r.clone()),
        },
    };

    // The question ends where the row starts: at its first button, or at
    // the label before it.
    let mut limit = row.first().map(|first| first.rect.left).unwrap_or(button.rect.left);
    if side == Some(Side::Left) {
        if let Some(Some(label)) = left.first() {
            limit = limit.min(label.left);
        }
    }
    let question = question_before(button, limit, words);
    (label.map(|label| label.text), question)
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone)]
struct Label {
    text: String,
    gap: f32,
    left: f32,
}

/// The words on one side of the button `row[i]`, on its line, up to the
/// next button of the row: the closest one and those right next to it.
fn label_beside(row: &[&Button], i: usize, y: f32, words: &[Word], side: Side) -> Option<Label> {
    let button = row[i].rect;
    let (from, to) = match side {
        Side::Left => (if i > 0 { row[i - 1].rect.right } else { f32::MIN }, button.left),
        Side::Right => (button.right, row.get(i + 1).map(|next| next.rect.left).unwrap_or(f32::MAX)),
    };
    let mut line: Vec<&Word> = words
        .iter()
        .filter(|word| word.rect.spans_y(y) && word.rect.left >= from - 1.0 && word.rect.right <= to + 1.0)
        .collect();
    line.sort_by(|a, b| a.rect.left.total_cmp(&b.rect.left));
    if side == Side::Left {
        line.reverse();
    }
    let closest = line.first()?;
    let gap = match side {
        Side::Left => button.left - closest.rect.right,
        Side::Right => closest.rect.left - button.right,
    };
    if gap > MAX_LABEL_GAP {
        return None;
    }
    let mut taken = vec![*closest];
    for word in line.iter().skip(1) {
        let last = taken.last().unwrap();
        let space = match side {
            Side::Left => last.rect.left - word.rect.right,
            Side::Right => word.rect.left - last.rect.right,
        };
        if space > last.rect.height() * 0.6 || taken.len() >= MAX_LABEL_WORDS {
            break;
        }
        taken.push(word);
    }
    if side == Side::Left {
        taken.reverse();
    }
    let left = taken.first().map(|word| word.rect.left).unwrap_or(button.left);
    let text = taken.iter().map(|word| word.text.as_str()).collect::<Vec<_>>().join(" ");
    Some(Label { text, gap: gap.max(0.0), left })
}

/// The text printed on the button's row before `limit`, line after line.
fn question_before(button: &Button, limit: f32, words: &[Word]) -> Option<String> {
    let rect = button.rect;
    let y = rect.center_y();
    let mut found: Vec<&Word> = words
        .iter()
        .filter(|word| word.rect.right <= limit && (rect.spans_y(word.rect.center_y()) || word.rect.spans_y(y)))
        .collect();
    if found.is_empty() {
        return None;
    }
    // Top to bottom, then left to right, grouping words of one line.
    found.sort_by(|a, b| b.rect.top.total_cmp(&a.rect.top));
    let mut lines: Vec<Vec<&Word>> = Vec::new();
    for word in found {
        match lines.last_mut() {
            Some(line) if line[0].rect.spans_y(word.rect.center_y()) => line.push(word),
            _ => lines.push(vec![word]),
        }
    }
    let mut text = String::new();
    for mut line in lines {
        line.sort_by(|a, b| a.rect.left.total_cmp(&b.rect.left));
        for word in line {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&word.text);
        }
    }
    if text.chars().count() > MAX_QUESTION_CHARS {
        let cut: String = text.chars().take(MAX_QUESTION_CHARS).collect();
        let cut = cut.rsplit_once(' ').map(|(head, _)| head.to_string()).unwrap_or(cut);
        text = format!("{cut}…");
    }
    Some(text)
}

/// The words printed on a page, with their boxes.
fn page_words(page: &PdfPage<'_>) -> Vec<Word> {
    let Ok(text) = page.text() else {
        return Vec::new();
    };
    let mut words: Vec<Word> = Vec::new();
    let mut current: Option<Word> = None;
    for ch in text.chars().iter() {
        let glyph = ch.unicode_string().unwrap_or_default();
        let bounds = ch.loose_bounds().ok().map(Rect::from_pdf);
        let (Some(bounds), false) = (bounds, glyph.trim().is_empty() || ch.is_generated().unwrap_or(false)) else {
            words.extend(current.take());
            continue;
        };
        if let Some(word) = current.as_mut() {
            let same_line = (bounds.center_y() - word.rect.center_y()).abs() < word.rect.height() / 2.0;
            let next_to = bounds.left >= word.rect.left - 1.0 && bounds.left - word.rect.right < word.rect.height() * 0.3;
            if same_line && next_to {
                word.text.push_str(&glyph);
                word.rect.right = word.rect.right.max(bounds.right);
                word.rect.bottom = word.rect.bottom.min(bounds.bottom);
                word.rect.top = word.rect.top.max(bounds.top);
                continue;
            }
            words.extend(current.take());
        }
        current = Some(Word { text: glyph, rect: bounds });
    }
    words.extend(current);
    words
}

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Letters only, lower case, without accents.
fn letters(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphabetic())
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            other => other,
        })
        .collect()
}

/// Names that form editors give a field when nobody names it
/// ("Check Box12", "Casilla de verificación 3", "Group1"...).
fn is_placeholder_name(name: &str) -> bool {
    const PLACEHOLDERS: &[&str] = &[
        "", "checkbox", "check", "casilla", "casilladeverificacion", "radio", "radiobutton", "group", "grupo",
        "botondeopcion", "boton", "button", "option", "opcion", "optionbutton", "choice", "field", "campo",
        "kontrollkastchen", "caseacocher", "untitled", "sintitulo",
    ];
    let last = name.rsplit('.').next().unwrap_or(name);
    PLACEHOLDERS.contains(&letters(last).as_str())
}

/// Whether the appearance state of a chosen option names the option, as
/// opposed to "On" or a placeholder such as "Choice2" or "0".
fn is_option_name(state: &str) -> bool {
    const PLACEHOLDERS: &[&str] = &[
        "", "on", "off", "choice", "option", "opcion", "radio", "button", "boton", "item", "value", "valor",
        "state", "estado", "check", "checkbox", "casilla",
    ];
    !PLACEHOLDERS.contains(&letters(state).as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_names_are_told_from_real_ones() {
        assert!(is_placeholder_name("Check Box12"));
        assert!(is_placeholder_name("form1.Casilla de verificación 3"));
        assert!(is_placeholder_name("Group1"));
        assert!(!is_placeholder_name("Beneficiario final"));
        assert!(!is_placeholder_name("Acepta notificaciones"));
    }

    #[test]
    fn option_names_are_told_from_placeholders() {
        assert!(is_option_name("No"));
        assert!(is_option_name("Sí"));
        assert!(is_option_name("Yes"));
        assert!(!is_option_name("Choice2"));
        assert!(!is_option_name("0"));
        assert!(!is_option_name("On"));
    }
}
