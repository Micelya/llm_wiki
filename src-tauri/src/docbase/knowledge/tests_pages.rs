//! The projection of the record into the pages of the base, over a real
//! base, with invented documents and a stand-in model (R-24: nothing here
//! comes from the test folder). No file is read or written (D-402).

use std::path::PathBuf;

use super::pages::{self, BLOCK_START, NOTES_MARK};
use super::tests::{answer, amount, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};

const A: &str = "DOC-A. Contrato de locación entre Ejemplo Uno S.A., CUIT 30-11111111-1, y Persona Dos, DNI 22.222.222, \
con domicilio en Calle Uno 100. Expediente 1234/2020 del juzgado.";
const B: &str = "DOC-B. Recibo firmado por Persona Dos, DNI 22.222.222, a favor de Ejemplo Uno S.A. por $ 5.000 en efectivo.";
const E: &str = "DOC-E. Carta de Ejemplo Uno S.A., con domicilio en Calle Uno 100, dirigida a Uds. y firmada por el apoderado.";
const C: &str = "DOC-C. Nota al pasar que menciona a Persona Cinco y a la empresa Otra Cosa sin decir nada más de ellas, sobre el Expediente 1234/2020.";

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote }])
}

fn reading(prompt: &str) -> String {
    let none = serde_json::json!([]);
    let (mentions, claims) = if prompt.contains("DOC-A.") {
        (
            vec![
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A., CUIT 30-11111111-1", ids("cuit", "30-11111111-1", "CUIT 30-11111111-1")),
                mention("p", "persona", "Persona Dos", "parte", "Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("x", "asunto", "Expediente 1234/2020", "referido", "Expediente 1234/2020 del juzgado", ids("expediente", "1234/2020", "Expediente 1234/2020")),
            ],
            vec![claim("p", "domicilio", None, Some("Calle Uno 100"), "Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100", None)],
        )
    } else if prompt.contains("DOC-B.") {
        (
            vec![
                mention("p", "persona", "Persona Dos", "firmante", "Recibo firmado por Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("o", "organizacion", "Ejemplo Uno S.A.", "destinatario", "a favor de Ejemplo Uno S.A.", none.clone()),
            ],
            vec![amount("p", "o", "$ 5.000", "cuota", "a favor de Ejemplo Uno S.A. por $ 5.000", None)],
        )
    } else if prompt.contains("DOC-E.") {
        // An address, a pronoun and a role read as names.
        (
            vec![
                mention("o", "organizacion", "Ejemplo Uno S.A.", "remitente", "Carta de Ejemplo Uno S.A.", none.clone()),
                mention("d", "organizacion", "Calle Uno 100", "domicilio", "con domicilio en Calle Uno 100", none.clone()),
                mention("u", "organizacion", "Uds.", "destinatario", "dirigida a Uds.", none.clone()),
                mention("f", "persona", "apoderado", "firmante", "firmada por el apoderado", none.clone()),
            ],
            vec![],
        )
    } else if prompt.contains("DOC-D.") {
        (vec![mention("p", "persona", "Persona Dos", "remitente", "Otra nota de Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222"))], vec![])
    } else if prompt.contains("DOC-C.") {
        (
            vec![
                mention("p", "persona", "Persona Cinco", "otro", "menciona a Persona Cinco", none.clone()),
                mention("o", "organizacion", "Otra Cosa", "otro", "la empresa Otra Cosa", none.clone()),
                mention("x", "asunto", "Expediente 1234/2020", "referido", "sobre el Expediente 1234/2020", ids("expediente", "1234/2020", "Expediente 1234/2020")),
            ],
            vec![],
        )
    } else {
        return empty_answer();
    };
    answer(mentions, claims)
}

pub(crate) fn is_synthesis(prompt: &str) -> bool {
    prompt.starts_with("Sos asistente de un estudio jurídico. Redactá lo que dicen")
}

/// A sentence per claim, backed by it; one with a number no claim says,
/// and one naming a claim that does not exist: both must go.
pub(crate) fn synthesis(prompt: &str) -> String {
    let mut sentences = Vec::new();
    let mut count = 0;
    for line in prompt.lines() {
        let Some(rest) = line.strip_prefix('[') else { continue };
        let Some((number, text)) = rest.split_once("] ") else { continue };
        let Ok(number) = number.parse::<i64>() else { continue };
        count += 1;
        let said = text.split(" — ").next().unwrap_or(text);
        sentences.push(serde_json::json!({ "texto": format!("Según los documentos, {said}."), "afirmaciones": [number] }));
    }
    sentences.push(serde_json::json!({ "texto": "Debe además $ 999.999.", "afirmaciones": [1] }));
    sentences.push(serde_json::json!({ "texto": "Algo sin respaldo.", "afirmaciones": [count + 5] }));
    serde_json::json!({ "oraciones": sentences }).to_string()
}

fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

fn synthesis_calls(model: &FakeModel) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| is_synthesis(q)).count()
}

/// The archive, read and resolved, with the folder of an old wiki project
/// as its general project (only the import of its files reads it).
fn archive(files: &[(&str, &str)]) -> (TempDir, DocBase, PathBuf, FakeModel) {
    let (dir, base) = base_with(files);
    dir.folder("General/wiki/sources");
    dir.write("General/schema.md", b"# schema");
    let project = dir.path().join("General");
    crate::docbase::testing::point_general_project_at(&base, &project);
    let asker = model();
    read(&base, &asker);
    (dir, base, project, asker)
}

pub(crate) fn read(base: &DocBase, asker: &FakeModel) {
    base.types_run(asker, 50).unwrap();
    while base.knowledge_run(asker, 50).unwrap().extracted > 0 {}
    base.knowledge_resolve(Some(asker), 50).unwrap();
}

/// The pages of a folder in the base (`wiki/entities`, `wiki/sources`):
/// (file name, text whole with its notes, as the old file had it).
pub(crate) fn pages_in(base: &DocBase, folder: &str) -> Vec<(String, String)> {
    let paths: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT path FROM knowledge_page WHERE path LIKE ?1 AND body IS NOT NULL").unwrap();
        let rows = statement.query_map([format!("{folder}/%")], |row| row.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let mut out: Vec<(String, String)> =
        paths.iter().map(|path| (path.rsplit('/').next().unwrap().to_string(), page(base, path))).collect();
    out.sort();
    out
}

/// The entity pages in the base: (file name, text).
pub(crate) fn entity_pages(base: &DocBase) -> Vec<(String, String)> {
    pages_in(base, "wiki/entities")
}

/// The page at a logical path, whole with its notes.
pub(crate) fn page(base: &DocBase, path: &str) -> String {
    base.knowledge_page_text(path).unwrap().unwrap_or_else(|| panic!("no page at {path}")).1
}

pub(crate) fn has_page(base: &DocBase, path: &str) -> bool {
    base.knowledge_page_text(path).unwrap().is_some()
}

/// The object of a page, as its header names it.
pub(crate) fn object_in(text: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix("entity_id: ").or_else(|| l.strip_prefix("document_id: ")).or_else(|| l.strip_prefix("matter_id: ")))
        .unwrap_or_else(|| panic!("no object in {text}"))
        .trim()
        .to_string()
}

