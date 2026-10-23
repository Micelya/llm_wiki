//! The portals with invented pages and an invented portal (`fixtures.rs`):
//! the reading of each screen, the walk (everything new is downloaded,
//! the repeated is skipped, «Notas» is never kept), the stop and unlock at
//! the challenge, resuming, the names and the data, the base.

use std::fs;
use std::path::Path;

use super::fixtures::{self, entries, FakePortal, FakeScreen, RecordView, NOTE_CUIT, NOTE_TEXT};
use super::page::Page;
use super::pjn::{Pjn, START_URL};
use super::store::{self, state, Entry, EntryKey, RecordFile};
use super::walk::{Stop, Walk, QUERY_IN_COURSE, QUERY_STUCK};
use super::{find_case_keys, is_app_webview, navigation, CaseKey, Connector, Navigation, Screen, Upgrades};
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;

const REFERENCE: &str = "ZZZ 004321/2011";
const TITLE: &str = "QUIMERA, ZORAIDA C/ ONDARRETA, FABRIZIA S/ COBRO DE PESOS INVENTADO";

fn key() -> CaseKey {
    CaseKey::new("ZZZ", "4321", "2011").unwrap()
}

fn marked(html: &str, url: &str) -> Page {
    Page::parse(url, &fixtures::mark(html, 1, true))
}

fn record_view<'a>(list: &'a [fixtures::FakeEntry], page: usize, challenge: bool, title_on_icon: bool) -> RecordView<'a> {
    RecordView { reference: REFERENCE, title: TITLE, entries: list, page, notes_page: 1, challenge, title_on_icon, linked: &[("ZZZ 4322/2011", "INCIDENTE")] }
}

fn walk_with<'a>(folder: &Path, known: &'a dyn store::KnownFiles) -> Walk<'a> {
    let mut walk = Walk::new(&Pjn, folder.join("PJN"), known);
    walk.pause = std::time::Duration::ZERO;
    walk.poll = std::time::Duration::ZERO;
    walk.person_poll = std::time::Duration::ZERO;
    walk.looks = 5;
    walk
}

fn all_text_in(folder: &Path) -> String {
    let mut out = String::new();
    for entry in walkdir::WalkDir::new(folder).into_iter().flatten() {
        if entry.file_type().is_file() {
            out.push_str(&String::from_utf8_lossy(&fs::read(entry.path()).unwrap()));
        }
    }
    out
}

fn pdfs_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".pdf")).collect())
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn the_record_page_is_read_by_its_labels_from_the_table_of_entries() {
    let list = entries(38, 100);
    for title_on_icon in [false, true] {
        let page = marked(&fixtures::record_page(&record_view(&list, 2, false, title_on_icon)), fixtures::RECORD_URL);
        assert_eq!(Pjn.screen(&page), Screen::Record);
        let record = Pjn.record(&page);
        assert_eq!(record.rows.len(), 15);
        assert_eq!(record.page_number, Some(2));
        assert_eq!(record.data.shown, Some(key()));
        assert_eq!(record.data.title.as_deref(), Some(TITLE));
        assert_eq!(record.data.office.as_deref(), Some("JUZGADO INVENTADO 99"));
        assert_eq!(record.data.situation.as_deref(), Some("EN LETRA"));
        // The rows of the table of entries, never those of «Notas».
        let first = &record.rows[0];
        assert_eq!(first.date, list[15].date);
        assert_eq!(first.folios, list[15].folios);
        assert_eq!(first.office.as_deref(), Some("OFICINA INVENTADA 9"));
        assert!(record.rows.iter().all(|r| !r.detail.contains("Zoraida") && !r.folios.contains(NOTE_CUIT)));
        // A MOVIMIENTO has no document; the others, their "Descargar".
        for (row, fake) in record.rows.iter().zip(&list[15..30]) {
            assert_eq!(row.download.is_some(), fake.document.is_some(), "{}", row.kind);
            if let (Some(link), Some(id)) = (&row.download, fake.document) {
                assert_eq!(link.href.as_deref(), Some(format!("https://scw.pjn.gov.ar/scw/actuacion.seam?doc={id}&cid=4242").as_str()));
            }
        }
        // "Siguiente" is the one of the table of entries.
        let next = record.next.expect("a next page");
        let index = page.els.iter().position(|e| e.mark() == Some(next.as_str())).unwrap();
        let pager = page.ancestor(index, &["div"]).unwrap();
        assert_eq!(page.el(pager).attr("id"), Some("expediente:action-pager"));
        assert_eq!(record.linked.len(), 1);
        assert_eq!(record.linked[0].key, CaseKey::new("ZZZ", "4322", "2011").unwrap());
    }
    // The last page has no "Siguiente" (the one of «Notas» does not count).
    let page = marked(&fixtures::record_page(&record_view(&list, 3, false, false)), fixtures::RECORD_URL);
    let record = Pjn.record(&page);
    assert_eq!(record.rows.len(), 8);
    assert_eq!(record.next, None);
    assert_eq!(record.page_number, Some(3));
}

/// The day of an invented date has one digit ("7/11/2025").
fn short_day(date: &str) -> bool {
    date.split('/').next().is_some_and(|day| day.len() == 1)
}

