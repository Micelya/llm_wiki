//! The passes over a real base, with invented documents (R-24: nothing
//! here comes from the test folder).

use super::store::Decision;
use crate::docbase::query::DocumentFilter;
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};

const CONTRACT: &str = "CONTRATO DE LOCACIÓN DE SERVICIOS. Entre la sociedad Ejemplo Uno S.A., en adelante la contratante, \
y la sociedad Ejemplo Dos S.R.L., en adelante la prestadora, se celebra el presente contrato. Primera: la prestadora se \
obliga a brindar servicios de mantenimiento preventivo y correctivo de los equipos informáticos de la contratante. \
Segunda: el plazo del contrato será de doce meses contados desde la firma. Tercera: la contratante abonará el precio \
pactado dentro de los diez días de recibida la factura. Cuarta: cualquiera de las partes podrá rescindir el contrato \
con un preaviso razonable. Quinta: las partes se someten a la jurisdicción de los tribunales ordinarios.";

fn contract_final() -> String {
    CONTRACT
        .replace(
            "abonará el precio pactado dentro de los diez días de recibida la factura",
            "pagará el precio convenido mediante transferencia bancaria dentro de los diez días hábiles posteriores a la recepción de la factura",
        )
        .replace("con un preaviso razonable", "con un preaviso fehaciente cursado por escrito con anticipación suficiente")
}

const AUDIT_PAGE: &str = "Historial\nFinal audit report\nFirmado electrónicamente por una de las partes\nTransaction ID: QWERTY";

const LETTER: &str = "Intimo a usted para que en el plazo de cuarenta y ocho horas regularice el pago de las facturas \
adeudadas por los servicios prestados durante el último trimestre, bajo apercibimiento de iniciar las acciones legales \
correspondientes y de suspender la prestación sin más aviso. Queda usted debidamente notificado.";

#[allow(clippy::too_many_arguments)]
fn invoice(number: &str, issued: &str, from: &str, to: &str, net: &str, tax: &str, total: &str, due: &str) -> String {
    format!(
        "FACTURA B N° {number}. Fecha de emisión: {issued}. Período facturado: {from} al {to}. Cliente: Ejemplo Tres S.A., \
         CUIT 30-11111111-1. Detalle: servicio mensual de alojamiento y soporte técnico de la plataforma, según contrato \
         vigente. Importe neto: $ {net}. IVA 21 %: $ {tax}. Total a pagar: $ {total}. Vencimiento: {due}. Medio de pago: \
         transferencia bancaria a la cuenta informada en el pie de este comprobante."
    )
}

fn march() -> String {
    invoice("0003-00001234", "15/03/2026", "01/02/2026", "28/02/2026", "120.000,00", "25.200,00", "145.200,00", "25/03/2026")
}

fn april() -> String {
    invoice("0003-00001290", "14/04/2026", "01/03/2026", "31/03/2026", "131.500,00", "27.615,00", "159.115,00", "24/04/2026")
}

/// A one-page fillable PDF: the page says `page`, and its only text
/// field, "Nombre", holds `name`.
fn filled_form(page: &str, name: &str) -> Vec<u8> {
    // One line of the page every eight words, so it fits the page.
    let words: Vec<&str> = page.split_whitespace().collect();
    let lines: Vec<String> = words.chunks(8).map(|line| format!("({}) Tj 0 -14 Td", line.join(" "))).collect();
    let content = format!("BT /F1 10 Tf 40 760 Td {} ET", lines.join(" "));
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 6 0 R >> >> /Annots [5 0 R] >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (Nombre) /V ({name}) /Rect [40 700 300 720] /P 3 0 R /F 4 >>"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
    }
    let xref = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for offset in offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
    pdf.into_bytes()
}

/// Invented words made of syllables: pure wording, never data. Each
/// `salt` gives words no other salt has.
fn words(range: std::ops::Range<usize>, salt: usize) -> String {
    const SYLLABLES: [&str; 10] = ["ba", "ce", "di", "fo", "gu", "la", "me", "ni", "po", "ru"];
    range
        .map(|n| format!("{}{}{}{}s", SYLLABLES[salt % 10], SYLLABLES[n % 10], SYLLABLES[(n / 10) % 10], SYLLABLES[(n / 100) % 10]))
        .collect::<Vec<_>>()
        .join(" ")
}

