//! The projection of the record into the archive's wiki, over a real base,
//! with invented documents and a stand-in model (R-24: nothing here comes
//! from the test folder).

use std::path::PathBuf;

use super::pages::{self, BLOCK_END, BLOCK_START, NOTES_MARK};
use super::tests::{answer, amount, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;

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

/// The archive, read and resolved, with its general project's wiki.
fn archive(files: &[(&str, &str)]) -> (TempDir, DocBase, PathBuf, FakeModel) {
    let (dir, base) = base_with(files);
    dir.folder("General/wiki/sources");
    dir.write("General/schema.md", b"# schema");
    let project = dir.path().join("General");
    base.link_general_project(&project, "General").unwrap();
    let asker = model();
    read(&base, &asker);
    (dir, base, project, asker)
}

pub(crate) fn read(base: &DocBase, asker: &FakeModel) {
    base.types_run(asker, 50).unwrap();
    while base.knowledge_run(asker, 50).unwrap().extracted > 0 {}
    base.knowledge_resolve(Some(asker), 50).unwrap();
}

/// The entity pages on disk: (file name, text).
pub(crate) fn entity_pages(project: &PathBuf) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(project.join("wiki/entities"))
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|e| (e.file_name().to_string_lossy().to_string(), std::fs::read_to_string(e.path()).unwrap()))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

pub(crate) fn page_of<'a>(pages: &'a [(String, String)], title: &str) -> &'a str {
    pages
        .iter()
        .find(|(_, text)| text.contains(&format!("title: \"{title}\"")))
        .map(|(_, text)| text.as_str())
        .unwrap_or_else(|| panic!("no page for {title}: {:?}", pages.iter().map(|p| &p.0).collect::<Vec<_>>()))
}

fn identity_of(base: &DocBase, name: &str) -> String {
    base.wiki_sources().unwrap().into_iter().find(|s| s.identity.ends_with(name)).unwrap().identity
}