fn overview(base: &DocBase) -> String {
    base.knowledge_overview().unwrap().expect("an overview").body
}

/// The note (D-441) the first mark after `after` names on a page, as the
/// end of the page writes it (`[^n]: …`).
pub(crate) fn note_after(page: &str, after: &str) -> String {
    let at = page.find(after).unwrap_or_else(|| panic!("no {after:?} in {page}")) + after.len();
    let rest = &page[at..];
    let start = rest.find("[^").unwrap_or_else(|| panic!("no note after {after:?} in {page}"));
    let label: String = rest[start + 2..].chars().take_while(|c| *c != ']').collect();
    let definition = format!("\n[^{label}]: ");
    let from = page.find(&definition).unwrap_or_else(|| panic!("no note {label} in {page}")) + definition.len();
    page[from..].lines().next().unwrap_or_default().to_string()
}

pub(crate) fn page_of<'a>(pages: &'a [(String, String)], title: &str) -> &'a str {
    pages
        .iter()
        .find(|(_, text)| text.contains(&format!("title: \"{title}\"")))
        .map(|(_, text)| text.as_str())
        .unwrap_or_else(|| panic!("no page for {title}: {:?}", pages.iter().map(|p| &p.0).collect::<Vec<_>>()))
}

/// What the pages call a file: its origin and its path inside it.
fn identity_of(base: &DocBase, name: &str) -> String {
    let origin = base.origins(&Visible::All).unwrap()[0].name.clone();
    let rel: String = base.conn().query_row("SELECT rel_path FROM location WHERE name = ?1", [name], |row| row.get(0)).unwrap();
    format!("{origin}/{rel}")
}

#[test]
fn every_entity_with_a_page_has_all_its_documents_and_only_verified_sentences() {
    let (_dir, base, _project, asker) = archive(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert!(!report.skipped, "{report:?}");
    let pages = entity_pages(&base);
    // A person in two documents, a company with a role, a matter two
    // documents are about; who is only named in passing has no page.
    assert_eq!(pages.len(), 3, "{:?}", pages.iter().map(|p| &p.0).collect::<Vec<_>>());
    assert!(!pages.iter().any(|(_, t)| t.contains("Persona Cinco") || t.contains("Otra Cosa")));
    let person = page_of(&pages, "Persona Dos");
    for header in ["type: entity", "entity_kind: persona", "entity_id: ent-", "generated_by: micelya-conocimiento"] {
        assert!(person.contains(header), "{header} missing:\n{person}");
    }
    // Every document that names it, in its header and in its list, with its roles.
    assert!(person.contains(&identity_of(&base, "a.txt")) && person.contains(&identity_of(&base, "b.txt")), "{person}");
    assert!(person.contains("## Documentos (2)"), "{person}");
    // Each document with a link to its page (D-340).
    assert!(person.contains("|a.txt]] — parte") && person.contains("|b.txt]] — firmante"), "{person}");
    // Grouped by matter: the lease under its file number, the receipt apart.
    assert!(person.contains("### [[expediente-1234-2020-"), "{person}");
    assert!(person.contains("### Sin caso identificado"), "{person}");
    // What the documents say: one sentence per claim, with its note; the
    // invented amount and the reference to nothing are thrown away.
    assert!(person.contains("Calle Uno 100.[^"), "{person}");
    assert!(person.contains("$ 5.000"), "{person}");
    assert!(!person.contains("999"), "{person}");
    assert!(!person.contains("Algo sin respaldo"), "{person}");
    // Each note says the claim, its quote and its document, at the end of
    // the page, numbered in the order the text names them (D-441); no list
    // of references nor of other data repeats them.
    let address = note_after(person, "Calle Uno 100.");
    assert!(address.starts_with("Domicilio: Calle Uno 100 — «Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100» ([["), "{address}");
    assert!(address.ends_with("|a.txt]])"), "{address}");
    assert!(note_after(person, "$ 5.000").contains("«a favor de Ejemplo Uno S.A. por $ 5.000» ([["), "{person}");
    let first = person.find("[^1]").expect("a first note");
    assert!(first < person.find("[^2]").expect("a second note") && person.contains("\n[^1]: ") && person.contains("\n[^2]: "), "{person}");
    for gone in ["### Referencias", "### Otros datos", "dato por dato", " [1]", " [2]"] {
        assert!(!person.contains(gone), "{gone}: {person}");
    }
    assert!(person.contains("DNI 22.222.222 (2 documentos)"), "{person}");
    assert_eq!(report.discarded, 2 * synthesis_calls(&asker), "{report:?}");
    // Related entities, as links to their pages.
    assert!(person.contains("## Relacionados") && person.contains("[[ejemplo-uno-s-a-"), "{person}");
    // The notes, last, under their mark.
    assert!(person.trim_end().ends_with("-->"), "{person}");
    assert!(person.contains(NOTES_MARK));
    // The amount is told on both pages, each from its side (D-244).
    assert!(person.contains("Paga cuotas a Ejemplo Uno S.A.: $ 5.000 (cuota)"), "{person}");
    let company = page_of(&pages, "Ejemplo Uno S.A.");
    assert!(company.contains("Cobra cuotas de Persona Dos: $ 5.000 (cuota)"), "{company}");
    assert_eq!(synthesis_calls(&asker), 2);
    // The overview, from the record, in the words of the vocabulary.
    let overview = overview(&base);
    assert!(overview.contains("# Resumen del archivo") && overview.contains("[[persona-dos-"), "{overview}");
    // Every document read counts, also one that names nobody with a page
    // (D-240); the most named, by the documents where they have a role.
    assert!(overview.contains("- Documentos leídos: 3\n"), "{overview}");
    assert!(!overview.contains("lectura falló"), "{overview}");
    assert!(overview.contains("· Persona — 2 documentos"), "{overview}");

    // Projecting again changes nothing and asks nothing (R-13).
    let again = base.knowledge_project(Some(&asker), 50).unwrap();
    assert!(again.skipped || (again.written == 0 && again.asked == 0), "{again:?}");
    assert_eq!(entity_pages(&base), pages);
    assert_eq!(synthesis_calls(&asker), 2);
}

#[test]
fn a_renamed_entity_moves_its_page_with_its_notes_and_the_links_follow() {
    let (dir, base, _project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&base);
    let (file, text) = pages.iter().find(|(_, t)| t.contains("title: \"Persona Dos\"")).unwrap().clone();
    base.knowledge_set_notes(&object_in(&text), "Cliente desde hace años; llamar antes de ir.").unwrap();
    let old_stem = file.trim_end_matches(".md").to_string();
    let id_part = old_stem.rsplit('-').next().unwrap().to_string();
    // A text the app does not write links to it: a saved query.
    base.save_query("Nota", "", &format!("related: [{old_stem}]\n\nVer [[{old_stem}|el cliente]].\n"), &[]).unwrap();

    // The entity is renamed and gets another document: the page moves to
    // the file of the new name (the same id at its end), with the notes as
    // they were, and the links to the old file follow (D-239).
    dir.write("o/d.txt", "DOC-D. Otra nota de Persona Dos, DNI 22.222.222, sobre el mismo contrato y sus pagos.".as_bytes());
    let origin = base.origins(&crate::docbase::Visible::All).unwrap()[0].id.clone();
    super::tests::refresh(&base, &origin);
    read(&base, &asker);
    base.conn().execute("UPDATE knowledge_entity SET name = 'Persona Dos Bis' WHERE name = 'Persona Dos'", []).unwrap();
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.moved, 1, "{report:?}");
    let after = entity_pages(&base);
    assert!(!after.iter().any(|(f, _)| *f == file), "{:?}", after.iter().map(|p| &p.0).collect::<Vec<_>>());
    let new_file = format!("persona-dos-bis-{id_part}.md");
    let renamed = after.iter().find(|(f, _)| *f == new_file).map(|(_, t)| t.as_str()).unwrap_or_else(|| panic!("{after:?}"));
    assert!(renamed.contains("title: \"Persona Dos Bis\""), "{renamed}");
    assert!(renamed.trim_end().ends_with("Cliente desde hace años; llamar antes de ir."), "{renamed}");
    assert_eq!(renamed.matches(NOTES_MARK).count(), 1);
    assert!(renamed.contains("## Documentos (3)"), "{renamed}");
    let note = base.saved_queries().unwrap()[0].answer.clone();
    let new_stem = new_file.trim_end_matches(".md");
    assert!(note.contains(&format!("[[{new_stem}|el cliente]]")) && note.contains(&format!("related: [{new_stem}]")), "{note}");
    // Nothing moves again.
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    assert_eq!(base.knowledge_project(Some(&asker), 50).unwrap().moved, 0);
}