#[test]
fn the_page_of_the_scw_is_read_whole_with_its_dates_without_a_zero() {
    let list = fixtures::scw_entries(40, 700);
    // The first page has rows whose day has one digit, as the portal
    // writes it: the reader of before left them out.
    assert!(list[..15].iter().filter(|e| short_day(&e.date)).count() >= 4);
    let page = marked(&fixtures::scw_record_page(&record_view(&list, 1, false, false)), fixtures::RECORD_URL);
    assert_eq!(Pjn.screen(&page), Screen::Record);
    assert!(!Pjn.busy(&page), "the hidden signs of «Consulta en proceso» are not the sign");
    let record = Pjn.record(&page);
    assert_eq!(record.rows.len(), 15);
    assert_eq!(record.unread, super::Unread::default());
    assert_eq!(record.page_number, Some(1));
    assert!(record.next.is_some());
    // The main data, each label in a box of its own.
    assert_eq!(record.data.shown, Some(key()));
    assert_eq!(record.data.title.as_deref(), Some(TITLE));
    assert_eq!(record.data.jurisdiction.as_deref(), Some("Cámara Inventada de Prueba"));
    assert_eq!(record.data.office.as_deref(), Some("JUZGADO INVENTADO 99 - SECRETARIA Nº 98"));
    assert_eq!(record.data.situation.as_deref(), Some("EN LETRA"));
    for (row, fake) in record.rows.iter().zip(&list[..15]) {
        // Without the labels only a screen reader says.
        assert_eq!(row.kind, fake.kind);
        assert_eq!(row.detail, fake.detail);
        assert_eq!(row.folios, fake.folios);
        assert_eq!(row.office.as_deref(), Some("099"));
        // The date, with its zeros.
        assert_eq!(store::canonical_date(&row.date), store::canonical_date(&fake.date));
        assert_eq!(row.date.len(), 10, "{}", row.date);
        // «Descargar» in the first cell, never «Ver».
        assert_eq!(row.download.is_some(), fake.document.is_some(), "{} {}", fake.kind, fake.date);
        if let (Some(link), Some(id)) = (&row.download, fake.document) {
            let href = link.href.as_deref().unwrap();
            assert!(href.contains(&format!("id=doc-{id}-")) && href.ends_with("&download=true"), "{href}");
        }
    }
    let with_document = record.rows.iter().filter(|r| r.download.is_some()).count();
    assert_eq!(with_document, 9);
    // The last page: no «Siguiente».
    let page = marked(&fixtures::scw_record_page(&record_view(&list, 3, false, false)), fixtures::RECORD_URL);
    let record = Pjn.record(&page);
    assert_eq!((record.rows.len(), record.next.is_none(), record.page_number), (10, true, Some(3)));
}

#[test]
fn a_row_that_cannot_be_read_is_told_never_left_out_in_silence() {
    let mut list = fixtures::scw_entries(20, 700);
    list[2].date = "s/f".to_string();
    assert!(list[2].document.is_some());
    let page = marked(&fixtures::scw_record_page(&record_view(&list, 1, false, false)), fixtures::RECORD_URL);
    let record = Pjn.record(&page);
    assert_eq!(record.rows.len(), 14);
    assert_eq!(record.unread, super::Unread { rows: 1, with_document: 1 });
    let dir = TempDir::new();
    let mut portal = FakePortal::new(REFERENCE, TITLE, list);
    portal.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    assert_eq!(
        outcome.warnings,
        vec!["La página 1 tiene 1 fila que la app no pudo leer como actuaciones, 1 con documento: no se trajeron. Mirala en el portal y avisá."]
    );
}

#[test]
fn dates_are_found_with_one_or_two_digits_and_written_with_two() {
    use super::pjn::{find_date, without_label};
    for (text, date) in [
        ("7/11/2025", Some("07/11/2025")),
        ("Fecha: 3/10/2025", Some("03/10/2025")),
        ("14/11/2025", Some("14/11/2025")),
        ("05/3/2024", Some("05/03/2024")),
        ("INFORME [Presentado 11/11/2025 14:01]", Some("11/11/2025")),
        ("123/11/2025", None),
        ("1/13/2025", None),
        ("5/3/20251", None),
        ("5/3/25", None),
        ("sin fecha", None),
    ] {
        assert_eq!(find_date(text).as_deref(), date, "{text}");
    }
    assert_eq!(without_label("Tipo actuacion: FIRMA DESPACHO"), "FIRMA DESPACHO");
    assert_eq!(without_label("Detalle: EN LETRA"), "EN LETRA");
    assert_eq!(without_label("Oficina: 099"), "099");
    assert_eq!(without_label("OFICIO: AL REGISTRO INVENTADO"), "OFICIO: AL REGISTRO INVENTADO");
    assert_eq!(without_label("ESCRITO AGREGADO"), "ESCRITO AGREGADO");
}

#[test]
fn the_walk_over_the_scw_brings_every_entry_and_every_document() {
    let dir = TempDir::new();
    let list = fixtures::scw_entries(40, 700);
    let documents: Vec<u32> = list.iter().filter_map(|e| e.document).collect();
    let mut portal = FakePortal::new(REFERENCE, TITLE, list.clone());
    portal.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    assert_eq!(outcome.seen, 40);
    assert_eq!(outcome.with_document, documents.len());
    let mut got = portal.downloads.clone();
    got.sort();
    let mut want = documents.clone();
    want.sort();
    assert_eq!(got, want);
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    assert_eq!(portal.presses, vec!["Consultar", "Siguiente", "Siguiente"]);
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    let record = store::load(&folder).unwrap().unwrap();
    assert_eq!(record.data.title.as_deref(), Some(TITLE));
    assert_eq!(record.entries.len(), 40);
    // Named without the screen reader's labels.
    assert!(pdfs_in(&folder).iter().all(|n| !n.to_uppercase().contains("TIPO-ACTUACION") && !n.contains("Detalle")), "{:?}", pdfs_in(&folder));
}

#[test]
fn the_progress_says_the_page_what_is_left_of_it_and_the_document_in_course() {
    let dir = TempDir::new();
    let list = fixtures::scw_entries(20, 700);
    let mut portal = FakePortal::new(REFERENCE, TITLE, list);
    portal.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    let downloading: Vec<_> = portal.reports.iter().filter(|r| r.message.starts_with("Bajando ")).collect();
    assert_eq!(downloading.len(), outcome.downloaded);
    // The first document of page 1: its 9 documents are still to look at,
    // and the one in course is named.
    let first = downloading[0];
    assert_eq!((first.page, first.seen, first.with_document, first.pending, first.downloaded), (Some(1), 15, 9, 9, 0));
    assert_eq!(first.current.as_deref(), first.message.strip_prefix("Bajando "));
    // The last of page 1, and the first of page 2.
    assert_eq!((downloading[8].pending, downloading[8].downloaded), (1, 8));
    assert_eq!((downloading[9].page, downloading[9].seen, downloading[9].pending), (Some(2), 20, 3));
    assert_eq!((outcome.pending, outcome.current.as_deref()), (0, None));
}

