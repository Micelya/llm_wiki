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
//! as the field name instead: its first sentence, when it goes on with
//! instructions ("… de la Operación. En caso negativo, …"). Unticked boxes
//! and unchosen options are not written: the label of the chosen one, plus
//! the question, already says which answer was given; the label of a
//! ticked box says so ("SI (marcada)"), so that nobody takes the printed
//! label of an option for the answer.
//!
//! A form filled and then flattened (printed to PDF, or signed by an
//! electronic signature service) has no fields left: a ticked box is a
//! tick glyph ("✔") printed where the box was. Those glyphs are read the
//! same way, as ticked boxes at their place on the page (P-135).

use pdfium_render::prelude::*;

/// Heading of the list of filled-in form fields in the extracted text.
pub const FORM_FIELDS_HEADING: &str = "### Form fields";

/// Farthest a label can be from its button, in points.
const MAX_LABEL_GAP: f32 = 36.0;
/// Most words taken as the label of a button.
const MAX_LABEL_WORDS: usize = 8;
/// Longest question used as the name of a field, in characters.
const MAX_QUESTION_CHARS: usize = 200;
/// What a ticked box or a chosen option is written with, after its label.
const TICKED: &str = "(marcada)";
/// Glyphs a flattened form prints where a box was ticked.
const TICK_GLYPHS: &[char] = &['✔', '✓', '☑', '☒', '✗', '✘', '🗸'];

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
    fn center_x(&self) -> f32 {
        (self.left + self.right) / 2.0
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

    // Reading the words of the page costs something: only when a box is
    // ticked, or when the page may be a flattened form (no box at all).
    let has_ticks = buttons.is_empty() && page.text().is_ok_and(|text| text.all().chars().any(|c| TICK_GLYPHS.contains(&c)));
    let words = if buttons.iter().any(|button| button.on) || has_ticks { page_words(page) } else { Vec::new() };
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
    if has_ticks {
        fields.extend(tick_fields(&words, &vertical_rules(page)));
    }
    fields
}

/// What a tick glyph says when the option it ticks cannot be told: it is
/// written, with the options it may be, so that nobody reads either one.
const TICK_UNKNOWN: &str = "marcada, sin saber cuál de las opciones";
/// Name of a tick with no question printed on its row.
const TICK_NO_QUESTION: &str = "Casilla marcada";

/// The x of the thin vertical lines of the page (the rules of a table),
/// with the heights they span: they split a row into cells.
fn vertical_rules(page: &PdfPage<'_>) -> Vec<Rect> {
    page.objects()
        .iter()
        .filter(|object| object.object_type() == PdfPageObjectType::Path)
        .filter_map(|object| object.bounds().ok().map(|b| Rect::from_pdf(b.to_rect())))
        .filter(|r| (r.right - r.left).abs() <= 2.5 && (r.top - r.bottom).abs() >= 8.0)
        .collect()
}