#[test]
fn a_date_in_english_is_written_in_the_language_of_the_pages() {
    use super::vocabulary::Vocabulary;
    let words = Vocabulary::builtin();
    assert_eq!(pages::localized_dates(&words, "Firmado el December 30, 2021 en Ciudad."), "Firmado el 30 de diciembre de 2021 en Ciudad.");
    assert_eq!(pages::localized_dates(&words, "vence el 1 March 2024, sin prórroga"), "vence el 1 de marzo de 2024, sin prórroga");
    assert_eq!(pages::localized_dates(&words, "pagó $ 30 May be"), "pagó $ 30 May be");
    assert_eq!(pages::localized_dates(&words, "el 23 días del mes de Octubre de 2025, tenía"), "el 23 de octubre de 2025, tenía");
    // An ordinal day, as Spanish writing in English puts it (D-283).
    assert_eq!(pages::localized_dates(&words, "rige desde el JANUARY 1°, 2026 (inicio)"), "rige desde el 1 de enero de 2026 (inicio)");
    // In the brackets of a form.
    assert_eq!(pages::localized_dates(&words, "celebrado as of [JANUARY 1°, 2026] y"), "celebrado as of [1 de enero de 2026] y");
}

#[test]
fn a_date_of_a_document_or_a_reading_is_spelled_as_the_pages_write_it() {
    use super::vocabulary::Vocabulary;
    let words = Vocabulary::builtin();
    for (written, spelled) in [
        ("16.04.2026", "16 de abril de 2026"),
        ("28/08/2026", "28 de agosto de 2026"),
        ("29-04-2026", "29 de abril de 2026"),
        ("2026-04-16", "16 de abril de 2026"),
        // The month first only when the day cannot be.
        ("12/31/2021", "31 de diciembre de 2021"),
        ("12 DE JUNIO DE 2026", "12 de junio de 2026"),
        ("December 30, 2021", "30 de diciembre de 2021"),
        ("Abril 2026", "Abril 2026"),
        ("31/02/20", "31/02/20"),
        // With its time, which stays; with its month in words.
        ("18/10/2024 17:06:26", "18 de octubre de 2024 17:06:26"),
        ("18/octubre/2024", "18 de octubre de 2024"),
        ("16.04.2026 en Ciudad", "16.04.2026 en Ciudad"),
    ] {
        assert_eq!(pages::spelled_date(&words, written), spelled, "{written}");
    }
    // A title says a date that starts with its day or its month, without its time.
    assert_eq!(pages::title_date(&words, "18/10/2024 17:06:26").as_deref(), Some("18 de octubre de 2024"));
    assert_eq!(pages::title_date(&words, "Abril 2026").as_deref(), Some("Abril 2026"));
    assert_eq!(pages::title_date(&words, "de octubre de 2024"), None);
}

const F: &str = "DOC-F. Factura 0001-00000123 de Ejemplo Uno S.A. a Persona Dos. Fecha: 16.04.2026. Vence el March 3, 2026. Total $ 500.";

/// The reading of an invoice: who sends it to whom, its date, and a point
/// of its summary with a date in English.
fn invoice_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if is_knowledge(prompt) {
            let none = serde_json::json!([]);
            let mentions = vec![
                mention("o", "organizacion", "Ejemplo Uno S.A.", "emisor", "Factura 0001-00000123 de Ejemplo Uno S.A.", none.clone()),
                mention("p", "persona", "Persona Dos", "destinatario", "a Persona Dos", none),
            ];
            let mut said: serde_json::Value = serde_json::from_str(&answer(mentions, vec![])).unwrap();
            said["fecha_documento"] = serde_json::json!("16.04.2026");
            said["resumen"] = serde_json::json!({
                "tema": "Factura de Ejemplo Uno S.A. a Persona Dos.",
                "puntos": [{ "texto": "Vence el March 3, 2026.", "menciones": [], "cita": "Vence el March 3, 2026", "pagina": null }]
            });
            said.to_string()
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("factura")
        }
    })
}