/// What the reader of before kept of a record of the SCW: only the rows
/// whose day has two digits, with the labels of the screen reader in
/// their kind, detail and office, and their documents downloaded with
/// those names.
fn kept_by_the_old_reader(folder: &Path, list: &[fixtures::FakeEntry]) -> Vec<String> {
    let mut record = RecordFile::new("pjn", &key());
    let mut names = Vec::new();
    for fake in list.iter().filter(|e| !short_day(&e.date)) {
        let mut entry = Entry {
            date: fake.date.clone(),
            kind: format!("Tipo actuacion: {}", fake.kind),
            detail: format!("Detalle: {}", fake.detail),
            folios: fake.folios.clone(),
            office: Some(format!("Oficina: {}", fake.office)),
            has_document: fake.document.is_some(),
            state: state::NO_DOCUMENT.to_string(),
            ..Entry::default()
        };
        if fake.document.is_some() {
            let name = store::document_name(&key(), &entry);
            let body = format!("%PDF-1.4 bajado antes {name}");
            fs::create_dir_all(folder).unwrap();
            fs::write(folder.join(&name), &body).unwrap();
            entry.state = state::DOWNLOADED.to_string();
            entry.size = Some(body.len() as u64);
            entry.file = Some(name.clone());
            names.push(name);
        }
        record.entries.push(entry);
    }
    record.updated_at = Some(store::now_text());
    store::save(folder, &record).unwrap();
    names
}

#[test]
fn an_update_brings_what_the_old_reader_left_out_and_downloads_nothing_twice() {
    let dir = TempDir::new();
    let list = fixtures::scw_entries(40, 700);
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    let before = kept_by_the_old_reader(&folder, &list);
    assert!(before.iter().all(|n| n.contains("TIPO-ACTUACION")), "{before:?}");
    let left_out: Vec<u32> = list.iter().filter(|e| short_day(&e.date)).filter_map(|e| e.document).collect();
    assert!(!left_out.is_empty() && !before.is_empty());
    let mut portal = FakePortal::new(REFERENCE, TITLE, list.clone());
    portal.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    // Only what was left out is downloaded.
    let mut got = portal.downloads.clone();
    got.sort();
    let mut want = left_out.clone();
    want.sort();
    assert_eq!(got, want);
    assert_eq!(outcome.downloaded, left_out.len());
    assert_eq!(outcome.already, before.len());
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    // Every entry once, read as now; the files of before keep their names.
    let record = store::load(&folder).unwrap().unwrap();
    assert_eq!(record.entries.len(), 40);
    assert!(record.entries.iter().all(|e| !e.kind.starts_with("Tipo") && !e.detail.starts_with("Detalle")));
    assert_eq!(record.entries.iter().filter(|e| e.office.as_deref() == Some("099")).count(), 40);
    let names = pdfs_in(&folder);
    assert_eq!(names.len(), before.len() + left_out.len());
    assert!(before.iter().all(|n| names.contains(n)));
    // And again: nothing.
    let mut again = FakePortal::new(REFERENCE, TITLE, list);
    again.scw = true;
    walk_with(dir.path(), &store::NoneKnown).run(&mut again, &key()).unwrap();
    assert!(again.downloads.is_empty(), "{:?}", again.downloads);
}