fn settle(base: &DocBase, origin_id: &str) {
    base.scan_origin(origin_id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    while base.similar_step(10).unwrap() > 0 {}
}

fn row(base: &DocBase, name: &str) -> crate::docbase::query::DocumentRow {
    base.documents(&Visible::All, &DocumentFilter::default())
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("no document named {name}"))
}

fn content(base: &DocBase, name: &str) -> i64 {
    row(base, name).content_id.unwrap()
}

fn pair(base: &DocBase, a: &str, b: &str) -> Option<super::SimilarDocument> {
    base.similar_of(&Visible::All, row(base, a).location_id)
        .unwrap()
        .into_iter()
        .find(|similar| similar.other.name == b)
}

fn relation(base: &DocBase, a: &str, b: &str) -> Option<String> {
    pair(base, a, b).map(|similar| similar.relation)
}

fn base_with(files: &[(&str, Vec<u8>)]) -> (TempDir, DocBase, String) {
    let dir = TempDir::new();
    for (name, bytes) in files {
        dir.write(&format!("o/{name}"), bytes);
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);
    (dir, base, origin.id)
}

#[test]
fn the_same_form_for_two_people_is_the_same_template() {
    let page = "Ficha de cliente persona humana. Complete todos los campos con los datos de la persona, indique domicilio \
                real y constituido, actividad principal, origen de los fondos, si reviste la condicion de persona expuesta \
                politicamente y firme al pie de cada hoja de este formulario.";
    let (_dir, base, _) = base_with(&[
        ("ficha uno.pdf", filled_form(page, "Ana Torres")),
        ("ficha dos.pdf", filled_form(page, "Bruno Vega")),
    ]);
    let similar = pair(&base, "ficha uno.pdf", "ficha dos.pdf").expect("the two forms are related");
    assert_eq!(similar.relation, "same-template");
    assert!(similar.description.contains("cambia un nombre"), "{}", similar.description);
    let marks: Vec<String> = serde_json::from_value(similar.evidence["marks"]["a"].clone()).unwrap();
    assert!(marks.contains(&"form".to_string()));
}

#[test]
fn a_draft_and_its_final_wording_are_versions() {
    let (_dir, base, _) = base_with(&[("borrador.md", CONTRACT.as_bytes().to_vec()), ("final.md", contract_final().into_bytes())]);
    let similar = pair(&base, "borrador.md", "final.md").unwrap();
    assert_eq!(similar.relation, "version");
    assert!(similar.description.starts_with("Mismo texto de base; cambian la redacción en"), "{}", similar.description);
    assert!(similar.cover >= 0.8 && similar.cover < 0.97, "{}", similar.cover);
    assert_eq!(similar.evidence["changes"]["dataTokens"], 0);
}

#[test]
fn a_copy_with_a_signature_history_is_the_signed_copy() {
    let signed = format!("## Page 1\n\n{}\n\n## Page 2\n\n{AUDIT_PAGE}\n", contract_final());
    let (_dir, base, _) = base_with(&[
        ("borrador.md", CONTRACT.as_bytes().to_vec()),
        ("final.md", contract_final().into_bytes()),
        ("final firmado.md", signed.into_bytes()),
    ]);
    let similar = pair(&base, "final.md", "final firmado.md").unwrap();
    assert_eq!(similar.relation, "signed-copy");
    let signed_id = content(&base, "final firmado.md");
    assert_eq!(similar.subject_content_id, Some(signed_id));
    assert_eq!(similar.suggested_source, Some(signed_id));
    // The draft is not the signed copy of anything: its text changed.
    assert_eq!(relation(&base, "borrador.md", "final firmado.md").as_deref(), Some("version"));
}

#[test]
fn a_seal_without_a_party_is_not_a_signed_copy() {
    let history = format!("## Page 1\n\n{}\n\n## Page 2\n\nHistorial\nFinal audit report\nTransaction ID: QWERTY\n", contract_final());
    let (_dir, base, _) = base_with(&[("final.md", contract_final().into_bytes()), ("enviado.md", history.into_bytes())]);
    assert_eq!(relation(&base, "final.md", "enviado.md").as_deref(), Some("near-identical"));
}