/// The page of a document is titled by what the base knows of it (P-182):
/// its kind, its number, who sends it to whom and its date, never its
/// file's name, which stays in the data; its dates are written as the
/// pages write them, its quotes as the document does (D-442).
#[test]
fn the_page_of_a_document_has_a_readable_title_and_its_dates_in_the_words_of_the_pages() {
    let (_dir, base) = base_with(&[("scan-001.txt", F)]);
    let asker = invoice_model();
    read(&base, &asker);
    let content = content_of(&base, "scan-001.txt");
    base.conn()
        .execute(
            "INSERT OR REPLACE INTO document_data (version_id, doc_type, data, missing, question, basis, computed_at)
             VALUES (?1, 'factura', ?2, '[]', NULL, 'prueba', 0)",
            rusqlite::params![
                content,
                serde_json::json!([{ "field": "numero", "value": "0001-00000123" }, { "field": "fecha-de-emision", "value": "16.04.2026" }]).to_string()
            ],
        )
        .unwrap();
    base.knowledge_project(Some(&asker), 50).unwrap();
    let page = page(&base, "wiki/sources/scan-001.md");
    assert!(
        page.contains("title: \"Factura 0001-00000123 de Ejemplo Uno S.A. a Persona Dos · 16 de abril de 2026\"\nfile: \"scan-001.txt\"\n"),
        "{page}"
    );
    assert!(page.contains("# Factura 0001-00000123 de Ejemplo Uno S.A. a Persona Dos · 16 de abril de 2026\n"), "{page}");
    assert!(page.contains("Fecha: 16 de abril de 2026"), "{page}");
    assert!(page.contains("- fecha de emision: 16 de abril de 2026\n"), "{page}");
    assert!(page.contains("- Vence el 3 de marzo de 2026.[^1]\n"), "{page}");
    assert_eq!(note_after(&page, "Vence el 3 de marzo de 2026."), "«Vence el March 3, 2026»");
    // The page of who sends it links to it by the name of its file.
    let company = page_of(&entity_pages(&base), "Ejemplo Uno S.A.").to_string();
    assert!(company.contains("|scan-001.txt]] — emisor"), "{company}");
}

#[test]
fn a_page_whose_entity_leaves_goes_unless_it_has_notes() {
    let (_dir, base, _project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&base);
    let (company, _) = pages.iter().find(|(_, t)| t.contains("title: \"Ejemplo Uno S.A.\"")).unwrap().clone();
    let (person, text) = pages.iter().find(|(_, t)| t.contains("title: \"Persona Dos\"")).unwrap().clone();
    base.knowledge_set_notes(&object_in(&text), "Una nota.").unwrap();
    // Both entities leave the record.
    base.conn().execute("UPDATE knowledge_entity SET state = 'retired' WHERE kind IN ('persona', 'organizacion')", []).unwrap();
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!((report.removed, report.kept), (1, 1), "{report:?}");
    assert!(!has_page(&base, &format!("wiki/entities/{company}")));
    let kept = page(&base, &format!("wiki/entities/{person}"));
    assert!(kept.contains("state: kept") && kept.trim_end().ends_with("Una nota."), "{kept}");
    // Its notes taken out, it goes too.
    base.knowledge_set_notes(&object_in(&kept), "").unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!((report.removed, report.kept), (1, 0), "{report:?}");
    assert!(!has_page(&base, &format!("wiki/entities/{person}")));
}

/// Every document gets its page from the base, and the notes a person
/// wrote on a page the old ingest wrote of it come into the base once
/// (D-403); what the old ingest wrote does not.
#[test]
fn every_document_gets_its_page_from_the_base_and_the_notes_of_an_older_one_come_once() {
    let (dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    let source = |name: &str| format!("---\ntype: source\ntitle: {name}\nsources: [\"{}\"]\n---\n\nResumen del documento.\n\n## Notas\n\nNota vieja.\n", identity_of(&base, name));
    // A page the old ingest wrote of one document; none of the other.
    dir.write("General/wiki/sources/viejo-a.md", source("a.txt").as_bytes());
    let imported = base.import_wiki_files().unwrap();
    assert_eq!((imported.pages, imported.with_notes, imported.to_objects, imported.orphans), (1, 1, 1, 0), "{imported:?}");
    // Once.
    assert!(base.import_wiki_files().unwrap().skipped);
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!((report.source_pages, report.blocks_written), (2, 2), "{report:?}");
    let a = page(&base, "wiki/sources/a.md");
    // Titled by its kind and its parties (P-182); its file in the data.
    assert!(
        a.starts_with("---\ntype: source\ntitle: \"Contrato entre Ejemplo Uno S.A. y Persona Dos\"\nfile: \"a.txt\"\ndocument_id: doc-")
            && a.contains("generated_by: micelya-conocimiento"),
        "{a}"
    );
    assert!(a.contains(BLOCK_START) && a.contains("[[persona-dos-") && a.contains("· Persona — parte"), "{a}");
    // What the old ingest wrote is not there; the notes of a person are.
    assert!(!a.contains("Resumen del documento.") && a.trim_end().ends_with("Nota vieja."), "{a}");
    // Where the document is, from the base.
    assert!(a.contains("## Ubicaciones") && a.contains(&format!("`{}`", identity_of(&base, "a.txt"))), "{a}");
    let b = page(&base, "wiki/sources/b.md");
    assert!(b.contains("[[persona-dos-") && b.contains("firmante"), "{b}");
    // The old file is as it was.
    assert_eq!(std::fs::read_to_string(project.join("wiki/sources/viejo-a.md")).unwrap(), source("a.txt"));

    // Projecting again writes nothing.
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    assert_eq!(base.knowledge_project(Some(&asker), 50).unwrap().blocks_written, 0);
    assert_eq!(page(&base, "wiki/sources/b.md"), b);
}

/// Nothing on a document's page but what the base knows (D-340); the notes
/// of a person are apart, and a page as the base wrote it is not written
/// again when only its notes change.
#[test]
fn a_document_page_holds_only_what_the_base_knows_and_its_notes_apart() {
    let (_dir, base, _project, asker) = archive(&[("a.txt", A)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let a = page(&base, "wiki/sources/a.md");
    let id = object_in(&a);
    base.knowledge_set_notes(&id, "Mi nota.").unwrap();
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.blocks_written, 0, "{report:?}");
    assert_eq!(page(&base, "wiki/sources/a.md"), format!("{a}Mi nota.\n"));
    let article = base.knowledge_article(&id).unwrap().unwrap();
    assert!(!article.body.contains("Mi nota.") && article.notes.unwrap().text == "Mi nota.", "{}", article.body);
}

#[test]
fn without_a_model_the_pages_are_written_and_the_synthesis_waits() {
    let (_dir, base, _project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    let report = base.knowledge_project(None, 0).unwrap();
    assert_eq!(report.left, 2, "{report:?}");
    let person = page_of(&entity_pages(&base), "Persona Dos").to_string();
    assert!(person.contains("La síntesis todavía no está escrita"), "{person}");
    // Meanwhile what the documents say is in «Datos y correcciones», with
    // its quote and its actions: the article says so, and does not repeat
    // it (D-441).
    assert!(person.contains("Cada dato, con su cita, está en «Datos y correcciones»."), "{person}");
    assert!(!person.contains("dato por dato") && !person.contains("Calle Uno 100 —"), "{person}");
    assert!(person.contains("## Documentos (2)"));
    // The next run with a model writes it.
    let report = base.knowledge_project(Some(&asker), 5).unwrap();
    assert_eq!((report.asked, report.written), (2, 2), "{report:?}");
    assert!(!page_of(&entity_pages(&base), "Persona Dos").contains("La síntesis todavía no está escrita"));
}

#[test]
fn a_claim_the_synthesis_leaves_out_is_counted_on_the_page_and_kept_in_its_data() {
    let (_dir, base, _project, _) = archive(&[("a.txt", A), ("b.txt", B)]);
    // A synthesis that only writes about the first claim.
    let terse = FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            serde_json::json!({ "oraciones": [{ "texto": "Tiene un dato.", "afirmaciones": [1] }] }).to_string()
        } else {
            reading(prompt)
        }
    });
    base.knowledge_project(Some(&terse), 5).unwrap();
    let person = page_of(&entity_pages(&base), "Persona Dos").to_string();
    assert!(person.contains("Tiene un dato.[^1]"), "{person}");
    // The second claim, not written: the page says one more is in
    // «Datos y correcciones» (D-441), where it is with its quote.
    assert!(person.contains("Hay un dato más de los documentos, con su cita, en «Datos y correcciones»."), "{person}");
    assert!(!person.contains("[^2]") && !person.contains("### Otros datos"), "{person}");
    let id = object_in(&person);
    let detail = base.knowledge_entity(&Visible::All, &id).unwrap().expect("its data");
    assert_eq!(detail.claims.iter().filter(|c| c.state != "retired").count(), 2, "{:?}", detail.claims);
}