/// The ticks printed on a page that has no tick boxes (a flattened form),
/// each as (question, chosen option). The option is the label printed in
/// the cell next to the tick, on the side where every option of the row
/// has its box (an empty cell); without the rules of a table, the only
/// label within reach. When it cannot be told, the tick says so and names
/// the options it may be: it never takes one by guessing (P-135).
fn tick_fields(words: &[Word], rules: &[Rect]) -> Vec<(String, String)> {
    let ticks: Vec<&Word> = words.iter().filter(|word| is_tick(&word.text)).collect();
    let others: Vec<&Word> = words.iter().filter(|word| !is_tick(&word.text)).collect();
    let mut out = Vec::new();
    for tick in ticks {
        let y = tick.rect.center_y();
        let line: Vec<&Word> = others.iter().copied().filter(|word| word.rect.spans_y(y) || tick.rect.spans_y(word.rect.center_y())).collect();
        // The cells of the row, between the rules that cross it.
        let mut xs: Vec<f32> = rules.iter().filter(|r| r.spans_y(y)).map(|r| (r.left + r.right) / 2.0).collect();
        xs.sort_by(f32::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 3.0);
        let (label, options, limit) = if xs.len() >= 3 {
            label_by_cells(tick, &line, &xs)
        } else {
            label_by_reach(tick, &line)
        };
        // The question: what is printed before the options, on the lines
        // around the tick (a question can take two or three lines).
        let height = tick.rect.height();
        let around = Rect { left: tick.rect.left, right: tick.rect.right, bottom: tick.rect.bottom - 2.0 * height, top: tick.rect.top + 2.0 * height };
        let near: Vec<Word> = others
            .iter()
            .filter(|word| word.rect.right <= limit && around.spans_y(word.rect.center_y()))
            .map(|word| Word { text: word.text.clone(), rect: word.rect })
            .collect();
        let probe = Button { name: String::new(), on: true, state: None, rect: around };
        let question = question_before(&probe, limit, &near).map(|q| first_sentence(&q)).filter(|q| !q.is_empty());
        let value = match label {
            Some(label) => format!("{label} {TICKED}"),
            None if !options.is_empty() => format!("{TICK_UNKNOWN}: {}", options.iter().map(|o| format!("«{o}»")).collect::<Vec<_>>().join(", ")),
            None => "marcada".to_string(),
        };
        out.push((question.unwrap_or_else(|| TICK_NO_QUESTION.to_string()), value));
    }
    out
}

/// The label of a tick in a table: the text of each cell of its row; the
/// first one with text is the question; the others are the options. The
/// options are left of their boxes when the cell after each one is empty,
/// right of them when the cell before each one is (and is not the
/// question's). Returns the label (when exactly one reading gives one),
/// the options next to the tick, and where the options start.
fn label_by_cells(tick: &Word, line: &[&Word], xs: &[f32]) -> (Option<String>, Vec<String>, f32) {
    let cells = xs.len() - 1;
    let cell_of = |x: f32| (0..cells).find(|&i| xs[i] <= x && x < xs[i + 1]);
    let mut texts: Vec<Vec<&Word>> = vec![Vec::new(); cells];
    for word in line {
        if let Some(i) = cell_of(word.rect.center_x()) {
            texts[i].push(word);
        }
    }
    let text = |i: usize| -> String {
        let mut list = texts[i].clone();
        list.sort_by(|a, b| a.rect.left.total_cmp(&b.rect.left));
        list.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
    };
    let Some(t) = cell_of(tick.rect.center_x()) else { return (None, Vec::new(), tick.rect.left) };
    let with_text: Vec<usize> = (0..cells).filter(|&i| !texts[i].is_empty()).collect();
    let question = if with_text.len() >= 2 { with_text.first().copied() } else { None };
    let labels: Vec<usize> = with_text.iter().copied().filter(|i| Some(*i) != question).collect();
    let empty = |i: usize| texts[i].is_empty();
    let left_ok = !labels.is_empty() && labels.iter().all(|&i| i + 1 < cells && empty(i + 1));
    let right_ok = !labels.is_empty() && labels.iter().all(|&i| i >= 1 && Some(i - 1) != question && empty(i - 1));
    let before = (t >= 1 && labels.contains(&(t - 1))).then(|| text(t - 1));
    let after = (labels.contains(&(t + 1))).then(|| text(t + 1));
    let options: Vec<String> = [before.clone(), after.clone()].into_iter().flatten().collect();
    let label = match (left_ok.then_some(before).flatten(), right_ok.then_some(after).flatten()) {
        (Some(one), None) | (None, Some(one)) => Some(one),
        _ => None,
    };
    let limit = labels.first().map(|&i| xs[i]).unwrap_or(tick.rect.left).min(tick.rect.left);
    (label, options, limit)
}