#[test]
fn every_entity_with_a_page_has_all_its_documents_and_only_verified_sentences() {
    let (_dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert!(!report.skipped, "{report:?}");
    let pages = entity_pages(&project);
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
    assert!(person.contains("a.txt — parte") && person.contains("b.txt — firmante"), "{person}");
    // Grouped by matter: the lease under its file number, the receipt apart.
    assert!(person.contains("### [[expediente-1234-2020-"), "{person}");
    assert!(person.contains("### Sin asunto identificado"), "{person}");
    // What the documents say: one sentence per claim, with its reference
    // and its quote; the invented amount and the reference to nothing are
    // thrown away.
    assert!(person.contains("Calle Uno 100. [1]") || person.contains("Calle Uno 100. [2]"), "{person}");
    assert!(person.contains("$ 5.000"), "{person}");
    assert!(!person.contains("999"), "{person}");
    assert!(!person.contains("Algo sin respaldo"), "{person}");
    assert!(person.contains("### Referencias") && person.contains("«a favor de Ejemplo Uno S.A. por $ 5.000»"), "{person}");
    assert!(person.contains("DNI 22.222.222 (2 documentos)"), "{person}");
    assert_eq!(report.discarded, 2 * synthesis_calls(&asker), "{report:?}");
    // Related entities, as links to their pages.
    assert!(person.contains("## Relacionados") && person.contains("[[ejemplo-uno-s-a-"), "{person}");
    // The notes, last, under their mark.
    assert!(person.trim_end().ends_with("-->"), "{person}");
    assert!(person.contains(NOTES_MARK));
    // The amount is told on both pages, each from its side (D-244).
    assert!(person.contains("Pagó a Ejemplo Uno S.A.: $ 5.000 (cuota pagada)"), "{person}");
    let company = page_of(&pages, "Ejemplo Uno S.A.");
    assert!(company.contains("Cobró de Persona Dos: $ 5.000 (cuota pagada)"), "{company}");
    assert_eq!(synthesis_calls(&asker), 2);
    // The overview, from the record, in the words of the vocabulary.
    let overview = std::fs::read_to_string(project.join("wiki/overview.md")).unwrap();
    assert!(overview.contains("# Resumen del archivo") && overview.contains("[[persona-dos-"), "{overview}");
    // Every document read counts, also one that names nobody with a page
    // (D-240); the most named, by the documents where they have a role.
    assert!(overview.contains("- Documentos leídos: 3\n"), "{overview}");
    assert!(!overview.contains("lectura falló"), "{overview}");
    assert!(overview.contains("· Persona — 2 documentos"), "{overview}");

    // Projecting again changes nothing and asks nothing (R-13).
    let again = base.knowledge_project(Some(&asker), 50).unwrap();
    assert!(again.skipped || (again.written == 0 && again.asked == 0), "{again:?}");
    assert_eq!(entity_pages(&project), pages);
    assert_eq!(synthesis_calls(&asker), 2);
}

#[test]
fn a_renamed_entity_moves_its_page_with_its_notes_and_the_links_follow() {
    let (dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&project);
    let (file, text) = pages.iter().find(|(_, t)| t.contains("title: \"Persona Dos\"")).unwrap().clone();
    std::fs::write(project.join("wiki/entities").join(&file), format!("{text}Cliente desde hace años; llamar antes de ir.\n")).unwrap();
    let old_stem = file.trim_end_matches(".md").to_string();
    let id_part = old_stem.rsplit('-').next().unwrap().to_string();
    // A page the app does not write links to it.
    dir.write("General/wiki/concepts/nota.md", format!("---\ntype: concept\ntitle: Nota\nrelated: [{old_stem}]\n---\n\nVer [[{old_stem}|el cliente]].\n").as_bytes());

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
    let after = entity_pages(&project);
    assert!(!after.iter().any(|(f, _)| *f == file), "{:?}", after.iter().map(|p| &p.0).collect::<Vec<_>>());
    let new_file = format!("persona-dos-bis-{id_part}.md");
    let renamed = after.iter().find(|(f, _)| *f == new_file).map(|(_, t)| t.as_str()).unwrap_or_else(|| panic!("{after:?}"));
    assert!(renamed.contains("title: \"Persona Dos Bis\""), "{renamed}");
    assert!(renamed.trim_end().ends_with("Cliente desde hace años; llamar antes de ir."), "{renamed}");
    assert_eq!(renamed.matches(NOTES_MARK).count(), 1);
    assert!(renamed.contains("## Documentos (3)"), "{renamed}");
    let note = std::fs::read_to_string(project.join("wiki/concepts/nota.md")).unwrap();
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
    // An ordinal day, as Spanish writing in English puts it (D-283).
    assert_eq!(pages::localized_dates(&words, "rige desde el JANUARY 1°, 2026 (inicio)"), "rige desde el 1 de enero de 2026 (inicio)");
}

#[test]
fn a_page_whose_entity_leaves_goes_unless_it_has_notes() {
    let (_dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&project);
    let (company, _) = pages.iter().find(|(_, t)| t.contains("title: \"Ejemplo Uno S.A.\"")).unwrap().clone();
    let (person, text) = pages.iter().find(|(_, t)| t.contains("title: \"Persona Dos\"")).unwrap().clone();
    std::fs::write(project.join("wiki/entities").join(&person), format!("{text}Una nota.\n")).unwrap();
    // Both entities leave the record.
    base.conn().execute("UPDATE knowledge_entity SET state = 'retired' WHERE kind IN ('persona', 'organizacion')", []).unwrap();
    super::mark_dirty(&base.conn(), "project", "test").unwrap();
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!((report.removed, report.kept), (1, 1), "{report:?}");
    assert!(!project.join("wiki/entities").join(&company).exists());
    let kept = std::fs::read_to_string(project.join("wiki/entities").join(&person)).unwrap();
    assert!(kept.contains("state: kept") && kept.trim_end().ends_with("Una nota."), "{kept}");
}

#[test]
fn a_document_page_written_before_or_after_gets_its_block() {
    let (dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    let source = |name: &str| format!("---\ntype: source\ntitle: {name}\nsources: [\"{}\"]\n---\n\nResumen del documento.\n", identity_of(&base, name));
    // One page of a document before the projection, one after.
    dir.write("General/wiki/sources/a.md", source("a.txt").as_bytes());
    base.knowledge_project(Some(&asker), 50).unwrap();
    let a = std::fs::read_to_string(project.join("wiki/sources/a.md")).unwrap();
    assert!(a.contains(BLOCK_START) && a.contains(BLOCK_END), "{a}");
    assert!(a.contains("[[persona-dos-") && a.contains("· Persona — parte"), "{a}");
    assert!(a.starts_with("---\ntype: source") && a.contains("Resumen del documento."), "{a}");

    dir.write("General/wiki/sources/b.md", source("b.txt").as_bytes());
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.blocks_written, 1, "{report:?}");
    let b = std::fs::read_to_string(project.join("wiki/sources/b.md")).unwrap();
    assert!(b.contains("[[persona-dos-") && b.contains("firmante"), "{b}");

    // The wiki writes the page again, keeping the old block in a merge:
    // there is still one block.
    let twice = format!("{b}\n{b}");
    std::fs::write(project.join("wiki/sources/b.md"), twice).unwrap();
    base.knowledge_project(Some(&asker), 50).unwrap();
    let b = std::fs::read_to_string(project.join("wiki/sources/b.md")).unwrap();
    assert_eq!(b.matches(BLOCK_START).count(), 1, "{b}");
}

#[test]
fn without_a_model_the_pages_are_written_and_the_synthesis_waits() {
    let (_dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    let report = base.knowledge_project(None, 0).unwrap();
    assert_eq!(report.left, 2, "{report:?}");
    let person = page_of(&entity_pages(&project), "Persona Dos").to_string();
    assert!(person.contains("La síntesis todavía no está escrita"), "{person}");
    // Meanwhile what the documents say is listed as it is, with its quote.
    assert!(person.contains("### Lo que dicen, dato por dato"), "{person}");
    assert!(person.contains("«a favor de Ejemplo Uno S.A. por $ 5.000»") && person.contains("Calle Uno 100 —"), "{person}");
    assert!(person.contains("## Documentos (2)"));
    // The next run with a model writes it.
    let report = base.knowledge_project(Some(&asker), 5).unwrap();
    assert_eq!((report.asked, report.written), (2, 2), "{report:?}");
    assert!(!page_of(&entity_pages(&project), "Persona Dos").contains("La síntesis todavía no está escrita"));
}

#[test]
fn a_claim_the_synthesis_leaves_out_is_still_on_the_page() {
    let (_dir, base, project, _) = archive(&[("a.txt", A), ("b.txt", B)]);
    // A synthesis that only writes about the first claim.
    let terse = FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            serde_json::json!({ "oraciones": [{ "texto": "Tiene un dato.", "afirmaciones": [1] }] }).to_string()
        } else {
            reading(prompt)
        }
    });
    base.knowledge_project(Some(&terse), 5).unwrap();
    let person = page_of(&entity_pages(&project), "Persona Dos").to_string();
    assert!(person.contains("Tiene un dato. [1]"), "{person}");
    let references = person.find("### Referencias").expect("references");
    let others = person.find("### Otros datos de los documentos").unwrap_or_else(|| panic!("{person}"));
    assert!(references < others, "{person}");
    // The second claim, not written: listed with its number and quote.
    assert!(person[others..].contains("- [2] "), "{person}");
    assert!(!person[references..others].contains("- [2] "), "{person}");
}