#[test]
fn a_page_with_nothing_more_to_say_does_not_wait_for_a_synthesis() {
    // The company of these documents has a page but no claim: it never
    // gets a synthesis, so the attention view must not wait for one.
    let (_dir, base, _project, asker) = archive(&[("a.txt", A)]);
    base.knowledge_project(None, 0).unwrap();
    assert_eq!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().synthesis_pending, 1);
    base.knowledge_project(Some(&asker), 5).unwrap();
    let company = page_of(&entity_pages(&base), "Ejemplo Uno S.A.").to_string();
    assert!(company.contains("Los documentos no dicen de esta entidad nada más"), "{company}");
    assert_eq!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().synthesis_pending, 0);
}

#[test]
fn an_address_a_pronoun_or_a_role_read_as_a_name_gets_no_page_nor_line() {
    let (_dir, base, _project, asker) = archive(&[("a.txt", A), ("e.txt", E)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&base);
    let titles: Vec<&str> = pages.iter().filter_map(|(_, t)| t.lines().find(|l| l.starts_with("title: "))).collect();
    assert!(titles.iter().any(|t| t.contains("Ejemplo Uno S.A.")), "{titles:?}");
    for not_a_name in ["Calle Uno 100", "Uds.", "apoderado"] {
        assert!(!titles.iter().any(|t| t.contains(not_a_name)), "{not_a_name} has a page: {titles:?}");
    }
    let e = page(&base, "wiki/sources/e.md");
    assert!(e.contains("[[ejemplo-uno-s-a-"), "{e}");
    for not_a_name in ["Calle Uno 100 ·", "Uds. ·", "apoderado ·"] {
        assert!(!e.contains(not_a_name), "{not_a_name} is in the block: {e}");
    }
}

#[test]
fn the_pages_do_not_depend_on_the_order_of_reading() {
    let texts = |files: &[(&str, &str)]| {
        let (_dir, base, _project, asker) = archive(files);
        base.knowledge_project(Some(&asker), 50).unwrap();
        entity_pages(&base)
            .into_iter()
            .map(|(_, text)| {
                // Without the date it was written and the folder of the test.
                text.lines().filter(|l| !l.starts_with("updated:") && !l.starts_with("sources:")).collect::<Vec<_>>().join("\n")
            })
            .collect::<Vec<_>>()
    };
    let one = texts(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let two = texts(&[("c.txt", C), ("b.txt", B), ("a.txt", A)]);
    assert_eq!(one, two);
}

#[test]
fn file_names_are_readable_and_carry_the_id() {
    assert_eq!(pages::slug("Ejemplo Uno S.A.", "organizacion"), "ejemplo-uno-s-a");
    assert_eq!(pages::slug("Peña, José", "persona"), "pena-jose");
    assert_eq!(pages::slug("—", "persona"), "persona");
    assert!(pages::name_like("persona", "Persona Dos") && pages::name_like("asunto", "1234/2020"));
    assert!(!pages::name_like("organizacion", "alguien@ejemplo.com") && !pages::name_like("persona", "(011) 4444-5555"));
    assert_eq!(pages::page_path("wiki/entities", "ent-0123456789abcdef", "Persona Dos", "persona"), "wiki/entities/persona-dos-01234567.md");
    let with = pages::with_block("---\ntype: source\n---\n\nTexto.\n", Some("<!-- micelya:entidades:inicio -->\nx\n<!-- micelya:entidades:fin -->\n"));
    assert!(with.ends_with("Texto.\n\n<!-- micelya:entidades:inicio -->\nx\n<!-- micelya:entidades:fin -->\n"), "{with}");
    assert_eq!(pages::with_block(&with, None), "---\ntype: source\n---\n\nTexto.\n");
}

#[test]
fn the_questions_are_kept_and_counted_once() {
    let (_dir, base, _project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    let questions = base.synthesis_questions("falso:1").unwrap();
    assert_eq!(questions.len(), 2);
    assert!(questions[0].prompt.contains("Escribí en el idioma en que están escritas las citas."));
    base.set_wiki_language(Some("Spanish")).unwrap();
    let questions = base.synthesis_questions("falso:1").unwrap();
    assert!(questions[0].prompt.contains("Escribí en este idioma: Spanish."));
    base.knowledge_project(Some(&asker), 50).unwrap();
    assert!(base.synthesis_questions("falso:1").unwrap().is_empty());
    assert_eq!(synthesis_calls(&asker), 2);
}

#[test]
fn a_document_page_links_the_names_its_summary_writes() {
    // No wiki at all: the pages need none (D-402).
    let (_dir, base) = base_with(&[("a.txt", A)]);
    let asker = summary_model();
    read(&base, &asker);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let a = page(&base, "wiki/sources/a.md");
    // The first place in the running text, once, with the app's mark; not the heading.
    assert!(a.contains("Contrato de locación entre [[ejemplo-uno-s-a-") && a.contains("y [[persona-dos-"), "{a}");
    assert!(a.contains("|Persona Dos]]<!--micelya-->.") && a.contains("\n- Persona Dos tiene domicilio"), "{a}");
    // Projecting again changes nothing.
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.blocks_written, 0, "{report:?}");
    assert_eq!(page(&base, "wiki/sources/a.md"), a);
}

/// The model of a test whose reading of DOC-A also sums it up.
fn summary_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if is_knowledge(prompt) {
            let mut said: serde_json::Value = serde_json::from_str(&reading(prompt)).unwrap();
            said["resumen"] = serde_json::json!({
                "tema": "Contrato de locación entre Ejemplo Uno S.A. y Persona Dos.",
                "puntos": [
                    { "texto": "Persona Dos tiene domicilio en Calle Uno 100.", "menciones": ["p"],
                      "cita": "Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100", "pagina": null },
                    { "texto": "Se cita el expediente 1234/2020.", "menciones": ["x"], "cita": "Expediente 1234/2020 del juzgado", "pagina": null }
                ]
            });
            said.to_string()
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

/// With the summary of the one reading, the page of a document is the
/// base's (D-331): what it is, its points with their quotes, the notes of
/// a person kept and the block at the end; nothing is asked to write it,
/// and projecting again changes nothing.
#[test]
fn the_page_of_a_document_is_written_from_its_summary() {
    let (dir, base) = base_with(&[("a.txt", A)]);
    dir.folder("General/wiki/sources");
    dir.write("General/schema.md", b"# schema");
    let project = dir.path().join("General");
    crate::docbase::testing::point_general_project_at(&base, &project);
    let asker = summary_model();
    read(&base, &asker);
    let old = format!(
        "---\ntype: source\ntitle: a.txt\nsources: [\"{}\"]\n---\n\nLo que escribió el modelo antes.\n\n## Notas\n\nUna nota mía.\n",
        identity_of(&base, "a.txt")
    );
    dir.write("General/wiki/sources/a.md", old.as_bytes());
    base.import_wiki_files().unwrap();
    let asked_before = asker.asked.lock().unwrap().len();
    base.knowledge_project(Some(&asker), 50).unwrap();
    let a = page(&base, "wiki/sources/a.md");
    assert!(a.starts_with("---\ntype: source\ntitle: \"Contrato entre Ejemplo Uno S.A. y Persona Dos\"\n") && a.contains("generated_by: micelya-conocimiento"), "{a}");
    assert!(a.contains("# Contrato entre Ejemplo Uno S.A. y Persona Dos\n"), "{a}");
    assert!(a.contains("Contrato de locación entre") && a.contains("## Lo esencial"), "{a}");
    // Each point with its quote as a note (D-441).
    assert!(a.contains("- Persona Dos tiene domicilio en Calle Uno 100.[^1]\n"), "{a}");
    assert_eq!(note_after(&a, "tiene domicilio en Calle Uno 100."), "«Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100»");
    assert!(!a.contains("— dice:"), "{a}");
    assert!(!a.contains("## Versiones de este documento"), "one version: {a}");
    assert!(!a.contains("Lo que escribió el modelo antes."), "{a}");
    assert!(!a.contains("Importes entre las partes"), "no amount in this document: {a}");
    assert!(a.contains("Una nota mía.") && a.contains(BLOCK_START), "{a}");
    // The block of entities under what the document is; the notes last.
    assert!(a.find(BLOCK_START).unwrap() < a.find("## Lo esencial").unwrap(), "{a}");
    assert!(a.trim_end().ends_with("Una nota mía."), "{a}");
    // Writing it asked nothing about the document (only syntheses of entities).
    assert!(asker.asked.lock().unwrap()[asked_before..].iter().all(|q| is_synthesis(q)));
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.blocks_written, 0, "{report:?}");
    assert_eq!(page(&base, "wiki/sources/a.md"), a);
}

const V1: &str = "DOC-V1. Borrador del contrato de servicios entre Ejemplo Uno S.A. y Persona Dos. Precio total $ 1.000. Plazo de doce meses.";
const V2: &str = "DOC-V2. Contrato de servicios entre Ejemplo Uno S.A. y Persona Dos, firmado por las dos. Precio total $ 1.200. Plazo de doce meses.";
const T: &str = "DOC-T. Contrato de servicios entre Ejemplo Uno S.A. y Persona Seis. Precio total $ 3.000. Plazo de doce meses.";

/// The reading of the versions: a summary point of each price and one of
/// the term all of them say.
fn versions_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if is_knowledge(prompt) {
            let price = ["$ 1.000", "$ 1.200", "$ 3.000"].into_iter().find(|p| prompt.contains(&format!("Precio total {p}")));
            let mut said: serde_json::Value = serde_json::from_str(&empty_answer()).unwrap();
            if let Some(price) = price {
                said["resumen"] = serde_json::json!({
                    "tema": "Contrato de servicios.",
                    "puntos": [
                        { "texto": format!("El precio es {price}."), "menciones": [], "cita": format!("Precio total {price}"), "pagina": null },
                        { "texto": "El plazo es de doce meses.", "menciones": [], "cita": "Plazo de doce meses", "pagina": null }
                    ]
                });
            }
            said.to_string()
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

fn content_of(base: &DocBase, name: &str) -> i64 {
    base.conn()
        .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| row.get(0))
        .unwrap()
}

fn relate(base: &DocBase, one: i64, other: i64, relation: &str, subject: Option<i64>) {
    let (a, b) = (one.min(other), one.max(other));
    base.conn()
        .execute(
            "INSERT INTO similar_pair (a, b, relation, subject, certainty, jaccard, containment, cover, similarity, description,
                 evidence, evidence_hash, engine, computed_at) VALUES (?1, ?2, ?3, ?4, 'high', 0.9, 0.9, 0.9, 0.9, '', '{}', 'x', 'prueba', 0)",
            rusqlite::params![a, b, relation, subject],
        )
        .unwrap();
}

/// The versions of one document share a page (D-340): named after the
/// signed one, each point and datum says which version it is from when not
/// all say it, and two versions that differ show both. A document of the
/// same template is a page apart, linked. The notes of the page the old
/// wiki had of a version come to the page of the document; a relation a
/// person discards puts the versions on pages apart again.
#[test]
fn the_versions_of_a_document_share_a_page_and_say_which_version_says_what() {
    let (dir, base) = base_with(&[("v1.txt", V1), ("v2.txt", V2), ("t.txt", T)]);
    dir.folder("General/wiki/sources");
    dir.write("General/schema.md", b"# schema");
    let project = dir.path().join("General");
    crate::docbase::testing::point_general_project_at(&base, &project);
    let asker = versions_model();
    read(&base, &asker);
    let (v1, v2, t) = (content_of(&base, "v1.txt"), content_of(&base, "v2.txt"), content_of(&base, "t.txt"));
    {
        let conn = base.conn();
        conn.execute("DELETE FROM similar_pair", []).unwrap();
        conn.execute("UPDATE document_type SET stage = 'borrador' WHERE version_id = ?1", [v1]).unwrap();
        conn.execute("UPDATE document_type SET stage = 'firmado' WHERE version_id = ?1", [v2]).unwrap();
        for (content, price) in [(v1, "$ 1.000"), (v2, "$ 1.200")] {
            conn.execute(
                "INSERT OR REPLACE INTO document_data (version_id, doc_type, data, missing, question, basis, computed_at)
                 VALUES (?1, 'contrato', ?2, '[]', NULL, 'prueba', 0)",
                rusqlite::params![content, serde_json::json!([{ "field": "precio", "value": price }, { "field": "plazo", "value": "doce meses" }]).to_string()],
            )
            .unwrap();
        }
    }
    relate(&base, v1, v2, "signed-copy", Some(v2));
    relate(&base, v1, t, "same-template", None);
    // The page the old ingest wrote of the draft, with a note.
    let old = format!("---\ntype: source\ntitle: v1\nsources: [\"{}\"]\n---\n\nTexto viejo.\n\n## Notas\n\nNota del borrador.\n", identity_of(&base, "v1.txt"));
    dir.write("General/wiki/sources/viejo-v1.md", old.as_bytes());
    assert_eq!(base.import_wiki_files().unwrap().to_objects, 1);

    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.source_pages, 2, "{report:?}");
    let sources = || pages_in(&base, "wiki/sources").into_iter().map(|(name, _)| name).collect::<Vec<_>>();
    // One page for the document, named after the signed one, with the
    // notes of the old page of the draft.
    assert_eq!(sources(), vec!["t.md".to_string(), "v2.md".to_string()]);
    let page = page(&base, "wiki/sources/v2.md");
    // Titled by its kind; two pages that would read the same say their
    // file too (P-182).
    assert!(page.contains("title: \"Contrato (v2)\"") && page.contains("# Contrato (v2)\n") && page.contains("file: \"v2.txt\""), "{page}");
    assert!(page.contains(&identity_of(&base, "v1.txt")) && page.contains(&identity_of(&base, "v2.txt")), "{page}");
    assert!(page.contains("## Versiones de este documento") && page.contains("- **firmado**") && page.contains("- **borrador**"), "{page}");
    assert!(page.contains("- firmado es el ejemplar firmado de borrador"), "{page}");
    // Each price with its version, in its note; the term both say, once
    // and without one.
    assert_eq!(note_after(&page, "El precio es $ 1.200."), "«Precio total $ 1.200» (firmado)");
    assert_eq!(note_after(&page, "El precio es $ 1.000."), "«Precio total $ 1.000» (borrador)");
    assert_eq!(page.matches("El plazo es de doce meses.").count(), 1, "{page}");
    assert_eq!(note_after(&page, "El plazo es de doce meses."), "«Plazo de doce meses»");
    assert!(page.contains("- precio: $ 1.200 (firmado), $ 1.000 (borrador)\n") && page.contains("- plazo: doce meses\n"), "{page}");
    // The document of the same template: its own page, linked.
    assert!(page.contains("## Documentos relacionados") && page.contains("- misma plantilla: [[t|Contrato (t)]]"), "{page}");
    let template = super::tests_pages::page(&base, "wiki/sources/t.md");
    assert!(template.contains("- misma plantilla: [[v2|Contrato (v2)]]") && !template.contains("Versiones"), "{template}");
    assert!(page.trim_end().ends_with("Nota del borrador."), "{page}");
    assert!(!page.contains("Texto viejo."), "{page}");

    // A person discards the relation: the versions are pages apart again,
    // the first one keeps the page (its path and its notes).
    base.conn().execute("UPDATE similar_pair SET state = 'discarded' WHERE relation = 'signed-copy'", []).unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.source_pages, 3, "{report:?}");
    assert_eq!(sources(), vec!["t.md".to_string(), format!("v2-{}.md", v1.max(v2)), "v2.md".to_string()]);
    let draft = super::tests_pages::page(&base, "wiki/sources/v2.md");
    assert!(draft.contains("title: \"Contrato (v1)\"") && !draft.contains("Versiones") && draft.trim_end().ends_with("Nota del borrador."), "{draft}");
    // Nothing of it was written in the old wiki's folder.
    assert_eq!(std::fs::read_dir(project.join("wiki/sources")).unwrap().count(), 1);
}