#[test]
fn the_postal_preview_is_a_preview() {
    let preview = format!("CARTA DOCUMENTO\nVISTA PREVIA - SIN VALOR LEGAL\nREMITENTE\nDESTINATARIO\n{LETTER}");
    let (_dir, base, _) = base_with(&[("carta.md", LETTER.as_bytes().to_vec()), ("vista previa.md", preview.into_bytes())]);
    let similar = pair(&base, "carta.md", "vista previa.md").unwrap();
    assert_eq!(similar.relation, "preview");
    assert_eq!(similar.subject_content_id, Some(content(&base, "vista previa.md")));
}

#[test]
fn invoices_that_differ_only_in_figures_share_a_template() {
    let (_dir, base, _) = base_with(&[("marzo.md", march().into_bytes()), ("abril.md", april().into_bytes())]);
    let similar = pair(&base, "marzo.md", "abril.md").unwrap();
    assert_eq!(similar.relation, "same-template");
    assert!(similar.description.contains("importes"), "{}", similar.description);
    assert!(similar.description.contains("fechas"), "{}", similar.description);
    assert!(similar.description.contains("un identificador"), "{}", similar.description);
}

#[test]
fn unrelated_documents_are_not_shown() {
    let (_dir, base, _) = base_with(&[("carta.md", LETTER.as_bytes().to_vec()), ("contrato.md", CONTRACT.as_bytes().to_vec())]);
    assert!(pair(&base, "carta.md", "contrato.md").is_none());
}

#[test]
fn a_chain_does_not_make_one_family() {
    // b changes 15 % of a; c changes another 15 % of b: c is not like a.
    let a = words(0..110, 0);
    let b = [words(0..20, 0), words(20..35, 1), words(35..100, 0)].join(" ");
    let c = [words(0..20, 0), words(20..35, 1), words(35..60, 0), words(60..75, 2), words(75..100, 0)].join(" ");
    let (_dir, base, _) = base_with(&[("a.md", a.into_bytes()), ("b.md", b.into_bytes()), ("c.md", c.into_bytes())]);
    assert_eq!(relation(&base, "a.md", "b.md").as_deref(), Some("version"));
    assert_eq!(relation(&base, "b.md", "c.md").as_deref(), Some("version"));
    assert_eq!(relation(&base, "a.md", "c.md").as_deref(), Some("similar"));

    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    assert_eq!(group.principal.name, "a.md");
    let members: Vec<&str> = group.members.iter().map(|m| m.doc.name.as_str()).collect();
    assert_eq!(members, ["a.md", "b.md"]);
    assert_eq!(group.flags, ["dispersed"]);
    assert_eq!(group.pending, 1);
}

#[test]
fn a_discarded_pair_is_not_proposed_again() {
    let (_dir, base, _) = base_with(&[("borrador.md", CONTRACT.as_bytes().to_vec()), ("final.md", contract_final().into_bytes())]);
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 1);
    assert_eq!(base.similar_pending(&Visible::All).unwrap().groups, 1);
    let similar = pair(&base, "borrador.md", "final.md").unwrap();
    base.decide_similar(similar.content_id, similar.other.content_id, &Decision::Discard, Some(&similar.evidence_hash))
        .unwrap();
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 0);

    // Everything computed again from the texts: the decision stays.
    base.similar_reset().unwrap();
    while base.similar_step(10).unwrap() > 0 {}
    let again = pair(&base, "borrador.md", "final.md").unwrap();
    assert_eq!(again.state, "discarded");
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 0);
    assert!(base.similar_groups(&Visible::All).unwrap().is_empty());

    // And it can be undone.
    base.decide_similar(similar.content_id, similar.other.content_id, &Decision::Undo, None).unwrap();
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 1);
}