#[test]
fn each_screen_is_told_by_what_a_person_reads() {
    let list = entries(5, 1);
    let screen = |html: String| Pjn.screen(&marked(&html, START_URL));
    assert_eq!(screen(fixtures::form_page(false, None)), Screen::Form { challenge: false });
    assert_eq!(screen(fixtures::form_page(true, None)), Screen::Form { challenge: true });
    assert_eq!(screen(fixtures::record_page(&record_view(&list, 1, false, false))), Screen::Record);
    assert_eq!(screen(fixtures::record_page(&record_view(&list, 1, true, false))), Screen::Challenge);
    assert_eq!(screen(fixtures::login_page()), Screen::Login);
    // The hidden sign-in box of every page is not a request to sign in.
    assert!(fixtures::form_page(false, None).contains(r#"type="password""#));
    assert_eq!(screen("<html><body><p>Mantenimiento</p></body></html>".to_string()), Screen::Other);
}

#[test]
fn the_form_is_filled_by_its_labels_with_the_option_of_the_code() {
    let page = marked(&fixtures::form_page(true, None), START_URL);
    let plan = Pjn.form(&page, &key()).unwrap();
    assert!(plan.challenge);
    let id = |mark: &str| page.els.iter().find(|e| e.mark() == Some(mark)).and_then(|e| e.attr("id")).unwrap().to_string();
    let fills: Vec<(String, String)> = plan.fills.iter().map(|(m, v)| (id(m), v.clone())).collect();
    assert_eq!(
        fills,
        vec![
            ("j_idt12:juris".to_string(), "7".to_string()),
            ("j_idt12:numero".to_string(), "4321".to_string()),
            ("j_idt12:anio".to_string(), "2011".to_string()),
        ]
    );
    assert_eq!(id(plan.submit.as_deref().unwrap()), "j_idt12:consultar");
    let unknown = CaseKey::new("QQQ", "1", "2020").unwrap();
    assert!(Pjn.form(&page, &unknown).unwrap_err().contains("QQQ"));
}

#[test]
fn the_walk_downloads_everything_new_and_never_keeps_the_notes() {
    let dir = TempDir::new();
    let list = entries(38, 500);
    let documents: Vec<u32> = list.iter().filter_map(|e| e.document).collect();
    let mut portal = FakePortal::new(REFERENCE, TITLE, list.clone());
    portal.linked = vec![("ZZZ 4322/2011", "INCIDENTE")];
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    // Every document, once, one after the other.
    let mut downloaded = portal.downloads.clone();
    downloaded.sort();
    let mut expected = documents.clone();
    expected.sort();
    assert_eq!(downloaded, expected);
    assert_eq!(outcome.downloaded, documents.len());
    assert_eq!(outcome.seen, 38);
    assert_eq!(outcome.with_document, documents.len());
    assert_eq!(outcome.failed, 0);
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    // It pressed the form's «Consultar» and "Siguiente" of the entries,
    // never the pager of «Notas» nor a page by its number.
    assert_eq!(portal.presses, vec!["Consultar", "Siguiente", "Siguiente"]);
    assert!(portal.fills.contains(&"j_idt12:numero=4321".to_string()));
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    assert_eq!(pdfs_in(&folder).len(), documents.len());
    // Nothing of «Notas» is kept anywhere.
    let kept = all_text_in(dir.path());
    assert!(!kept.contains(NOTE_CUIT));
    assert!(!kept.contains(NOTE_TEXT));
    // The record's file.
    let record = store::load(&folder).unwrap().unwrap();
    assert_eq!(record.data.title.as_deref(), Some(TITLE));
    assert_eq!(record.entries.len(), 38);
    assert_eq!(record.count(state::DOWNLOADED), documents.len());
    assert_eq!(record.count(state::NO_DOCUMENT), 38 - documents.len());
    assert_eq!(record.linked.len(), 1);
    assert!(record.updated_at.is_some());
    // The staging folder holds nothing left.
    assert_eq!(fs::read_dir(dir.path().join("PJN").join(store::STAGING)).unwrap().count(), 0);
}

#[test]
fn each_document_is_named_by_the_record_and_has_its_data_next_to_it() {
    let dir = TempDir::new();
    let list = entries(3, 7);
    let mut portal = FakePortal::new(REFERENCE, TITLE, list.clone());
    walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    let first = &list[0];
    assert_eq!((first.date.as_str(), first.folios.as_str()), ("04/04/2013", "6 / 7"));
    let name = "ZZZ4321-2011_fs6-7_2013-04-04_ESCRITO_Actuacion-inventada-numero-3.pdf";
    let names = pdfs_in(&folder);
    assert!(names.contains(&name.to_string()), "{names:?}");
    let pdf = folder.join(name);
    let data: serde_json::Value = serde_json::from_str(&fs::read_to_string(store::data_path(&pdf)).unwrap()).unwrap();
    assert_eq!(data["portal"], "PJN");
    assert_eq!(data["expediente"], "ZZZ 4321/2011");
    assert_eq!(data["fecha"], first.date.as_str());
    assert_eq!(data["fojas"], first.folios.as_str());
    assert_eq!(data["tipo"], "ESCRITO");
    assert_eq!(data["detalle"], first.detail.as_str());
    assert_eq!(data["oficina"], "OFICINA INVENTADA 9");
    assert!(data["direccion_origen"].as_str().unwrap().starts_with("https://scw.pjn.gov.ar/scw/actuacion.seam?doc="));
    assert!(data["fecha_descarga"].as_str().unwrap().starts_with("20"));
    assert_eq!(data["tamano"].as_u64(), Some(fs::metadata(&pdf).unwrap().len()));
}

#[test]
fn names_dates_and_folios() {
    let entry = Entry {
        date: "05/03/2020".to_string(),
        kind: "AVISO ELECTRÓNICO".to_string(),
        detail: "Notificación: traslado de demanda (inventada)".to_string(),
        folios: "146 / 148".to_string(),
        ..Entry::default()
    };
    let name = store::document_name(&key(), &entry);
    assert_eq!(name, "ZZZ4321-2011_fs146-148_2020-03-05_AVISO-ELECTRONICO_Notificacion-traslado-de-demanda-inventada.pdf");
    let parsed = store::parse_document_name(&name).unwrap();
    assert_eq!((parsed.jurisdiction.as_str(), parsed.number, parsed.year), ("ZZZ", 4321, 2011));
    assert_eq!(parsed.entry.folios, vec![146, 148]);
    assert_eq!(parsed.entry.date, "20200305");
    assert_eq!(parsed.entry.kind, "AVISOELECTRONICO");
    assert_eq!(parsed.entry.kind, entry.key().kind);
    for written in ["05/03/2020", "2020-03-05", "20200305", "05032020"] {
        assert_eq!(store::canonical_date(written).as_deref(), Some("20200305"), "{written}");
    }
    assert_eq!(store::canonical_date("sin fecha"), None);
    let dir = TempDir::new();
    dir.write("a.pdf", b"x");
    assert_eq!(store::free_name(dir.path(), "a.pdf", &[]), "a_2.pdf");
    assert_eq!(store::free_name(dir.path(), "b.pdf", &["B.pdf".to_string()]), "b_2.pdf");
    assert_eq!(store::free_name(dir.path(), "c.pdf", &[]), "c.pdf");
    assert_eq!(key().display(), "ZZZ 4321/2011");
    assert_eq!(key().folder_name(), "ZZZ 4321-2011");
    assert_eq!(find_case_keys("Vinculado: ZZZ 004321/2011 y QQ 7/2020; no 12/2020 ni ABCDE 1/2020"), vec![key(), CaseKey::new("QQ", "7", "2020").unwrap()]);
    assert!(CaseKey::new("ZZZ", "12a", "2011").is_err());
    assert!(CaseKey::new("", "12", "2011").is_err());
    assert!(CaseKey::new("ZZZ", "12", "11").is_err());
}

#[test]
fn the_challenge_stops_the_app_unlocks_the_view_and_the_walk_goes_on() {
    let dir = TempDir::new();
    let list = entries(20, 900);
    let mut portal = FakePortal::new(REFERENCE, TITLE, list.clone());
    portal.form_challenge = true;
    portal.challenge_at_page = Some(2);
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    // Unlocked twice, each time for a person, and locked again after.
    assert_eq!(portal.unlocked_while, vec!["needs-person", "needs-person"]);
    assert_eq!(portal.lock_log, vec![true, false, true, false, true]);
    assert!(portal.locked);
    // Each time, the focus on the challenge, and off after.
    assert_eq!(portal.focus_log, vec![true, false, true, false]);
    assert_eq!(portal.focused_on, vec![FakeScreen::Form, FakeScreen::Record]);
    assert!(!portal.focused);
    // While it was the person's turn, the app pressed nothing: the person
    // pressed «Consultar».
    assert_eq!(portal.presses, vec!["persona: Consultar", "Siguiente"]);
    let expected = list.iter().filter(|e| e.document.is_some()).count();
    assert_eq!(outcome.downloaded, expected);
    assert!(portal.reports.iter().any(|r| r.state == "needs-person" && r.person.as_deref() == Some("challenge")));
}

#[test]
fn a_form_that_does_not_move_on_is_handed_to_the_person() {
    let dir = TempDir::new();
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(4, 1));
    portal.form_challenge = true;
    portal.unseen_challenge = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    // The app pressed «Consultar» once; the form did not move on, so the
    // view went to the person, who did what the portal asked.
    assert_eq!(portal.presses, vec!["Consultar", "persona: Consultar"]);
    assert_eq!(portal.unlocked_while, vec!["needs-person"]);
    assert_eq!(outcome.downloaded, 3);
}

#[test]
fn the_sign_of_a_query_in_course_is_read_only_while_it_shows() {
    let page = |html: String| marked(&html, START_URL);
    let busy = page(fixtures::form_page_with(false, None, true));
    assert!(Pjn.busy(&busy));
    // While it shows, the page is still the form.
    assert_eq!(Pjn.screen(&busy), Screen::Form { challenge: false });
    // The sign is always in the page, hidden.
    let idle = fixtures::form_page(true, None);
    assert!(idle.contains("Consulta en proceso"));
    assert!(!Pjn.busy(&page(idle)));
    assert!(!Pjn.busy(&marked(&fixtures::record_page(&record_view(&entries(5, 1), 1, false, false)), fixtures::RECORD_URL)));
    // A small wrapper around the hidden sign is not the sign.
    let wrapped = r#"<html><body><div id="estado"><div style="display:none"><span>Consulta en proceso</span></div></div></body></html>"#;
    assert!(!Pjn.busy(&page(wrapped.to_string())));
    assert!(Pjn.busy(&page(wrapped.replace(r#" style="display:none""#, ""))));
}

#[test]
fn a_query_the_app_pressed_is_waited_for_with_the_view_free_and_never_pressed_again() {
    let dir = TempDir::new();
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(4, 1));
    portal.query_busy = 3;
    portal.query_loading = 2;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    // «Consultar» once: never again while the portal processed it.
    assert_eq!(portal.presses, vec!["Consultar"]);
    // The app saw the sign at its first look, and from then on the view
    // was free for the portal's «Cancelar»; locked again after.
    assert_eq!(portal.busy_locks, vec![true, false, false]);
    assert_eq!(portal.unlocked_while, vec!["working"]);
    assert!(portal.locked);
    assert!(portal.reports.iter().any(|r| r.busy && r.message == QUERY_IN_COURSE));
    assert!(!outcome.busy);
    assert_eq!(outcome.downloaded, 3);
}

#[test]
fn a_query_the_person_pressed_is_never_pressed_again_and_cancelar_works() {
    let dir = TempDir::new();
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(4, 1));
    portal.form_challenge = true;
    portal.query_busy = 3;
    portal.cancel_while_busy = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    // The person pressed «Consultar», then the portal's «Cancelar» (the
    // view was free), and «Consultar» again after the challenge: the app
    // pressed none of them.
    assert_eq!(portal.presses, vec!["persona: Consultar", "persona: Cancelar", "persona: Consultar"]);
    assert!(!portal.busy_locks.is_empty());
    assert!(portal.busy_locks.iter().all(|locked| !locked), "{:?}", portal.busy_locks);
    // Nor was the focus on the challenge over the portal's «Cancelar».
    assert!(portal.busy_focused.iter().all(|focused| !focused), "{:?}", portal.busy_focused);
    assert_eq!(portal.focus_log, vec![true, false, true, false]);
    // After the cancelled query the form went back to the person; while
    // the second one was processed, the person had nothing to do.
    assert_eq!(portal.unlocked_while, vec!["needs-person", "needs-person"]);
    assert!(portal.reports.iter().any(|r| r.busy && r.state == "working" && r.person.is_none()));
    assert!(portal.locked);
    assert_eq!(outcome.downloaded, 3);
}

#[test]
fn a_query_that_never_ends_stops_with_what_to_do() {
    let dir = TempDir::new();
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(4, 1));
    portal.query_busy = usize::MAX;
    let mut walk = walk_with(dir.path(), &store::NoneKnown);
    walk.query_looks = 12;
    match walk.run(&mut portal, &key()) {
        Err(Stop::Failed(message)) => assert_eq!(message, QUERY_STUCK),
        other => panic!("{other:?}"),
    }
    assert_eq!(portal.presses, vec!["Consultar"]);
    assert_eq!(portal.busy_locks.len(), 12);
    assert!(portal.locked);
}

#[test]
fn an_address_of_the_portal_without_encryption_is_opened_with_https() {
    let go = |url: &str| navigation(&Pjn, &tauri::Url::parse(url).unwrap());
    assert_eq!(go(START_URL), Navigation::Allow);
    assert_eq!(go("about:blank"), Navigation::Allow);
    // The PJN's redirect after «Consultar».
    assert_eq!(
        go("http://scw.pjn.gov.ar/scw/expediente.seam?cid=1234567"),
        Navigation::Upgrade(tauri::Url::parse("https://scw.pjn.gov.ar/scw/expediente.seam?cid=1234567").unwrap())
    );
    assert_eq!(go("http://pjn.gov.ar/"), Navigation::Upgrade(tauri::Url::parse("https://pjn.gov.ar/").unwrap()));
    assert_eq!(go("http://scw.pjn.gov.ar:80/scw/home.seam"), Navigation::Upgrade(tauri::Url::parse(START_URL).unwrap()));
    for refused in [
        "http://scw.pjn.gov.ar:8080/scw/home.seam",
        "http://usuario:clave@scw.pjn.gov.ar/scw/home.seam",
        "http://pjn.gov.ar.example.com/",
        "http://example.com/",
        "http://localhost:1420/",
        "http://tauri.localhost/",
        "https://example.com/",
        "ftp://scw.pjn.gov.ar/",
        "file:///C:/Windows/win.ini",
        "javascript:alert(1)",
    ] {
        assert_eq!(go(refused), Navigation::Refuse, "{refused}");
    }
}

#[test]
fn a_portal_that_keeps_sending_the_view_to_http_is_not_followed_forever() {
    let url = tauri::Url::parse("https://scw.pjn.gov.ar/scw/expediente.seam?cid=1").unwrap();
    let other = tauri::Url::parse("https://scw.pjn.gov.ar/scw/expediente.seam?cid=2").unwrap();
    let mut upgrades = Upgrades::default();
    let now = std::time::Instant::now();
    for _ in 0..Upgrades::MAX_SAME {
        assert!(upgrades.admit(&url, now));
    }
    assert!(!upgrades.admit(&url, now));
    assert!(upgrades.admit(&other, now));
    // Later, the same address is followed again.
    assert!(upgrades.admit(&url, now + Upgrades::WINDOW));
}

#[test]
fn a_year_given_by_a_list_is_filled_too() {
    let html = r#"<html><body><form><label>Jurisdicción</label><select><option value="9">ZZZ - Inventada</option></select>
        <label>Número</label><input type="text" id="n"><label>Año</label><select id="y"><option>2010</option><option>2011</option></select>
        <button type="submit">Consultar</button></form></body></html>"#;
    let page = marked(html, START_URL);
    let plan = Pjn.form(&page, &key()).unwrap();
    let id = |mark: &str| page.els.iter().find(|e| e.mark() == Some(mark)).and_then(|e| e.attr("id")).map(str::to_string);
    assert_eq!(plan.fills.len(), 3);
    assert_eq!(plan.fills[0].1, "9");
    assert_eq!((id(&plan.fills[1].0).as_deref(), plan.fills[1].1.as_str()), (Some("n"), "4321"));
    assert_eq!((id(&plan.fills[2].0).as_deref(), plan.fills[2].1.as_str()), (Some("y"), "2011"));
    assert!(!plan.challenge);
}

#[test]
fn signing_in_is_left_to_the_person() {
    let dir = TempDir::new();
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(4, 1));
    portal.login_first = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut portal, &key()).unwrap();
    assert!(portal.reports.iter().any(|r| r.person.as_deref() == Some("login")));
    assert_eq!(outcome.downloaded, 3);
    // Signing in has no focus.
    assert!(portal.focus_log.is_empty());
}