/// The pages, their notes, the overview and the saved queries live in the
/// base, and every link of a page leads to its object (D-402, D-404): no
/// file is written, not even in the old wiki's folder.
#[test]
fn the_articles_their_notes_and_their_links_live_in_the_base() {
    let (_dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let files = |root: &PathBuf| -> usize {
        fn count(dir: &std::path::Path) -> usize {
            std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| if e.path().is_dir() { count(&e.path()) } else { 1 }).sum()
        }
        count(root)
    };
    let before = files(&project);
    base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(files(&project), before);
    let pages = entity_pages(&base);
    let person = object_in(page_of(&pages, "Persona Dos"));
    let article = base.knowledge_article(&person).unwrap().unwrap();
    assert_eq!((article.kind.as_str(), article.title.as_str(), article.state.as_str()), ("entity", "Persona Dos", "projected"));
    assert!(article.path.starts_with("wiki/entities/persona-dos-") && article.notes.is_none(), "{article:?}");
    // A matter and a document have theirs.
    let matter: String = base.conn().query_row("SELECT entity_id FROM knowledge_page WHERE entity_id LIKE 'mat-%'", [], |r| r.get(0)).unwrap();
    assert_eq!(base.knowledge_article(&matter).unwrap().unwrap().kind, "matter");
    let content = content_of(&base, "a.txt");
    let document = base.knowledge_document_article(content).unwrap().unwrap();
    assert_eq!((document.kind.as_str(), document.path.as_str()), ("document", "wiki/sources/a.md"));
    // Every way of naming a page leads to its object.
    let stem = article.path.trim_start_matches("wiki/entities/").trim_end_matches(".md").to_string();
    for reference in [article.path.clone(), format!("{}/{}", project.display(), article.path), stem.clone(), format!("{stem}.md#Notas")] {
        let object = base.knowledge_object_of_page(&reference).unwrap().unwrap_or_else(|| panic!("{reference}"));
        assert_eq!((object.object_id.as_str(), object.kind.as_str()), (person.as_str(), "entity"), "{reference}");
    }
    let of_document = base.knowledge_object_of_page("a").unwrap().unwrap();
    assert_eq!((of_document.kind.as_str(), of_document.content_id), ("document", Some(content)));
    assert!(of_document.location_id.is_some());
    assert_eq!(base.knowledge_object_of_page("wiki/overview.md").unwrap().unwrap().kind, "overview");
    assert_eq!(base.knowledge_object_of_page("nadie-por-aqui").unwrap(), None);
    assert!(overview(&base).contains("# Resumen del archivo"));
    // Notes: kept, read back, taken out; anything but an object refused.
    let note = base.knowledge_set_notes(&person, "Llamar los lunes.").unwrap().unwrap();
    assert_eq!(note.text, "Llamar los lunes.");
    assert_eq!(base.knowledge_notes(&person).unwrap().unwrap().text, "Llamar los lunes.");
    assert!(base.knowledge_set_notes(&person, "  ").unwrap().is_none());
    assert!(base.knowledge_set_notes("overview", "x").is_err());
    // Saved queries.
    let saved = base.save_query("Plazo", "¿Cuál es el plazo?", "Doce meses, según [[a]].", &["a".to_string()]).unwrap();
    assert_eq!(base.saved_queries().unwrap(), vec![saved.clone()]);
    assert!(base.save_query(" ", "", "x", &[]).is_err());
    base.delete_saved_query(saved.id).unwrap();
    assert!(base.saved_queries().unwrap().is_empty());
    assert_eq!(files(&project), before);
}