#[test]
fn a_page_with_nothing_more_to_say_does_not_wait_for_a_synthesis() {
    // The company of these documents has a page but no claim: it never
    // gets a synthesis, so the attention view must not wait for one.
    let (_dir, base, project, asker) = archive(&[("a.txt", A)]);
    base.knowledge_project(None, 0).unwrap();
    assert_eq!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().synthesis_pending, 1);
    base.knowledge_project(Some(&asker), 5).unwrap();
    let company = page_of(&entity_pages(&project), "Ejemplo Uno S.A.").to_string();
    assert!(company.contains("Los documentos no dicen de esta entidad nada más"), "{company}");
    assert_eq!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().synthesis_pending, 0);
}

#[test]
fn an_address_a_pronoun_or_a_role_read_as_a_name_gets_no_page_nor_line() {
    let (dir, base, project, asker) = archive(&[("a.txt", A), ("e.txt", E)]);
    let source = format!("---
type: source
title: e.txt
sources: [\"{}\"]
---

Resumen.
", identity_of(&base, "e.txt"));
    dir.write("General/wiki/sources/e.md", source.as_bytes());
    base.knowledge_project(Some(&asker), 50).unwrap();
    let pages = entity_pages(&project);
    let titles: Vec<&str> = pages.iter().filter_map(|(_, t)| t.lines().find(|l| l.starts_with("title: "))).collect();
    assert!(titles.iter().any(|t| t.contains("Ejemplo Uno S.A.")), "{titles:?}");
    for not_a_name in ["Calle Uno 100", "Uds.", "apoderado"] {
        assert!(!titles.iter().any(|t| t.contains(not_a_name)), "{not_a_name} has a page: {titles:?}");
    }
    let e = std::fs::read_to_string(project.join("wiki/sources/e.md")).unwrap();
    assert!(e.contains("[[ejemplo-uno-s-a-"), "{e}");
    for not_a_name in ["Calle Uno 100 ·", "Uds. ·", "apoderado ·"] {
        assert!(!e.contains(not_a_name), "{not_a_name} is in the block: {e}");
    }
}

#[test]
fn the_pages_do_not_depend_on_the_order_of_reading() {
    let texts = |files: &[(&str, &str)]| {
        let (_dir, base, project, asker) = archive(files);
        base.knowledge_project(Some(&asker), 50).unwrap();
        entity_pages(&project)
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
fn a_document_page_links_the_names_its_text_writes_as_the_document_does() {
    let (dir, base, project, asker) = archive(&[("a.txt", A), ("b.txt", B)]);
    let text = "Recibo de Persona Dos a favor de Ejemplo Uno S.A. por un pago. Persona Dos lo firma.";
    let page = format!("---\ntype: source\ntitle: b.txt\nsources: [\"{}\"]\n---\n\n# Persona Dos\n\n{text}\n", identity_of(&base, "b.txt"));
    dir.write("General/wiki/sources/b.md", page.as_bytes());
    base.knowledge_project(Some(&asker), 50).unwrap();
    let b = std::fs::read_to_string(project.join("wiki/sources/b.md")).unwrap();
    // The first place in the running text, once, with the app's mark; not the heading.
    assert!(b.contains("Recibo de [[persona-dos-"), "{b}");
    assert!(b.contains("|Persona Dos]]<!--micelya--> a favor de [[ejemplo-uno-s-a-"), "{b}");
    assert!(b.contains("# Persona Dos\n") && b.contains("pago. Persona Dos lo firma."), "{b}");
    // Projecting again changes nothing; without the app's part, the wiki's text.
    let report = base.knowledge_project(Some(&asker), 50).unwrap();
    assert_eq!(report.blocks_written, 0, "{report:?}");
    assert!(pages::with_block(&b, None).contains(text));
    // The links are not links a page the app does not write makes.
    assert_eq!(std::fs::read_to_string(project.join("wiki/sources/b.md")).unwrap(), b);
}