#[test]
fn a_cut_walk_resumes_where_it_left_and_never_downloads_twice() {
    let dir = TempDir::new();
    let list = entries(38, 300);
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    first.stop_after = Some(5);
    let stopped = walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key());
    assert!(matches!(stopped, Err(Stop::Cancelled)));
    assert_eq!(first.downloads.len(), 5);
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    assert_eq!(store::load(&folder).unwrap().unwrap().count(state::DOWNLOADED), 5);
    let mut second = FakePortal::new(REFERENCE, TITLE, list.clone());
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut second, &key()).unwrap();
    let total = list.iter().filter(|e| e.document.is_some()).count();
    assert_eq!(second.downloads.len(), total - 5);
    assert!(second.downloads.iter().all(|d| !first.downloads.contains(d)));
    assert_eq!(outcome.already, 5);
    assert_eq!(pdfs_in(&folder).len(), total);
    // A third time, nothing (these invented dates do not come newest
    // first, so it goes through every page).
    let mut third = FakePortal::new(REFERENCE, TITLE, list);
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut third, &key()).unwrap();
    assert!(third.downloads.is_empty());
    assert_eq!(outcome.already, total);
    assert_eq!(outcome.known_from, None);
    assert!(outcome.message.contains("0 documentos nuevos"), "{}", outcome.message);
}