#[test]
fn joining_as_one_document_needs_the_wiki_source() {
    let (_dir, base, _) = base_with(&[("borrador.md", CONTRACT.as_bytes().to_vec()), ("final.md", contract_final().into_bytes())]);
    let similar = pair(&base, "borrador.md", "final.md").unwrap();
    let (me, other) = (similar.content_id, similar.other.content_id);
    let version = |source| Decision::Join { relation: "version".into(), wiki_source: source };
    assert!(base.decide_similar(me, other, &version(None), None).is_err());
    assert!(base.decide_similar(me, other, &version(Some(-1)), None).is_err());
    assert!(base
        .decide_similar(me, other, &Decision::Join { relation: "no-such-relation".into(), wiki_source: None }, None)
        .is_err());
    base.decide_similar(me, other, &version(Some(other)), Some("shown")).unwrap();
    let joined = pair(&base, "borrador.md", "final.md").unwrap();
    assert_eq!(joined.state, "joined");
    assert_eq!(joined.chosen_relation.as_deref(), Some("version"));
    assert_eq!(joined.wiki_source, Some(other));
    let shown: String = base
        .conn()
        .query_row("SELECT shown_evidence_hash FROM similar_pair", [], |row| row.get(0))
        .unwrap();
    assert_eq!(shown, "shown");
    // A relation between different documents keeps both in the wiki.
    assert!(base
        .decide_similar(me, other, &Decision::Join { relation: "same-template".into(), wiki_source: Some(me) }, None)
        .is_err());
    // A decision arrives from the screens as JSON.
    let parsed: Decision = serde_json::from_str(r#"{"kind": "join", "relation": "version", "wikiSource": 7}"#).unwrap();
    assert_eq!(parsed, Decision::Join { relation: "version".into(), wiki_source: Some(7) });
}

/// The paths of the general project's wiki sources, sorted.
fn wiki_paths(base: &DocBase) -> Vec<String> {
    let mut paths: Vec<String> = base.wiki_sources().unwrap().into_iter().map(|source| source.source_path).collect();
    paths.sort();
    paths
}

#[test]
fn joined_as_one_document_only_the_chosen_one_feeds_the_wiki() {
    let (dir, base, origin) = base_with(&[
        ("borrador.md", CONTRACT.as_bytes().to_vec()),
        ("final.md", contract_final().into_bytes()),
        ("copias/borrador otra vez.md", CONTRACT.as_bytes().to_vec()),
        ("marzo.md", march().into_bytes()),
        ("abril.md", april().into_bytes()),
    ]);
    let project = dir.folder("General");
    dir.folder("General/wiki");
    dir.folder("General/raw/sources");
    dir.write("General/schema.md", b"# schema");
    base.link_general_project(&project, "General").unwrap();
    let everything = [
        "raw/sources/O/abril.md",
        "raw/sources/O/borrador.md",
        "raw/sources/O/copias/borrador otra vez.md",
        "raw/sources/O/final.md",
        "raw/sources/O/marzo.md",
    ];
    assert_eq!(wiki_paths(&base), everything);
    let asked: Vec<String> = everything.iter().map(|path| path.to_string()).collect();
    assert!(base.excluded_sources(&project, &asked).unwrap().is_empty());
    // Every source says which document it is.
    assert!(base.wiki_sources().unwrap().iter().all(|source| source.content_id.is_some()));

    // Joined as versions, with the final wording as the wiki's source.
    let draft = content(&base, "borrador.md");
    let last = content(&base, "final.md");
    let version = Decision::Join { relation: "version".into(), wiki_source: Some(last) };
    base.decide_similar(draft, last, &version, None).unwrap();
    // The draft leaves the wiki's sources, in every place it is.
    assert_eq!(wiki_paths(&base), ["raw/sources/O/abril.md", "raw/sources/O/final.md", "raw/sources/O/marzo.md"]);
    // Whoever asks to ingest it (the watcher, a person) is told to leave it out.
    assert_eq!(
        base.excluded_sources(&project, &asked).unwrap(),
        ["raw/sources/O/borrador.md", "raw/sources/O/copias/borrador otra vez.md"]
    );
    // Only for the general project's wiki.
    assert!(base.excluded_sources(&dir.path().join("Otro"), &asked).unwrap().is_empty());
    // Nothing was touched on disk.
    assert!(dir.path().join("o/borrador.md").is_file());

    // A relation between different documents keeps both.
    let (one, two) = (content(&base, "marzo.md"), content(&base, "abril.md"));
    base.decide_similar(one, two, &Decision::Join { relation: "same-template".into(), wiki_source: None }, None)
        .unwrap();
    assert_eq!(wiki_paths(&base).len(), 3);

    // Choosing the other one swaps them; undoing offers both again.
    base.decide_similar(draft, last, &Decision::Join { relation: "version".into(), wiki_source: Some(draft) }, None)
        .unwrap();
    assert_eq!(
        wiki_paths(&base),
        ["raw/sources/O/abril.md", "raw/sources/O/borrador.md", "raw/sources/O/copias/borrador otra vez.md", "raw/sources/O/marzo.md"]
    );
    base.decide_similar(draft, last, &Decision::Undo, None).unwrap();
    assert_eq!(wiki_paths(&base), everything);
    assert!(base.excluded_sources(&project, &asked).unwrap().is_empty());

    // Measured again from the texts, the decision still holds.
    base.decide_similar(draft, last, &version, None).unwrap();
    base.similar_reset().unwrap();
    settle(&base, &origin);
    assert!(!wiki_paths(&base).contains(&"raw/sources/O/borrador.md".to_string()));
}

#[test]
fn a_new_document_only_measures_its_neighbourhood() {
    let (dir, base, origin) = base_with(&[("borrador.md", CONTRACT.as_bytes().to_vec()), ("final.md", contract_final().into_bytes())]);
    let before: i64 = base
        .conn()
        .query_row("SELECT computed_at FROM similar_pair", [], |row| row.get(0))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));

    dir.write("o/marzo.md", march().as_bytes());
    dir.write("o/abril.md", april().as_bytes());
    settle(&base, &origin);

    let after: i64 = base
        .conn()
        .query_row("SELECT computed_at FROM similar_pair WHERE relation = 'version'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(before, after, "the contracts' pair was not measured again");
    assert_eq!(relation(&base, "marzo.md", "abril.md").as_deref(), Some("same-template"));

    // A document that goes away takes its pairs and its shingles along.
    let counted_with: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM similar_common", [], |row| row.get(0))
        .unwrap();
    dir.remove("o/abril.md");
    settle(&base, &origin);
    assert!(pair(&base, "marzo.md", "abril.md").is_none());
    let (signatures, counted): (i64, i64) = base
        .conn()
        .query_row(
            "SELECT (SELECT count(*) FROM similar_signature), (SELECT count(*) FROM similar_common)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(signatures, 3);
    assert!(counted > 0 && counted < counted_with);
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 1);
}

