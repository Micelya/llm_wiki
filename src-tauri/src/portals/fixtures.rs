//! Invented pages that copy the structure of the PJN's portal, and a
//! made-up portal that serves them, for the tests. Every name, number and
//! text here is invented.
//!
//! What they copy: the form with its list of jurisdictions and the
//! challenge before «Consultar»; the record's main data by their labels;
//! the table of entries with the column "A FS.", 15 per page, the
//! "MOVIMIENTO" rows without document and "Ver"/"Descargar" on the others;
//! a pager with a moving window of ten numbers, the current page as a
//! `<span>` and arrows that are icons named by their `title`; a second
//! paged table, «Notas», with who looked at the record; the related
//! records; the challenge present or absent; the sign «Consulta en
//! proceso», with its own «Cancelar», while the portal processes a query.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use dom_query::Document;

use super::page::{Page, Snapshot, HIDDEN, MARK};
use super::pjn::START_URL;
use super::walk::{Browser, Fetched, Progress};

pub const RECORD_URL: &str = "https://scw.pjn.gov.ar/scw/expediente.seam?cid=4242";
pub const PER_PAGE: usize = 15;
/// What the «Notas» table says: it must never be kept.
pub const NOTE_CUIT: &str = "20-99999999-7";
pub const NOTE_TEXT: &str = "Consulta de la letrada Zoraida Quimera";

#[derive(Debug, Clone)]
pub struct FakeEntry {
    pub date: String,
    pub kind: String,
    pub detail: String,
    pub folios: String,
    pub office: String,
    /// The id of its document, when it has one.
    pub document: Option<u32>,
}

/// `count` invented entries, newest first; every fourth one a MOVIMIENTO
/// without document.
pub fn entries(count: usize, first_document: u32) -> Vec<FakeEntry> {
    (0..count)
        .map(|i| {
            let n = (count - i) as u32;
            let movement = i % 4 == 3;
            FakeEntry {
                date: format!("{:02}/{:02}/20{:02}", 1 + (n % 28), 1 + (n % 12), 10 + (n % 10)),
                kind: if movement { "MOVIMIENTO".to_string() } else if i % 2 == 0 { "ESCRITO".to_string() } else { "DESPACHO".to_string() },
                detail: format!("Actuación inventada número {n}"),
                folios: format!("{} / {}", 2 * n, 2 * n + 1),
                office: "OFICINA INVENTADA 9".to_string(),
                document: if movement { None } else { Some(first_document + n) },
            }
        })
        .collect()
}

pub fn notes() -> Vec<(String, String, String)> {
    (0..20).map(|i| (format!("{:02}/10/2026", 1 + i % 20), NOTE_CUIT.to_string(), format!("{NOTE_TEXT} {i}"))).collect()
}

fn wrap(body: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><title>Consulta de Expedientes</title><script>var jsf = {{}};</script></head>
<body><div id="j_idt3" class="cabecera"><a href="/scw/home.seam">Inicio</a> <a href="https://sso.pjn.gov.ar/auth">Iniciar sesión</a></div>
<div id="j_idt7:contenido">{body}</div>
<div id="j_idt99:ingreso" style="display:none"><label>Usuario</label><input type="text" name="u"><label>Contraseña</label><input type="password" name="p"></div>
</body></html>"#
    )
}

fn challenge_box(present: bool) -> &'static str {
    if present {
        r#"<div class="captcha-pjn"><button type="button" id="j_idt40:verDesafio">Ver desafío</button></div>"#
    } else {
        ""
    }
}

pub fn form_page(challenge: bool, message: Option<&str>) -> String {
    form_page_with(challenge, message, false)
}

