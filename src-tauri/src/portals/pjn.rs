//! The connector of the Sistema de Consulta Web of the Poder Judicial de la
//! Nación (SCW, `scw.pjn.gov.ar`): what only this portal knows.
//!
//! It is a JSF/Seam application: its ids are generated and change between
//! versions, so everything is found by the labels a person reads.
//!
//! - The form: a jurisdiction (a list), a number and a year, and before
//!   "Consultar" an anti-robot challenge ("Ver desafío") a person solves.
//! - The record (`expediente.seam`): its main data, and the table of its
//!   entries ("actuaciones"), 15 per page, paged by AJAX. The table is the
//!   one with the column "A FS."; each entry with a document has a
//!   "Descargar" link (a GET that answers the PDF with the session); the
//!   "MOVIMIENTO" ones have none. "Ver" opens a viewer: never used.
//! - The pager: a moving window of about ten numbers, the current page as
//!   a `<span>`, and arrows that are icons named by their `title`
//!   ("Primera página", "Anterior", "Siguiente", "Última página"). It is
//!   always found from the table of entries: under it there is a second
//!   paged table, «Notas» (who looked at the record), which is never read
//!   nor kept.

use super::page::{label_norm, norm, Page};
use super::{find_case_keys, CaseKey, Connector, DownloadLink, FormPlan, Linked, RecordData, RecordPage, Row, Screen};

pub struct Pjn;

pub const START_URL: &str = "https://scw.pjn.gov.ar/scw/home.seam";

/// The header of the column of folios: it names the table of entries.
const FOLIOS_HEADERS: &[&str] = &["a fs.", "a fs", "fs.", "fojas", "a fojas"];

/// The names of the arrows of a pager.
const NEXT: &str = "siguiente";
const PAGER_NAMES: &[&str] = &["siguiente", "anterior", "primera pagina", "ultima pagina", "primera", "ultima"];

impl Connector for Pjn {
    fn id(&self) -> &'static str {
        "pjn"
    }

    fn folder(&self) -> &'static str {
        "PJN"
    }

    fn name(&self) -> &'static str {
        "Poder Judicial de la Nación (Consulta Web)"
    }

    fn start_url(&self) -> &'static str {
        START_URL
    }

    fn allows(&self, url: &tauri::Url) -> bool {
        if url.scheme() == "about" {
            return url.as_str() == "about:blank";
        }
        if url.scheme() != "https" {
            return false;
        }
        match url.host_str() {
            Some(host) => {
                let host = host.to_lowercase();
                host == "pjn.gov.ar" || host.ends_with(".pjn.gov.ar")
            }
            None => false,
        }
    }

    fn screen(&self, page: &Page) -> Screen {
        if password_visible(page) {
            return Screen::Login;
        }
        let challenge = challenge_visible(page);
        if entries_table(page).is_some() {
            return if challenge { Screen::Challenge } else { Screen::Record };
        }
        if jurisdiction_select(page).is_some() && consult_button(page).is_some() {
            return Screen::Form { challenge };
        }
        if challenge {
            return Screen::Challenge;
        }
        Screen::Other
    }

    fn form(&self, page: &Page, key: &CaseKey) -> Result<FormPlan, String> {
        let select = jurisdiction_select(page).ok_or("No encontré la lista de jurisdicciones en el formulario del portal")?;
        let option = jurisdiction_option(page, select, &key.jurisdiction)
            .ok_or_else(|| format!("La jurisdicción {} no está en la lista del portal", key.jurisdiction))?;
        let select_mark = mark_of(page, select)?;
        let (number, year) = number_and_year(page, select).ok_or("No encontré dónde va el número y el año del expediente")?;
        let mut fills = vec![(select_mark, option)];
        fills.push((mark_of(page, number)?, key.number.to_string()));
        fills.push((mark_of(page, year)?, key.year.to_string()));
        let submit = consult_button(page).and_then(|b| page.el(b).mark().map(str::to_string));
        Ok(FormPlan { fills, submit, challenge: challenge_visible(page) })
    }

    fn record(&self, page: &Page) -> RecordPage {
        let Some(table) = entries_table(page) else {
            return RecordPage {
                data: RecordData::default(),
                rows: Vec::new(),
                next: None,
                page_number: None,
                signature: String::new(),
                linked: Vec::new(),
            };
        };
        let rows = entries(page, &table);
        let (next, page_number) = pager(page, table.table);
        let signature = format!(
            "{}|{}|{}",
            page_number.map(|n| n.to_string()).unwrap_or_default(),
            rows.len(),
            rows.first().map(|r| format!("{} {} {} {}", r.date, r.kind, r.folios, r.detail)).unwrap_or_default()
        );
        RecordPage { data: record_data(page, table.table), rows, next, page_number, signature, linked: linked(page, table.table) }
    }

    fn message(&self, page: &Page) -> Option<String> {
        page.els
            .iter()
            .enumerate()
            .filter(|(i, el)| {
                let class = norm(el.attr("class").unwrap_or(""));
                (class.contains("error") || class.contains("msg") || class.contains("mensaje") || class.contains("alert"))
                    && !el.text.is_empty()
                    && el.text.chars().count() < 300
                    && page.visible(*i)
            })
            .map(|(_, el)| el.text.clone())
            .next()
    }

    fn rows_per_page(&self) -> Option<usize> {
        Some(15)
    }

    fn jurisdictions(&self) -> Vec<(&'static str, &'static str)> {
        JURISDICTIONS.to_vec()
    }
}