/// The label of a tick with no table: the words right next to it, when
/// only one side has any within reach.
fn label_by_reach(tick: &Word, line: &[&Word]) -> (Option<String>, Vec<String>, f32) {
    let words: Vec<Word> = line.iter().map(|w| Word { text: w.text.clone(), rect: w.rect }).collect();
    let probe = Button { name: String::new(), on: true, state: None, rect: tick.rect };
    let row = [&probe];
    let left = label_beside(&row, 0, tick.rect.center_y(), &words, Side::Left);
    let right = label_beside(&row, 0, tick.rect.center_y(), &words, Side::Right);
    match (left, right) {
        (Some(l), None) => (Some(l.text), Vec::new(), l.left),
        (None, Some(r)) => (Some(r.text), Vec::new(), tick.rect.left),
        (Some(l), Some(r)) => (None, vec![l.text, r.text], l.left),
        (None, None) => (None, Vec::new(), tick.rect.left),
    }
}

/// Whether a word is only tick glyphs.
fn is_tick(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| TICK_GLYPHS.contains(&c))
}

/// The filled-in fields of a PDF form, page after page (also the ticks of
/// a flattened one); empty for a PDF that is not a form or was left blank.
///
/// Takes the global pdfium lock, like
/// [`crate::commands::extract_images::extract_pdf_markdown`].
pub fn pdf_form_fields(path: &str) -> Result<Vec<(String, String)>, String> {
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to open PDF '{path}': {e}"))?;
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
    let question = question.map(|question| first_sentence(&question)).filter(|question| !question.is_empty());
    let field_name = if is_placeholder_name(name) {
        question.clone()
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
        return Some(match (question, label) {
            (Some(question), Some(label)) => (question, format!("{label} {TICKED}")),
            (None, Some(label)) => (label, "marcado".to_string()),
            (Some(question), None) => (question, "marcado".to_string()),
            (None, None) => (field_name, "marcado".to_string()),
        });
    }

    let values: Vec<String> = chosen
        .iter()
        .map(|(index, button)| {
            if let Some(state) = button.state.as_deref().filter(|state| is_option_name(state)) {
                return format!("{state} {TICKED}");
            }
            let (label, _) = label_and_question(button, buttons, words);
            label.map(|label| format!("{label} {TICKED}")).unwrap_or_else(|| format!("opción {} de {}", index + 1, group.len()))
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

/// The first sentence of a question printed before a row of boxes, when
/// what follows it is another sentence (instructions): "Indique si es X de
/// la Operación. En caso negativo, complete…" is "Indique si es X de la
/// Operación". The end of the sentence is left out.
fn first_sentence(question: &str) -> String {
    let chars: Vec<char> = question.chars().collect();
    let mut end = chars.len();
    for i in 0..chars.len() {
        let stop = matches!(chars[i], '.' | '?' | '!');
        let next_upper = chars.get(i + 1) == Some(&' ') && chars.get(i + 2).is_some_and(|c| c.is_uppercase());
        if stop && next_upper && chars[..i].iter().filter(|c| **c == ' ').count() >= 2 {
            end = i;
            break;
        }
    }
    let sentence: String = chars[..end].iter().collect();
    sentence.trim().trim_end_matches([':', ';', '.']).trim().to_string()
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

    /// Prints what pdfium says of every button of the PDF in
    /// `MICELYA_FORM_PDF` (a real form, outside the repository).
    #[test]
    #[ignore = "reads a real PDF named in MICELYA_FORM_PDF"]
    fn probe_buttons_of_a_real_form() {
        let Ok(path) = std::env::var("MICELYA_FORM_PDF") else { return };
        let _guard = crate::commands::fs::lock_pdfium();
        let pdfium = crate::commands::fs::pdfium().unwrap();
        let doc = pdfium.load_pdf_from_file(&path, None).unwrap();
        println!("form: {:?}", doc.form().map(|f| f.form_type()));
        for (index, page) in doc.pages().iter().enumerate() {
            for annotation in page.annotations().iter() {
                let kind = annotation.annotation_type();
                let Some(field) = annotation.as_form_field() else {
                    println!("page {index}: {kind:?} without field");
                    continue;
                };
                println!(
                    "page {index}: {kind:?} {:?} {:?} as={:?} bounds={:?}",
                    field.field_type(),
                    field.name(),
                    field.appearance_stream(),
                    annotation.bounds().ok()
                );
            }
            println!("page {index} fields: {:?}", page_form_fields(&page));
            let words = page_words(&page);
            for tick in words.iter().filter(|w| is_tick(&w.text)) {
                println!("  tick at {:?}", tick.rect);
                for word in words.iter().filter(|w| (w.rect.center_y() - tick.rect.center_y()).abs() < 25.0) {
                    println!("    word {:?} {:?}", word.text, word.rect);
                }
                for object in page.objects().iter() {
                    if let Ok(b) = object.bounds() {
                        let r = Rect::from_pdf(b.to_rect());
                        if (r.center_y() - tick.rect.center_y()).abs() < 25.0 && (r.right - r.left) < 80.0 {
                            println!("    object {:?} {:?}", object.object_type(), r);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn placeholder_names_are_told_from_real_ones() {
        assert!(is_placeholder_name("Check Box12"));
        assert!(is_placeholder_name("form1.Casilla de verificación 3"));
        assert!(is_placeholder_name("Group1"));
        assert!(!is_placeholder_name("Beneficiario final"));
        assert!(!is_placeholder_name("Acepta notificaciones"));
    }

    #[test]
    fn a_question_is_named_by_its_first_sentence() {
        assert_eq!(
            first_sentence("Indique si es el titular de la cuenta. En caso negativo, complete otro formulario."),
            "Indique si es el titular de la cuenta"
        );
        assert_eq!(first_sentence("Sírvase indicar si operará"), "Sírvase indicar si operará");
        assert_eq!(first_sentence("Estado civil:"), "Estado civil");
        // An abbreviation at the start is not the end of a sentence.
        assert_eq!(first_sentence("Sr. Pérez declara si es residente"), "Sr. Pérez declara si es residente");
    }

    fn word(text: &str, left: f32, right: f32, bottom: f32) -> Word {
        Word { text: text.to_string(), rect: Rect { left, right, bottom, top: bottom + 12.0 } }
    }

    fn rule(x: f32) -> Rect {
        Rect { left: x, right: x + 1.0, bottom: 0.0, top: 400.0 }
    }

    #[test]
    fn a_tick_in_a_table_takes_the_option_whose_box_it_is_in() {
        // | Es cliente del estudio? | SI | ✔ | NO |   |
        let words = vec![
            word("Es", 40.0, 50.0, 100.0),
            word("cliente", 52.0, 80.0, 100.0),
            word("del", 82.0, 92.0, 100.0),
            word("estudio?", 94.0, 130.0, 100.0),
            word("SI", 210.0, 220.0, 100.0),
            word("✔", 240.0, 248.0, 101.0),
            word("NO", 270.0, 284.0, 100.0),
        ];
        let rules: Vec<Rect> = [30.0, 200.0, 230.0, 260.0, 290.0, 320.0].iter().map(|&x| rule(x)).collect();
        assert_eq!(tick_fields(&words, &rules), vec![("Es cliente del estudio?".to_string(), "SI (marcada)".to_string())]);
        // The same tick with no table: an option on each side, it is not
        // known which one it is.
        let fields = tick_fields(&words, &[]);
        assert!(fields[0].1.starts_with(TICK_UNKNOWN), "{fields:?}");
        assert!(fields[0].1.contains("«SI»") && fields[0].1.contains("«NO»"), "{fields:?}");
    }

    #[test]
    fn a_tick_glyph_is_a_ticked_box() {
        assert!(is_tick("✔"));
        assert!(is_tick("✓✓"));
        assert!(!is_tick("X"));
        assert!(!is_tick(""));
        assert!(!is_tick("SI"));
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