/// The portal's sign of a query in course, with its own «Cancelar»: always
/// in the page, shown only while a query is processed.
fn busy_sign(busy: bool) -> String {
    let hidden = if busy { "" } else { r#" style="display:none""# };
    format!(
        r#"<div id="j_idt50:procesando" class="popup-consulta"{hidden}><div class="popup-cuerpo"><span>Consulta en proceso…</span>
<button type="button" id="j_idt50:cancelar">Cancelar</button></div></div>"#
    )
}

/// The form; `busy`: while the portal processes the query, with its sign.
pub fn form_page_with(challenge: bool, message: Option<&str>, busy: bool) -> String {
    let message = message.map(|m| format!(r#"<ul class="rf-msgs"><li class="rf-msgs-err">{m}</li></ul>"#)).unwrap_or_default();
    wrap(&format!(
        r#"<h2>Consulta pública de expedientes</h2>{message}
<form id="j_idt12" action="/scw/home.seam" method="post">
<table class="formulario">
<tr><td><label for="j_idt12:juris">Jurisdicción</label></td>
<td><select id="j_idt12:juris" name="j_idt12:juris"><option value="">Seleccione…</option>
<option value="0">CSJ - Corte Suprema de Justicia de la Nación</option>
<option value="1">CIV - Cámara Nacional de Apelaciones en lo Civil</option>
<option value="7">ZZZ - Cámara Inventada de Prueba</option></select></td></tr>
<tr><td><span>Número</span></td><td><input type="text" id="j_idt12:numero" name="j_idt12:numero"></td></tr>
<tr><td><span>Año</span></td><td><input type="text" id="j_idt12:anio" name="j_idt12:anio"></td></tr>
</table>
{}
<input type="submit" id="j_idt12:consultar" value="Consultar">
</form>
{}"#,
        challenge_box(challenge),
        busy_sign(busy)
    ))
}

pub fn login_page() -> String {
    format!(
        r#"<!DOCTYPE html><html><head><title>Ingreso</title></head><body><h2>Iniciar sesión</h2>
<form><label>CUIT/CUIL</label><input type="text" name="user"><label>Contraseña</label><input type="password" name="pass">
<button type="submit">Ingresar</button></form></body></html>"#
    )
}

/// The moving window of the pager, as the portal draws it.
fn pager(current: usize, total: usize, title_on_icon: bool, id: &str) -> String {
    let arrow = |title: &str, icon: &str| {
        if title_on_icon {
            format!(r##"<a href="#" onclick="return false;" id="{id}:{icon}"><i class="fa {icon}" title="{title}"></i></a>"##)
        } else {
            format!(r##"<a href="#" onclick="return false;" id="{id}:{icon}" title="{title}"><i class="fa {icon}"></i></a>"##)
        }
    };
    let mut out = format!(r#"<div class="paginador" id="{id}">"#);
    if current > 1 {
        out.push_str(&arrow("Primera página", "fa-angle-double-left"));
        out.push_str(&arrow("Anterior", "fa-arrow-circle-o-left"));
    }
    let end = (current.saturating_sub(4).max(1) + 9).min(total);
    let start = end.saturating_sub(9).max(1);
    for n in start..=end {
        if n == current {
            out.push_str(&format!(r#"<span class="pagina-actual">{n}</span>"#));
        } else {
            out.push_str(&format!(r##"<a href="#" onclick="return false;">{n}</a>"##));
        }
    }
    if current < total {
        out.push_str(&arrow("Siguiente", "fa-arrow-circle-o-right"));
        out.push_str(&arrow("Última página", "fa-angle-double-right"));
    }
    out.push_str("</div>");
    out
}

pub struct RecordView<'a> {
    pub reference: &'a str,
    pub title: &'a str,
    pub entries: &'a [FakeEntry],
    pub page: usize,
    pub notes_page: usize,
    pub challenge: bool,
    pub title_on_icon: bool,
    pub linked: &'a [(&'a str, &'a str)],
}

pub fn record_page(view: &RecordView) -> String {
    let total = view.entries.len().div_ceil(PER_PAGE).max(1);
    let rows: String = view
        .entries
        .iter()
        .skip((view.page - 1) * PER_PAGE)
        .take(PER_PAGE)
        .map(|e| {
            let links = match e.document {
                Some(id) => format!(
                    r#"<a href="/scw/viewer.seam?id={id}&amp;cid=4242" title="Ver"><i class="fa fa-eye"></i></a> <a href="/scw/actuacion.seam?doc={id}&amp;cid=4242" title="Descargar"><i class="fa fa-download"></i></a>"#
                ),
                None => String::new(),
            };
            format!(
                "<tr><td>{}</td><td><span>{}</span></td><td>{}</td><td>{}</td><td>{}</td><td>{links}</td></tr>",
                e.office, e.date, e.kind, e.detail, e.folios
            )
        })
        .collect();
    let notes = notes();
    let notes_total = notes.len().div_ceil(5);
    let note_rows: String = notes
        .iter()
        .skip((view.notes_page - 1) * 5)
        .take(5)
        .map(|(date, cuit, text)| format!("<tr><td>{date}</td><td>{cuit}</td><td>{text}</td></tr>"))
        .collect();
    let linked = if view.linked.is_empty() {
        String::new()
    } else {
        let rows: String = view.linked.iter().map(|(r, d)| format!("<tr><td>{r}</td><td>{d}</td></tr>")).collect();
        format!(r#"<fieldset><legend>Expedientes vinculados</legend><table class="vinculados">{rows}</table></fieldset>"#)
    };
    wrap(&format!(
        r#"<div class="datos-expediente">
<div class="fila"><label>Expediente:</label> <span>{reference}</span></div>
<div class="fila"><label>Jurisdicción:</label> <span>Cámara Inventada de Prueba</span></div>
<div class="fila"><label>Dependencia:</label> <span>JUZGADO INVENTADO 99</span></div>
<div class="fila"><label>Situación actual:</label> <span>EN LETRA</span></div>
<div class="fila"><label>Carátula:</label> <span>{title}</span></div>
</div>
{challenge}
<h3>Actuaciones</h3>
<table class="table" id="expediente:action-table">
<thead><tr><th>OFICINA</th><th>FECHA</th><th>TIPO</th><th>DESCRIPCION/DETALLE</th><th>A FS.</th><th></th></tr></thead>
<tbody>{rows}</tbody>
</table>
{pager}
<a href="/scw/historicas.seam">Ver históricas</a>
{linked}
<h3>Notas</h3>
<table class="table" id="expediente:notas-table"><thead><tr><th>FECHA</th><th>CUIT</th><th>NOTA</th></tr></thead><tbody>{note_rows}</tbody></table>
{notes_pager}"#,
        reference = view.reference,
        title = view.title,
        challenge = challenge_box(view.challenge),
        pager = pager(view.page, total, view.title_on_icon, "expediente:action-pager"),
        notes_pager = pager(view.notes_page, notes_total, view.title_on_icon, "expediente:notas-pager"),
    ))
}

/// Marks the page as the snapshot script does: the elements the app may
/// act on, and the ones a person cannot see.
pub fn mark(html: &str, generation: u32, act: bool) -> String {
    let document = Document::from(html);
    for node in document.select("[style]").nodes() {
        if node.attr("style").is_some_and(|s| s.replace(' ', "").contains("display:none")) {
            node.set_attr(HIDDEN, "1");
            for inner in node.descendants() {
                if inner.is_element() {
                    inner.set_attr(HIDDEN, "1");
                }
            }
        }
    }
    if act {
        let marked = document.select(r#"a, button, input, select, textarea, [onclick], [role="button"]"#);
        for (i, node) in marked.nodes().iter().enumerate() {
            node.set_attr(MARK, &format!("{generation}-{i}"));
        }
    }
    document.html().to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub enum FakeScreen {
    Blank,
    Form,
    Login,
    Record,
}

/// A portal made up from the invented pages, and a person who solves its
/// challenge after a few looks.
pub struct FakePortal {
    pub reference: String,
    pub title: String,
    pub entries: Vec<FakeEntry>,
    pub linked: Vec<(&'static str, &'static str)>,
    pub screen: FakeScreen,
    pub page: usize,
    pub notes_page: usize,
    pub form_challenge: bool,
    /// The challenge is there but the page does not show it as the
    /// connector knows it (only a person sees it).
    pub unseen_challenge: bool,
    /// The record shows the challenge when the walk gets to this page.
    pub challenge_at_page: Option<usize>,
    pub record_challenge: bool,
    pub login_first: bool,
    pub title_on_icon: bool,
    /// The person solves what is asked after this many looks.
    pub person_after: usize,
    looks: usize,
    /// After «Consultar» (the app's or the person's), the portal shows its
    /// sign «Consulta en proceso» for this many looks (`usize::MAX`: it
    /// never ends), then the page loads for `query_loading` looks, and
    /// then the record is there.
    pub query_busy: usize,
    pub query_loading: usize,
    busy_left: usize,
    loading_left: usize,
    /// The person presses the portal's «Cancelar» the first time they see
    /// the sign with the view free: the form comes back with its
    /// challenge.
    pub cancel_while_busy: bool,
    /// Whether the view was locked at each look at the sign.
    pub busy_locks: Vec<bool>,
    pub locked: bool,
    pub lock_log: Vec<bool>,
    /// The state of the walk each time the view was unlocked.
    pub unlocked_while: Vec<String>,
    pub presses: Vec<String>,
    pub fills: Vec<String>,
    pub downloads: Vec<u32>,
    /// Documents that come back as a page, not a PDF.
    pub not_pdf: HashSet<u32>,
    /// The walk is stopped after this many downloads.
    pub stop_after: Option<usize>,
    pub reports: Vec<Progress>,
    generation: u32,
    last: Option<Page>,
}

impl FakePortal {
    pub fn new(reference: &str, title: &str, entries: Vec<FakeEntry>) -> FakePortal {
        FakePortal {
            reference: reference.to_string(),
            title: title.to_string(),
            entries,
            linked: Vec::new(),
            screen: FakeScreen::Blank,
            page: 1,
            notes_page: 1,
            form_challenge: false,
            unseen_challenge: false,
            challenge_at_page: None,
            record_challenge: false,
            login_first: false,
            title_on_icon: false,
            person_after: 2,
            looks: 0,
            query_busy: 0,
            query_loading: 0,
            busy_left: 0,
            loading_left: 0,
            cancel_while_busy: false,
            busy_locks: Vec::new(),
            locked: true,
            lock_log: Vec::new(),
            unlocked_while: Vec::new(),
            presses: Vec::new(),
            fills: Vec::new(),
            downloads: Vec::new(),
            not_pdf: HashSet::new(),
            stop_after: None,
            reports: Vec::new(),
            generation: 0,
            last: None,
        }
    }

    pub fn total_pages(&self) -> usize {
        self.entries.len().div_ceil(PER_PAGE).max(1)
    }

    fn html(&self) -> (String, String) {
        match self.screen {
            FakeScreen::Blank => ("about:blank".to_string(), "<html><head></head><body></body></html>".to_string()),
            FakeScreen::Form => (START_URL.to_string(), form_page(self.form_challenge && !self.unseen_challenge, None)),
            FakeScreen::Login => ("https://sso.pjn.gov.ar/auth".to_string(), login_page()),
            FakeScreen::Record => (
                RECORD_URL.to_string(),
                record_page(&RecordView {
                    reference: &self.reference,
                    title: &self.title,
                    entries: &self.entries,
                    page: self.page,
                    notes_page: self.notes_page,
                    challenge: self.record_challenge,
                    title_on_icon: self.title_on_icon,
                    linked: &self.linked,
                }),
            ),
        }
    }

    /// Whether a query is in course: the sign shows, or the page loads.
    fn in_course(&self) -> bool {
        self.busy_left > 0 || self.loading_left > 0
    }

    /// «Consultar» with nothing left to solve: the portal processes the
    /// query, and then shows the record.
    fn consult(&mut self) {
        self.busy_left = self.query_busy;
        self.loading_left = self.query_loading;
        if !self.in_course() {
            self.arrive();
        }
    }

    fn arrive(&mut self) {
        self.screen = FakeScreen::Record;
        self.page = 1;
    }

    /// What the person does while the view is theirs.
    fn person(&mut self) {
        if self.locked {
            return;
        }
        if self.busy_left > 0 && self.cancel_while_busy {
            self.cancel_while_busy = false;
            self.presses.push("persona: Cancelar".to_string());
            self.busy_left = 0;
            self.loading_left = 0;
            self.form_challenge = true;
            self.looks = 0;
            return;
        }
        if self.in_course() {
            return;
        }
        self.looks += 1;
        if self.looks < self.person_after {
            return;
        }
        self.looks = 0;
        match self.screen {
            FakeScreen::Login => {
                self.login_first = false;
                self.screen = FakeScreen::Form;
            }
            FakeScreen::Form if self.form_challenge => {
                // Solves the challenge and presses «Consultar».
                self.form_challenge = false;
                self.presses.push("persona: Consultar".to_string());
                self.consult();
            }
            FakeScreen::Record if self.record_challenge => {
                self.record_challenge = false;
            }
            _ => {}
        }
    }

    /// What the element with this mark is, in the last marked page.
    fn element(&self, mark: &str) -> Option<(usize, Page)> {
        let page = self.last.clone()?;
        let index = page.els.iter().position(|e| e.mark() == Some(mark))?;
        Some((index, page))
    }
}

impl Browser for FakePortal {
    fn navigate(&mut self, url: &str) -> Result<(), String> {
        assert_eq!(url, START_URL, "the walk only goes to the portal's form");
        self.screen = if self.login_first { FakeScreen::Login } else { FakeScreen::Form };
        Ok(())
    }

    fn snapshot(&mut self, act: bool) -> Result<Snapshot, String> {
        if !act {
            self.person();
        }
        if self.busy_left > 0 {
            self.busy_locks.push(self.locked);
            self.busy_left -= 1;
            let html = form_page_with(false, None, true);
            if !self.in_course() {
                self.arrive();
            }
            self.generation += 1;
            let marked = mark(&html, self.generation, act);
            if act {
                self.last = Some(Page::parse(START_URL, &marked));
            }
            return Ok(Snapshot { url: START_URL.to_string(), ready: "complete".to_string(), html: marked, error: None });
        }
        if self.loading_left > 0 {
            self.loading_left -= 1;
            if !self.in_course() {
                self.arrive();
            }
            return Ok(Snapshot { url: START_URL.to_string(), ready: "loading".to_string(), html: String::new(), error: None });
        }
        let (url, html) = self.html();
        self.generation += 1;
        let marked = mark(&html, self.generation, act);
        if act {
            self.last = Some(Page::parse(&url, &marked));
        }
        Ok(Snapshot { url, ready: "complete".to_string(), html: marked, error: None })
    }

    fn click(&mut self, mark: &str) -> Result<bool, String> {
        let Some((index, page)) = self.element(mark) else { return Ok(false) };
        let el = page.el(index);
        let notes_start = page.els.iter().position(|e| e.attr("id") == Some("expediente:notas-table")).unwrap_or(usize::MAX);
        let named = |name: &str| el.title() == name || page.children(index).iter().any(|&c| page.el(c).title() == name);
        if el.label() == "Consultar" {
            if self.in_course() {
                self.presses.push("Consultar sobre una consulta en curso".to_string());
                return Ok(true);
            }
            self.presses.push("Consultar".to_string());
            if self.form_challenge {
                return Ok(true);
            }
            self.consult();
        } else if named("Siguiente") && index > notes_start {
            self.presses.push("Siguiente de Notas".to_string());
            self.notes_page += 1;
        } else if named("Siguiente") {
            self.presses.push("Siguiente".to_string());
            if self.page < self.total_pages() {
                self.page += 1;
                if self.challenge_at_page == Some(self.page) {
                    self.challenge_at_page = None;
                    self.record_challenge = true;
                }
            }
        } else {
            self.presses.push(format!("otro: {} {}", el.tag, el.text));
        }
        Ok(true)
    }

    fn fill(&mut self, mark: &str, value: &str) -> Result<bool, String> {
        let Some((index, page)) = self.element(mark) else { return Ok(false) };
        self.fills.push(format!("{}={value}", page.el(index).attr("id").unwrap_or("?")));
        Ok(true)
    }

    fn download(&mut self, mark: &str, destination: &Path) -> Result<Fetched, String> {
        let Some((index, page)) = self.element(mark) else { return Ok(Fetched::Missing) };
        let href = page.el(index).attr("href").unwrap_or("").to_string();
        let id: u32 = href.split("doc=").nth(1).and_then(|r| r.split('&').next()).and_then(|n| n.parse().ok()).ok_or("not a download link")?;
        self.downloads.push(id);
        let body = if self.not_pdf.contains(&id) {
            b"<html><body>Sesion vencida</body></html>".to_vec()
        } else {
            let mut bytes = format!("%PDF-1.4\n% documento inventado {id}\n").into_bytes();
            bytes.extend(std::iter::repeat_n(b'x', 100 + id as usize));
            bytes
        };
        fs::write(destination, body).map_err(|e| e.to_string())?;
        Ok(Fetched::Done)
    }

    fn lock(&mut self, locked: bool, _note: &str) {
        if !locked && self.locked {
            self.unlocked_while.push(self.reports.last().map(|r| r.state.clone()).unwrap_or_default());
        }
        if self.lock_log.last() != Some(&locked) {
            self.lock_log.push(locked);
        }
        self.locked = locked;
    }

    fn wait(&mut self, _time: Duration) {}

    fn cancelled(&self) -> bool {
        self.stop_after.is_some_and(|n| self.downloads.len() >= n)
    }

    fn take_go_on(&mut self) -> bool {
        false
    }

    fn report(&mut self, progress: &Progress) {
        self.reports.push(progress.clone());
    }
}