/// The jurisdictions of the form, by the code that starts each option of
/// the portal's list. The names help choosing; the app looks the code up
/// in the portal's own list.
const JURISDICTIONS: &[(&str, &str)] = &[
    ("CIV", "Cámara Nacional de Apelaciones en lo Civil"),
    ("COM", "Cámara Nacional de Apelaciones en lo Comercial"),
    ("CNT", "Cámara Nacional de Apelaciones del Trabajo"),
    ("CAF", "Contencioso Administrativo Federal"),
    ("CCF", "Civil y Comercial Federal"),
    ("CSS", "Cámara Federal de la Seguridad Social"),
    ("CPE", "Penal Económico"),
    ("CCC", "Criminal y Correccional"),
    ("CFP", "Criminal y Correccional Federal"),
    ("CPF", "Cámara Federal de Casación Penal"),
    ("CPN", "Cámara Nacional de Casación en lo Criminal y Correccional"),
    ("CNE", "Cámara Nacional Electoral"),
    ("CSJ", "Corte Suprema de Justicia de la Nación"),
    ("FBB", "Justicia Federal de Bahía Blanca"),
    ("FCB", "Justicia Federal de Córdoba"),
    ("FCR", "Justicia Federal de Comodoro Rivadavia"),
    ("FCT", "Justicia Federal de Corrientes"),
    ("FGR", "Justicia Federal de General Roca"),
    ("FLP", "Justicia Federal de La Plata"),
    ("FMP", "Justicia Federal de Mar del Plata"),
    ("FMZ", "Justicia Federal de Mendoza"),
    ("FPA", "Justicia Federal de Paraná"),
    ("FPO", "Justicia Federal de Posadas"),
    ("FRE", "Justicia Federal de Resistencia"),
    ("FRO", "Justicia Federal de Rosario"),
    ("FSA", "Justicia Federal de Salta"),
    ("FSM", "Justicia Federal de San Martín"),
    ("FTU", "Justicia Federal de Tucumán"),
];

fn mark_of(page: &Page, index: usize) -> Result<String, String> {
    page.el(index).mark().map(str::to_string).ok_or_else(|| "La página cambió mientras la leía".to_string())
}

fn password_visible(page: &Page) -> bool {
    (0..page.len()).any(|i| page.el(i).tag == "input" && page.el(i).input_type() == "password" && page.visible(i))
}

/// The challenge shows as a button "Ver desafío", or as the frame of the
/// portal's challenge.
fn challenge_visible(page: &Page) -> bool {
    (0..page.len()).any(|i| {
        let el = page.el(i);
        if !page.visible(i) {
            return false;
        }
        match el.tag.as_str() {
            "a" | "button" | "input" | "span" | "div" | "label" => {
                let words = label_norm(&el.label());
                el.label().chars().count() <= 40 && words.starts_with("ver desafio")
            }
            "iframe" => norm(el.attr("src").unwrap_or("")).contains("captcha"),
            _ => false,
        }
    })
}