/// `count` invented SCW entries, all of them newer than those of `scw_entries`.
fn newer_entries(count: usize, first_document: u32) -> Vec<fixtures::FakeEntry> {
    let mut newer = fixtures::scw_entries(count, first_document);
    for (i, e) in newer.iter_mut().enumerate() {
        e.date = format!("{}/12/2026", 20 - i);
        e.detail = format!("NOVEDAD INVENTADA {i}");
    }
    newer
}

#[test]
fn an_update_stops_at_the_first_page_it_already_had_whole() {
    let dir = TempDir::new();
    let list = fixtures::scw_entries(50, 700);
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    first.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key()).unwrap();
    // The first time, every page (the file did not exist).
    assert_eq!(first.presses, vec!["Consultar", "Siguiente", "Siguiente", "Siguiente"]);
    assert_eq!(outcome.known_from, None);
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    assert!(store::load(&folder).unwrap().unwrap().complete);
    // Three new entries on top: page 1 has them, page 2 is all known, and
    // the pages after it are older.
    let mut now = newer_entries(3, 900);
    now.extend(list.iter().cloned());
    let mut second = FakePortal::new(REFERENCE, TITLE, now.clone());
    second.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut second, &key()).unwrap();
    assert_eq!(second.presses, vec!["Consultar", "Siguiente"]);
    let mut got = second.downloads.clone();
    got.sort();
    let mut want: Vec<u32> = now[..3].iter().filter_map(|e| e.document).collect();
    want.sort();
    assert_eq!(got, want);
    assert_eq!(outcome.known_from, Some(2));
    assert_eq!(outcome.seen, 30);
    assert!(store::load(&folder).unwrap().unwrap().complete);
    assert!(outcome.message.contains("desde la página 2 ya estaba todo"), "{}", outcome.message);
    // «Revisar todo el expediente»: every page, and nothing to bring.
    let mut whole = FakePortal::new(REFERENCE, TITLE, now.clone());
    whole.scw = true;
    let mut walk = walk_with(dir.path(), &store::NoneKnown);
    walk.full = true;
    let outcome = walk.run(&mut whole, &key()).unwrap();
    assert_eq!(whole.presses, vec!["Consultar", "Siguiente", "Siguiente", "Siguiente"]);
    assert!(whole.downloads.is_empty());
    assert_eq!((outcome.seen, outcome.known_from), (53, None));
}

#[test]
fn a_walk_that_did_not_end_leaves_the_next_one_going_through_it_all() {
    let dir = TempDir::new();
    let list = fixtures::scw_entries(50, 700);
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    first.scw = true;
    walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key()).unwrap();
    // 18 new entries: the first page is all new; cut while on it.
    let mut now = newer_entries(18, 900);
    now.extend(list.iter().cloned());
    let mut cut = FakePortal::new(REFERENCE, TITLE, now.clone());
    cut.scw = true;
    cut.stop_after = Some(9);
    assert!(matches!(walk_with(dir.path(), &store::NoneKnown).run(&mut cut, &key()), Err(Stop::Cancelled)));
    assert!(!store::load(&folder).unwrap().unwrap().complete);
    // The next one does not stop at page 1, which it now has whole: the
    // new entries of page 2 were never seen.
    let mut next = FakePortal::new(REFERENCE, TITLE, now.clone());
    next.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut next, &key()).unwrap();
    assert_eq!(next.presses.iter().filter(|p| *p == "Siguiente").count(), 4);
    let new_documents = now[..18].iter().filter(|e| e.document.is_some()).count();
    assert_eq!(cut.downloads.len() + next.downloads.len(), new_documents);
    assert_eq!(outcome.known_from, None);
    assert!(store::load(&folder).unwrap().unwrap().complete);
}