/// The import of the old wiki's files (D-403): the notes of each page go
/// to its object, those of a page that matches none are kept apart, the
/// saved queries come in; once, and the folder stays as it was.
#[test]
fn the_notes_and_queries_of_the_old_wiki_come_into_the_base_once() {
    let (dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&base);
    let (person_file, person_text) = pages.iter().find(|(_, t)| t.contains("title: \"Persona Dos\"")).unwrap().clone();
    // The old wiki's pages, as the projection wrote them, with notes.
    dir.write(&format!("General/wiki/entities/{person_file}"), format!("{person_text}Nota sobre la persona.\n").as_bytes());
    let a = page(&base, "wiki/sources/a.md");
    dir.write("General/wiki/sources/a.md", format!("{a}Nota sobre el contrato.\n").as_bytes());
    // A page no object matches, with notes; one without notes.
    dir.write("General/wiki/entities/alguien-viejo.md", format!("---\ntype: entity\ntitle: Alguien Viejo\n---\n\n# Alguien Viejo\n\n## Notas\n\n{NOTES_MARK}: x -->\nNo perder esto.\n").as_bytes());
    dir.write("General/wiki/entities/vacia.md", b"---\ntype: entity\ntitle: Vacia\n---\n\n# Vacia\n");
    dir.write("General/wiki/queries/plazo.md", "---\ntype: query\ntitle: \"Plazo\"\ncreated: 2025-03-04\n---\n\nDoce meses, según [[a]].\n".as_bytes());
    let before: Vec<(String, Vec<u8>)> = {
        let mut out = Vec::new();
        for folder in ["wiki/entities", "wiki/sources", "wiki/queries"] {
            for entry in std::fs::read_dir(project.join(folder)).unwrap().flatten() {
                out.push((entry.path().display().to_string(), std::fs::read(entry.path()).unwrap()));
            }
        }
        out.sort();
        out
    };

    let report = base.import_wiki_files().unwrap();
    assert_eq!((report.pages, report.with_notes, report.to_objects, report.orphans, report.queries), (4, 3, 2, 1, 1), "{report:?}");
    let person = object_in(&person_text);
    assert_eq!(base.knowledge_notes(&person).unwrap().unwrap().text, "Nota sobre la persona.");
    assert_eq!(base.knowledge_notes(&object_in(&a)).unwrap().unwrap().text, "Nota sobre el contrato.");
    let orphans = base.knowledge_orphan_notes().unwrap();
    assert_eq!(orphans.len(), 1);
    assert_eq!((orphans[0].title.as_deref(), orphans[0].text.as_str()), (Some("Alguien Viejo"), "No perder esto."));
    assert_eq!(orphans[0].origin_path.as_deref(), Some("wiki/entities/alguien-viejo.md"));
    let queries = base.saved_queries().unwrap();
    assert_eq!((queries[0].title.as_str(), queries[0].refs.clone()), ("Plazo", vec!["a".to_string()]));
    // Once: a second time reads nothing and changes nothing.
    assert!(base.import_wiki_files().unwrap().skipped);
    assert_eq!(base.saved_queries().unwrap().len(), 1);
    // The notes are on the pages; the folder is as it was.
    assert!(page(&base, &format!("wiki/entities/{person_file}")).trim_end().ends_with("Nota sobre la persona."));
    let after: Vec<(String, Vec<u8>)> = {
        let mut out = Vec::new();
        for folder in ["wiki/entities", "wiki/sources", "wiki/queries"] {
            for entry in std::fs::read_dir(project.join(folder)).unwrap().flatten() {
                out.push((entry.path().display().to_string(), std::fs::read(entry.path()).unwrap()));
            }
        }
        out.sort();
        out
    };
    assert_eq!(after, before);
    // An orphan note, once read, can go.
    base.knowledge_delete_note(orphans[0].id).unwrap();
    assert!(base.knowledge_orphan_notes().unwrap().is_empty());
}