fn jurisdiction_select(page: &Page) -> Option<usize> {
    let selects: Vec<usize> = (0..page.len()).filter(|&i| page.el(i).tag == "select" && page.visible(i)).collect();
    for label in page.labels(&["jurisdiccion"], 0..page.len()) {
        if let Some(target) = page.el(label).attr("for") {
            if let Some(&select) = selects.iter().find(|&&s| page.el(s).attr("id") == Some(target)) {
                return Some(select);
            }
        }
        if let Some(&select) = selects.iter().find(|&&s| s > label && s - label <= 80) {
            return Some(select);
        }
    }
    // A single list in the page is the one.
    if selects.len() == 1 {
        return selects.first().copied();
    }
    None
}

/// The option of the list for a code: its text starts with the code, or
/// has it in parentheses, or the code is its value. Returns its value.
fn jurisdiction_option(page: &Page, select: usize, code: &str) -> Option<String> {
    let code = norm(code);
    // Its options, also those inside a group.
    (select + 1..=page.el(select).end)
        .filter(|&j| page.el(j).tag == "option")
        .find(|&j| {
            let el = page.el(j);
            let text = norm(&el.text);
            let value = norm(el.attr("value").unwrap_or(""));
            let starts = text.strip_prefix(&code).is_some_and(|rest| rest.is_empty() || !rest.starts_with(|c: char| c.is_alphanumeric()));
            starts || value == code || text.contains(&format!("({code})"))
        })
        .map(|j| page.el(j).attr("value").map(str::to_string).unwrap_or_else(|| page.el(j).text.clone()))
}

fn is_text_input(page: &Page, i: usize) -> bool {
    let el = page.el(i);
    el.tag == "input" && matches!(el.input_type().as_str(), "text" | "number" | "tel" | "search") && page.visible(i)
}

/// The fields of the number and the year (a text field, or a list for the
/// year): after their labels, or else the first two fields after the list
/// of jurisdictions in its form.
fn number_and_year(page: &Page, select: usize) -> Option<(usize, usize)> {
    let inputs: Vec<usize> = (0..page.len())
        .filter(|&i| is_text_input(page, i) || (i != select && page.el(i).tag == "select" && page.visible(i)))
        .collect();
    let after = |labels: Vec<usize>, skip: Option<usize>| -> Option<usize> {
        for label in labels {
            if let Some(target) = page.el(label).attr("for") {
                if let Some(&input) = inputs.iter().find(|&&s| page.el(s).attr("id") == Some(target)) {
                    return Some(input);
                }
            }
            if let Some(&input) = inputs.iter().find(|&&s| s > label && s - label <= 80 && Some(s) != skip) {
                return Some(input);
            }
        }
        None
    };
    let number = after(page.labels(&["numero", "nro", "nro."], 0..page.len()), None).filter(|&n| is_text_input(page, n));
    let year = after(page.labels(&["ano", "anio"], 0..page.len()), number);
    if let (Some(number), Some(year)) = (number, year) {
        if number != year {
            return Some((number, year));
        }
    }
    let form = page.closest(select, &["form"]);
    let mut own: Vec<usize> =
        inputs.into_iter().filter(|&i| form.is_none_or(|f| page.contains(f, i)) && i > select).collect();
    own.truncate(2);
    match own.as_slice() {
        [number, year] if is_text_input(page, *number) => Some((*number, *year)),
        _ => None,
    }
}

fn consult_button(page: &Page) -> Option<usize> {
    (0..page.len()).find(|&i| {
        let el = page.el(i);
        let clickable = match el.tag.as_str() {
            "button" | "a" => true,
            "input" => matches!(el.input_type().as_str(), "submit" | "button"),
            _ => false,
        };
        clickable && page.visible(i) && label_norm(&el.label()) == "consultar"
    })
}

/// The table of entries, with the place of each column.
struct EntriesTable {
    table: usize,
    header: usize,
    date: Option<usize>,
    kind: Option<usize>,
    detail: Option<usize>,
    folios: Option<usize>,
    office: Option<usize>,
}

fn cells(page: &Page, row: usize) -> Vec<usize> {
    page.children(row).into_iter().filter(|&c| matches!(page.el(c).tag.as_str(), "td" | "th")).collect()
}