#[test]
fn what_failed_on_a_later_page_is_tried_again_before_stopping() {
    let dir = TempDir::new();
    let list = fixtures::scw_entries(50, 700);
    let bad = list[40..].iter().find_map(|e| e.document).unwrap();
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    first.scw = true;
    first.not_pdf.insert(bad);
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key()).unwrap();
    assert_eq!(outcome.failed, 1);
    let mut second = FakePortal::new(REFERENCE, TITLE, list);
    second.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut second, &key()).unwrap();
    assert_eq!(second.downloads, vec![bad]);
    assert_eq!(second.presses.iter().filter(|p| *p == "Siguiente").count(), 3);
    assert_eq!((outcome.failed, outcome.known_from), (0, None));
}

#[test]
fn entries_that_do_not_come_newest_first_never_stop_the_walk_early() {
    let dir = TempDir::new();
    let mut list = fixtures::scw_entries(50, 700);
    list.reverse();
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    first.scw = true;
    walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key()).unwrap();
    let mut second = FakePortal::new(REFERENCE, TITLE, list);
    second.scw = true;
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut second, &key()).unwrap();
    assert_eq!(second.presses.iter().filter(|p| *p == "Siguiente").count(), 3);
    assert_eq!(outcome.known_from, None);
}

#[test]
fn an_update_brings_only_what_is_new() {
    let dir = TempDir::new();
    let mut list = entries(16, 40);
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key()).unwrap();
    // Three new entries, on top.
    let mut newer = entries(3, 90);
    for (i, e) in newer.iter_mut().enumerate() {
        e.date = format!("{:02}/10/2026", 10 + i);
        e.detail = format!("Novedad inventada {i}");
    }
    newer.extend(list.drain(..));
    let mut second = FakePortal::new(REFERENCE, TITLE, newer.clone());
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut second, &key()).unwrap();
    let mut got = second.downloads.clone();
    got.sort();
    let mut want: Vec<u32> = newer[..3].iter().filter_map(|e| e.document).collect();
    want.sort();
    assert_eq!(got, want);
    assert_eq!(outcome.seen, 19);
}