#[test]
fn the_documents_screen_filters_by_pending_similar_and_by_family() {
    let (_dir, base, _) = base_with(&[
        ("borrador.md", CONTRACT.as_bytes().to_vec()),
        ("final.md", contract_final().into_bytes()),
        ("carta.md", LETTER.as_bytes().to_vec()),
    ]);
    let names = |filter: DocumentFilter| -> Vec<String> {
        let mut names: Vec<String> =
            base.documents(&Visible::All, &filter).unwrap().rows.into_iter().map(|row| row.name).collect();
        names.sort();
        names
    };
    let pending = DocumentFilter { similar_pending: true, ..Default::default() };
    assert_eq!(names(pending.clone()), ["borrador.md", "final.md"]);

    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!(groups.len(), 1);
    let family = DocumentFilter { similar_group: Some(groups[0].principal.content_id), ..Default::default() };
    assert_eq!(names(family.clone()), ["borrador.md", "final.md"]);

    // Once decided, nothing waits; the family stays.
    let similar = pair(&base, "borrador.md", "final.md").unwrap();
    base.decide_similar(
        similar.content_id,
        similar.other.content_id,
        &Decision::Join { relation: "version".into(), wiki_source: Some(similar.other.content_id) },
        Some(&similar.evidence_hash),
    )
    .unwrap();
    assert!(names(pending).is_empty());
    assert_eq!(names(family), ["borrador.md", "final.md"]);
    // The filter arrives from the screens as JSON.
    let parsed: DocumentFilter = serde_json::from_str(r#"{"similarPending": true, "similarGroup": 4}"#).unwrap();
    assert!(parsed.similar_pending);
    assert_eq!(parsed.similar_group, Some(4));
}