fn entries_table(page: &Page) -> Option<EntriesTable> {
    let header_cell = (0..page.len()).find(|&i| {
        let el = page.el(i);
        matches!(el.tag.as_str(), "th" | "td") && FOLIOS_HEADERS.contains(&label_norm(&el.text).as_str())
    })?;
    let header = page.closest(header_cell, &["tr"])?;
    let table = page.closest(header, &["table"])?;
    let mut found = EntriesTable { table, header, date: None, kind: None, detail: None, folios: None, office: None };
    for (position, cell) in cells(page, header).into_iter().enumerate() {
        let name = label_norm(&page.el(cell).text);
        if FOLIOS_HEADERS.contains(&name.as_str()) {
            found.folios = Some(position);
        } else if name.starts_with("fecha") {
            found.date = Some(position);
        } else if name.starts_with("tipo") {
            found.kind = Some(position);
        } else if name.contains("detalle") || name.contains("descripcion") {
            found.detail = Some(position);
        } else if name.starts_with("oficina") || name.starts_with("dependencia") {
            found.office = Some(position);
        }
    }
    Some(found)
}

/// "dd/mm/yyyy" inside a text, as it is written.
pub fn find_date(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    for start in 0..chars.len() {
        let rest: String = chars[start..].iter().take(10).collect();
        let b = rest.as_bytes();
        if b.len() == 10
            && b[0..2].iter().all(u8::is_ascii_digit)
            && b[2] == b'/'
            && b[3..5].iter().all(u8::is_ascii_digit)
            && b[5] == b'/'
            && b[6..10].iter().all(u8::is_ascii_digit)
            && (start == 0 || !chars[start - 1].is_ascii_digit())
        {
            return Some(rest);
        }
    }
    None
}

fn entries(page: &Page, table: &EntriesTable) -> Vec<Row> {
    let in_footer = |row: usize| page.ancestor(row, &["tfoot", "table"]).is_some_and(|a| page.el(a).tag == "tfoot");
    (table.table + 1..=page.el(table.table).end)
        .filter(|&r| page.el(r).tag == "tr" && r != table.header)
        .filter(|&r| page.ancestor(r, &["table"]) == Some(table.table) && !in_footer(r))
        .filter_map(|r| {
            let cells = cells(page, r);
            if cells.iter().any(|&c| page.el(c).tag == "th") {
                return None;
            }
            let text = |column: Option<usize>| column.and_then(|c| cells.get(c)).map(|&c| page.el(c).text.clone());
            let date = text(table.date).as_deref().and_then(find_date)?;
            Some(Row {
                date,
                kind: text(table.kind).unwrap_or_default(),
                detail: text(table.detail).unwrap_or_default(),
                folios: text(table.folios).unwrap_or_default(),
                office: text(table.office).filter(|o| !o.is_empty()),
                download: download_link(page, r),
            })
        })
        .collect()
}

/// The "Descargar" link of an entry: by its words, or by the `title` of
/// its icon.
fn download_link(page: &Page, row: usize) -> Option<DownloadLink> {
    (row + 1..=page.el(row).end)
        .filter(|&i| page.visible(i))
        .find_map(|i| {
            let el = page.el(i);
            let named = label_norm(&el.label()) == "descargar" || label_norm(&el.title()).starts_with("descargar");
            if !named {
                return None;
            }
            let target = page.clickable(i)?;
            let mark = page.el(target).mark()?.to_string();
            let href = page.el(target).attr("href").map(str::trim).filter(|h| !h.is_empty() && *h != "#" && !h.starts_with("javascript:"));
            let href = href.map(|h| resolve(&page.url, h));
            Some(DownloadLink { mark, href })
        })
}

fn resolve(base: &str, href: &str) -> String {
    tauri::Url::parse(base).and_then(|b| b.join(href)).map(|u| u.to_string()).unwrap_or_else(|_| href.to_string())
}

/// The pager of the table of entries: what follows the table up to the
/// next table (the «Notas» one has its own pager), or else what is just
/// before it. Returns the mark of "Siguiente" and the current page.
fn pager(page: &Page, table: usize) -> (Option<String>, Option<u32>) {
    let after_end = page.el(table).end + 1;
    let next_table = (after_end..page.len()).find(|&j| page.el(j).tag == "table").unwrap_or(page.len());
    let previous_table_end = (0..table)
        .filter(|&j| page.el(j).tag == "table" && !page.contains(j, table))
        .map(|j| page.el(j).end + 1)
        .max()
        .unwrap_or(0);
    for range in [table..next_table, previous_table_end..table] {
        let arrows: Vec<usize> = range
            .clone()
            .filter(|&i| page.visible(i))
            .filter(|&i| {
                let el = page.el(i);
                let name = label_norm(&el.title());
                let words = label_norm(&el.text);
                PAGER_NAMES.contains(&name.as_str()) || (matches!(el.tag.as_str(), "a" | "button") && PAGER_NAMES.contains(&words.as_str()))
            })
            .collect();
        if arrows.is_empty() {
            continue;
        }
        let next = arrows.iter().find_map(|&i| {
            let el = page.el(i);
            let named = label_norm(&el.title()) == NEXT || label_norm(&el.text) == NEXT;
            if !named {
                return None;
            }
            // A "Siguiente" that is not a link is the last page.
            let target = page.closest(i, &["a", "button"])?;
            let disabled = page.el(target).attr("disabled").is_some()
                || norm(page.el(target).attr("class").unwrap_or("")).split(' ').any(|c| c == "disabled" || c.ends_with("-dis"));
            if disabled {
                return None;
            }
            page.el(target).mark().map(str::to_string)
        });
        let current = current_page(page, arrows[0], &range);
        return (next, current);
    }
    (None, Some(1))
}