#[test]
fn what_the_base_already_has_by_its_name_is_not_downloaded() {
    let dir = TempDir::new();
    let list = entries(4, 60);
    let first = &list[0];
    let old = Entry { date: first.date.clone(), kind: first.kind.clone(), folios: first.folios.clone(), detail: "otro texto".to_string(), ..Entry::default() };
    // A copy brought before, in some folder of the base.
    let name = store::document_name(&key(), &old);
    dir.write(&format!("archivo/viejo/{name}"), b"%PDF-1.4 copia anterior");
    let base = DocBase::open(&dir.path().join("base.sqlite3")).unwrap();
    let origin = base.add_origin("Archivo", &dir.path().join("archivo"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    let mut portal = FakePortal::new(REFERENCE, TITLE, list.clone());
    let outcome = walk_with(&dir.path().join("trabajo"), &base).run(&mut portal, &key()).unwrap();
    assert!(!portal.downloads.contains(&first.document.unwrap()));
    assert_eq!(portal.downloads.len(), 2);
    assert_eq!(outcome.already, 1);
    let record = store::load(&dir.path().join("trabajo").join("PJN").join("ZZZ 4321-2011")).unwrap().unwrap();
    let entry = &record.entries[record.entry_index(&EntryKey::new(&first.folios, &first.date, &first.kind, &first.detail)).unwrap()];
    assert_eq!(entry.state, state::ALREADY);
    assert!(entry.file.as_deref().unwrap().ends_with(&name));
}

#[test]
fn what_is_not_a_pdf_is_not_kept_and_is_tried_again() {
    let dir = TempDir::new();
    let list = entries(3, 10);
    let bad = list[0].document.unwrap();
    let mut first = FakePortal::new(REFERENCE, TITLE, list.clone());
    first.not_pdf.insert(bad);
    let outcome = walk_with(dir.path(), &store::NoneKnown).run(&mut first, &key()).unwrap();
    assert_eq!(outcome.failed, 1);
    assert!(outcome.message.contains("1 no se pudieron bajar"));
    let folder = dir.path().join("PJN").join("ZZZ 4321-2011");
    assert_eq!(pdfs_in(&folder).len(), 2);
    let record = store::load(&folder).unwrap().unwrap();
    assert_eq!(record.count(state::FAILED), 1);
    let mut second = FakePortal::new(REFERENCE, TITLE, list);
    walk_with(dir.path(), &store::NoneKnown).run(&mut second, &key()).unwrap();
    assert_eq!(second.downloads, vec![bad]);
    assert_eq!(pdfs_in(&folder).len(), 3);
}

#[test]
fn the_record_goes_into_the_base_as_a_confirmed_matter_with_its_documents() {
    let dir = TempDir::new();
    let work = dir.path().join("trabajo");
    let expedientes = work.join("Expedientes");
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(3, 20));
    walk_with(&expedientes, &store::NoneKnown).run(&mut portal, &key()).unwrap();
    let base = DocBase::open(&dir.path().join("base.sqlite3")).unwrap();
    // The records' folder is part of the work folder's origin, never an
    // origin of its own (D-451).
    let (origin, inside) = store::ensure_origin(&base, &work, &expedientes).unwrap();
    assert_eq!(inside, "Expedientes");
    assert_eq!(base.origin(&origin).unwrap().unwrap().name, "Carpeta de trabajo");
    assert_eq!(base.origin(&origin).unwrap().unwrap().root(), work);
    assert_eq!(base.origins(&crate::docbase::Visible::All).unwrap().len(), 1);
    base.scan_origin(&origin).unwrap();
    base.hash_pending(100).unwrap();
    // The data files are not documents of the base.
    let names: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT name FROM location").unwrap();
        statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
    };
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(names.iter().all(|n| n.ends_with(".pdf")));
    let folder = store::record_folder(&expedientes.join("PJN"), &key());
    let mut record: RecordFile = store::load(&folder).unwrap().unwrap();
    let rel = store::rel_inside(&inside, "PJN", &key());
    assert_eq!(rel, "Expedientes/PJN/ZZZ 4321-2011");
    assert_eq!(store::ensure_matter(&base, &mut record, &origin, &rel).unwrap(), 3);
    let matter = record.matter_id.clone().unwrap();
    let (name, origin_kind, confirmation): (String, String, String) = base
        .conn()
        .query_row("SELECT name, origin, confirmation FROM knowledge_matter WHERE id = ?1", [&matter], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!(name, format!("{TITLE} (ZZZ 4321/2011)"));
    assert_eq!((origin_kind.as_str(), confirmation.as_str()), ("person", "confirmed"));
    // Again: the same matter, nothing new to put in.
    assert_eq!(store::ensure_matter(&base, &mut record, &origin, &rel).unwrap(), 0);
    assert_eq!(record.matter_id.as_deref(), Some(matter.as_str()));
    // A work folder that already is an origin holds the records' folder.
    let other = TempDir::new();
    let work = other.folder("trabajo");
    let base = DocBase::open(&other.path().join("base.sqlite3")).unwrap();
    let origin = base.add_origin("Trabajo", &work, &[]).unwrap();
    let (found, inside) = store::ensure_origin(&base, &work, &work.join("Expedientes")).unwrap();
    assert_eq!((found, inside.as_str()), (origin.id, "Expedientes"));
    assert_eq!(store::rel_inside(&inside, "PJN", &key()), "Expedientes/PJN/ZZZ 4321-2011");
    // A records' origin made before inside the work folder (the first
    // version of the portals) joins the work folder's, with its documents
    // and their matter.
    let third = TempDir::new();
    let work = third.path().join("trabajo");
    let expedientes = work.join("Expedientes");
    let mut portal = FakePortal::new(REFERENCE, TITLE, entries(3, 20));
    walk_with(&expedientes, &store::NoneKnown).run(&mut portal, &key()).unwrap();
    let base = DocBase::open(&third.path().join("base.sqlite3")).unwrap();
    let old = base.add_origin("Expedientes", &expedientes, &[]).unwrap();
    base.scan_origin(&old.id).unwrap();
    base.hash_pending(100).unwrap();
    let folder = store::record_folder(&expedientes.join("PJN"), &key());
    let mut record: RecordFile = store::load(&folder).unwrap().unwrap();
    assert_eq!(store::ensure_matter(&base, &mut record, &old.id, "PJN/ZZZ 4321-2011").unwrap(), 3);
    let (origin, inside) = store::ensure_origin(&base, &work, &expedientes).unwrap();
    assert_eq!(inside, "Expedientes");
    let origins = base.origins(&crate::docbase::Visible::All).unwrap();
    assert_eq!(origins.iter().map(|o| (o.id.as_str(), o.name.as_str())).collect::<Vec<_>>(), [(origin.as_str(), "Carpeta de trabajo")]);
    let rel = store::rel_inside(&inside, "PJN", &key());
    // Nothing is lost: the same documents, already in the matter.
    assert_eq!(store::ensure_matter(&base, &mut record, &origin, &rel).unwrap(), 0);
    let in_matter: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_matter_choice WHERE matter_id = ?1", [record.matter_id.clone().unwrap()], |r| r.get(0))
        .unwrap();
    assert_eq!(in_matter, 3);
    let report = base.scan_origin(&origin).unwrap();
    assert_eq!((report.added, report.removed), (0, 0));
}

#[test]
fn only_the_portal_may_download_several_documents_without_asking_and_nothing_else() {
    use super::{grants_by_itself, Permission};
    for origin in ["https://scw.pjn.gov.ar/", "https://scw.pjn.gov.ar/scw/expediente.seam?cid=1", "https://pjn.gov.ar", "https://SCW.PJN.GOV.AR/"] {
        assert!(grants_by_itself(&Pjn, Permission::MultipleDownloads, origin), "{origin}");
    }
    for origin in [
        "http://scw.pjn.gov.ar/",
        "https://pjn.gov.ar.example.com/",
        "https://example.com/",
        "https://evilpjn.gov.ar/",
        "about:blank",
        "file:///C:/",
        "",
        "no es una dirección",
    ] {
        assert!(!grants_by_itself(&Pjn, Permission::MultipleDownloads, origin), "{origin}");
    }
    // No other permission, not even to the portal.
    assert!(!grants_by_itself(&Pjn, Permission::Other, "https://scw.pjn.gov.ar/"));
}

#[test]
fn the_portal_view_has_no_permission_of_the_app() {
    // The capabilities are only for the app's own window, and never for a
    // remote page.
    let capability: serde_json::Value = serde_json::from_str(include_str!("../../capabilities/default.json")).unwrap();
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    assert!(capability.get("remote").is_none());
    assert!(capability.get("webviews").is_none());
    // And every app command is refused to any webview but the app's own.
    assert!(is_app_webview("main"));
    assert!(!is_app_webview(&format!("portal-{}", Pjn.id())));
    // The view only goes to the portal.
    let allows = |url: &str| Pjn.allows(&tauri::Url::parse(url).unwrap());
    assert!(allows(START_URL));
    assert!(allows("https://captcha.pjn.gov.ar/desafio"));
    assert!(allows("https://sso.pjn.gov.ar/auth/realms/x"));
    assert!(allows("about:blank"));
    for refused in [
        "http://scw.pjn.gov.ar/scw/home.seam",
        "tauri://localhost/index.html",
        "http://tauri.localhost/",
        "http://localhost:1420/",
        "https://ipc.localhost/x",
        "https://pjn.gov.ar.example.com/",
        "https://example.com/",
        "file:///C:/Windows/win.ini",
        "javascript:alert(1)",
        "data:text/html,hola",
    ] {
        assert!(!allows(refused), "{refused}");
    }
}