/// The current page: a number that is a `<span>`, not a link, near the
/// arrows.
fn current_page(page: &Page, arrow: usize, range: &std::ops::Range<usize>) -> Option<u32> {
    let mut container = page.el(arrow).parent;
    for _ in 0..4 {
        let Some(c) = container else { break };
        let found = (c + 1..=page.el(c).end).filter(|i| range.contains(i)).find(|&i| {
            let el = page.el(i);
            el.tag == "span"
                && !el.text.is_empty()
                && el.text.chars().all(|ch| ch.is_ascii_digit())
                && page.ancestor(i, &["a"]).is_none()
                && page.visible(i)
        });
        if let Some(found) = found {
            return page.el(found).text.parse().ok();
        }
        container = page.el(c).parent;
    }
    None
}

/// A value next to its label, before the table of entries.
fn labeled(page: &Page, names: &[&str], before: usize) -> Option<String> {
    for label in page.labels(names, 0..before) {
        let el = page.el(label);
        let sibling = el.parent.and_then(|p| page.children(p).into_iter().find(|&c| c > label));
        if let Some(value) = sibling.map(|s| page.el(s).text.clone()).filter(|t| !t.is_empty()) {
            return Some(value);
        }
        if let Some(parent) = el.parent {
            let whole = &page.el(parent).text;
            if let Some(rest) = whole.strip_prefix(el.text.as_str()) {
                let rest = rest.trim_start_matches(':').trim();
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
        }
    }
    None
}

fn record_data(page: &Page, table: usize) -> RecordData {
    let shown = labeled(page, &["expediente", "expediente nro", "expediente n°", "nro. de expediente"], table)
        .and_then(|value| find_case_keys(&value).into_iter().next());
    RecordData {
        shown,
        title: labeled(page, &["caratula"], table),
        jurisdiction: labeled(page, &["jurisdiccion"], table),
        office: labeled(page, &["dependencia"], table),
        situation: labeled(page, &["situacion actual", "situacion"], table),
    }
}

/// The related records: the table after a label that says «vinculados»,
/// if it is not the one of the entries nor comes after «Notas». Only the
/// references are kept.
fn linked(page: &Page, entries: usize) -> Vec<Linked> {
    let notes = page.labels(&["notas"], 0..page.len()).first().copied().unwrap_or(page.len());
    let Some(label) = (0..page.len()).find(|&i| {
        let el = page.el(i);
        page.visible(i) && el.text.chars().count() <= 60 && norm(&el.text).contains("vinculad") && !page.contains(entries, i)
            && page.children(i).iter().all(|&c| !norm(&page.el(c).text).contains("vinculad"))
    }) else {
        return Vec::new();
    };
    let Some(table) = (label + 1..page.len()).find(|&j| page.el(j).tag == "table" && j != entries && !page.contains(entries, j)) else {
        return Vec::new();
    };
    if table > notes || norm(&page.el(table).text).contains("cuit") {
        return Vec::new();
    }
    let mut found: Vec<Linked> = Vec::new();
    for row in (table + 1..=page.el(table).end).filter(|&r| page.el(r).tag == "tr") {
        let text = page.el(row).text.clone();
        for key in find_case_keys(&text) {
            if found.iter().any(|l| l.key == key) {
                continue;
            }
            let detail = cells(page, row)
                .into_iter()
                .map(|c| page.el(c).text.clone())
                .find(|t| !t.is_empty() && find_case_keys(t).is_empty() && t.chars().count() <= 200);
            found.push(Linked { reference: key.display(), key, detail });
        }
    }
    found
}
