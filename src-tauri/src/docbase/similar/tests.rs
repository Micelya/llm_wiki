//! The passes over a real base, with invented documents (R-24: nothing
//! here comes from the test folder).

use rusqlite::OptionalExtension;

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
    // Two different documents made with one template: it waits for no one
    // (D-497); the family and its template stay.
    let pending = base.similar_pending(&Visible::All).unwrap();
    assert_eq!((pending.pairs, pending.groups, pending.same_document_pairs), (0, 0, 0));
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!((groups[0].members.len(), groups[0].pending, groups[0].pending_same_document), (2, 0, 0));
}

#[test]
fn a_draft_and_its_final_wording_are_versions() {
    let (_dir, base, _) = base_with(&[("borrador.md", CONTRACT.as_bytes().to_vec()), ("final.md", contract_final().into_bytes())]);
    let similar = pair(&base, "borrador.md", "final.md").unwrap();
    assert_eq!(similar.relation, "version");
    assert!(similar.description.starts_with("Mismo texto de base; cambian la redacción en"), "{}", similar.description);
    assert!(similar.cover >= 0.8 && similar.cover < 0.97, "{}", similar.cover);
    assert_eq!(similar.evidence["changes"]["dataTokens"], 0);
    // Versions are one document: deciding them changes what the wiki reads.
    let pending = base.similar_pending(&Visible::All).unwrap();
    assert_eq!((pending.pairs, pending.same_document_pairs), (1, 1));
    assert_eq!(base.similar_groups(&Visible::All).unwrap()[0].pending_same_document, 1);
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

#[test]
fn choices_that_close_a_circle_are_refused() {
    // Three versions, each a few words away from the others.
    let a = words(0..120, 0);
    let b = [words(0..30, 0), words(30..34, 1), words(34..120, 0)].join(" ");
    let c = [words(0..80, 0), words(80..84, 2), words(84..120, 0)].join(" ");
    let (_dir, base, _) = base_with(&[("a.md", a.into_bytes()), ("b.md", b.into_bytes()), ("c.md", c.into_bytes())]);
    let (a, b, c) = (content(&base, "a.md"), content(&base, "b.md"), content(&base, "c.md"));
    for (x, y) in [("a.md", "b.md"), ("b.md", "c.md"), ("a.md", "c.md")] {
        assert!(pair(&base, x, y).is_some(), "{x} and {y} are not similar");
    }
    let choose = |source| Decision::Join { relation: "version".into(), wiki_source: Some(source) };

    // A gives way to B, B to C: a chain, C is the one chosen.
    base.decide_similar(a, b, &choose(b), None).unwrap();
    base.decide_similar(b, c, &choose(c), None).unwrap();
    // C giving way to A would close the circle: refused, nothing changes.
    let refused = base.decide_similar(c, a, &choose(a), None).unwrap_err();
    assert!(refused.contains("ninguno de los 3 documentos"), "{refused}");
    assert_eq!(pair(&base, "a.md", "c.md").unwrap().state, "proposed");
    // Choosing C is fine.
    base.decide_similar(c, a, &choose(c), None).unwrap();
    // Changing an earlier choice is checked too: A over C closes it now.
    assert!(base.decide_similar(a, c, &choose(a), None).is_err());
    assert_eq!(pair(&base, "a.md", "c.md").unwrap().wiki_source, Some(c));
    // B over C, then A over C, is fine.
    base.decide_similar(b, c, &choose(b), None).unwrap();
    base.decide_similar(a, c, &choose(a), None).unwrap();
    // A relation between different documents is no choice of one.
    base.decide_similar(a, b, &Decision::Join { relation: "same-template".into(), wiki_source: None }, None).unwrap();
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

#[test]
fn a_family_of_the_same_template_waits_for_no_one_and_can_still_be_confirmed_at_once() {
    let may = invoice("0003-00001377", "15/05/2026", "01/04/2026", "30/04/2026", "98.000,00", "20.580,00", "118.580,00", "25/05/2026");
    let (_dir, base, _) = base_with(&[
        ("marzo.md", march().into_bytes()),
        ("abril.md", april().into_bytes()),
        ("mayo.md", may.into_bytes()),
    ]);
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!(groups.len(), 1, "the three invoices are one family");
    let group = &groups[0];
    // Three invoices of one template are three invoices: nothing to decide (D-497).
    assert_eq!((group.members.len(), group.pending, group.pending_same_document), (3, 0, 0));
    assert!(base.similar_open_pairs(&Visible::All).unwrap().is_empty(), "the pair between the two members waits for no one");
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 0);

    // A person may still confirm it: every member with its principal, in one go.
    let principal = group.principal.content_id;
    let decisions: Vec<super::PairDecision> = base
        .similar_of(&Visible::All, group.principal.location_id)
        .unwrap()
        .into_iter()
        .filter(|similar| similar.state == "proposed" && similar.relation == "same-template")
        .map(|similar| super::PairDecision {
            content_id: principal,
            other_content_id: similar.other.content_id,
            decision: Decision::Join { relation: "same-template".into(), wiki_source: None },
            evidence_hash: Some(similar.evidence_hash),
        })
        .collect();
    assert_eq!(decisions.len(), 2);
    assert_eq!(base.decide_similar_many(&decisions).unwrap(), 2);
    let after = base.similar_groups(&Visible::All).unwrap();
    assert_eq!((after[0].members.len(), after[0].pending), (3, 0));
    let joined = base.similar_of(&Visible::All, group.principal.location_id).unwrap();
    assert!(joined.iter().filter(|s| s.relation == "same-template").all(|s| s.state == "joined"), "what the person decided stays");
    assert!(base.similar_open_pairs(&Visible::All).unwrap().is_empty());

    // A batch that fails part way keeps what it saved and says so.
    let bad = vec![super::PairDecision {
        content_id: principal,
        other_content_id: 999_999,
        decision: Decision::Discard,
        evidence_hash: None,
    }];
    let error = base.decide_similar_many(&bad).unwrap_err();
    assert!(error.starts_with("Se guardaron 0 de 1"), "{error}");
}

#[test]
fn a_family_of_the_same_template_has_its_template_and_follows_its_family() {
    let may = invoice("0003-00001377", "15/05/2026", "01/04/2026", "30/04/2026", "98.000,00", "20.580,00", "118.580,00", "25/05/2026");
    let (_dir, base, _) = base_with(&[
        ("marzo.md", march().into_bytes()),
        ("abril.md", april().into_bytes()),
        ("mayo.md", may.into_bytes()),
    ]);
    let group = base.similar_groups(&Visible::All).unwrap().remove(0);
    let template = base
        .similar_template(&Visible::All, row(&base, "abril.md").location_id)
        .unwrap()
        .expect("a member reaches the template of its family");
    assert_eq!(template.principal, group.principal.content_id);
    assert_eq!(template.asked, content(&base, "abril.md"));
    assert_eq!(template.state, "proposed", "nobody confirmed the family");
    assert_eq!(template.documents.len(), 3);
    assert_eq!(template.documents[0].doc.content_id, template.principal);
    assert_eq!(group.template_fields, Some(template.fields.len() as i64));
    // The number, four dates and three amounts change; the rest is the template.
    let kinds: Vec<String> = template
        .fields
        .iter()
        .map(|f| format!("{}{}", serde_json::to_value(f.kind).unwrap().as_str().unwrap(), f.ordinal))
        .collect();
    assert_eq!(kinds, ["identifier1", "date1", "date2", "date3", "amount1", "amount2", "amount3", "date4"]);
    let text: String = template
        .segments
        .iter()
        .filter_map(|s| match s {
            super::template::Segment::Text { text } => Some(text.as_str()),
            super::template::Segment::Field { .. } => None,
        })
        .collect();
    assert!(text.contains("Cliente: Ejemplo Tres S.A., CUIT 30-11111111-1."), "{text}");
    assert!(!text.contains("120.000,00") && !text.contains("15/03/2026"), "{text}");
    let column = template.documents.iter().position(|d| d.doc.name == "marzo.md").unwrap();
    assert_eq!(template.fields[0].values[column], "0003-00001234");
    assert_eq!(template.fields[4].values[column], "120.000,00");
    assert!(template.possibly_same.is_empty());

    // Confirming the family makes it a confirmed template; nothing is built again.
    let principal = template.principal;
    let decisions: Vec<super::PairDecision> = template.documents[1..]
        .iter()
        .map(|d| super::PairDecision {
            content_id: principal,
            other_content_id: d.doc.content_id,
            decision: Decision::Join { relation: "same-template".into(), wiki_source: None },
            evidence_hash: None,
        })
        .collect();
    base.decide_similar_many(&decisions).unwrap();
    let confirmed = base.similar_template(&Visible::All, row(&base, "marzo.md").location_id).unwrap().unwrap();
    assert_eq!(confirmed.state, "confirmed");
    assert_eq!(confirmed.computed_at, template.computed_at);
    assert_eq!(base.similar_templates_refresh().unwrap(), 0, "nothing changed");

    // A member taken out of the family leaves the template.
    let gone = &template.documents[2].doc;
    base.decide_similar(principal, gone.content_id, &Decision::Discard, None).unwrap();
    let after = base.similar_template(&Visible::All, template.documents[0].doc.location_id).unwrap().unwrap();
    assert_eq!(after.documents.len(), 2);
    assert!(after.documents.iter().all(|d| d.doc.content_id != gone.content_id));
    assert!(base.similar_template(&Visible::All, gone.location_id).unwrap().is_none());
}

/// Three invented receipts of one template, with `tag` in their fixed text
/// so two sets of them are two templates.
fn receipts(tag: &str) -> Vec<(String, Vec<u8>)> {
    [("Persona Uno", "1.500,00", "10/01/2026"), ("Persona Dos", "2.750,00", "10/02/2026"), ("Persona Tres", "980,50", "10/03/2026")]
        .iter()
        .enumerate()
        .map(|(i, (who, amount, date))| {
            let body = format!(
                "RECIBO {tag}. Recibí de {who} la suma de pesos $ {amount} en concepto de honorarios profesionales por la \
                 consulta del día {date}, según lo convenido entre las partes en el acuerdo de servicios vigente. {}",
                words(0..30, tag.len())
            );
            (format!("recibo {tag} {i}.md"), body.into_bytes())
        })
        .collect()
}

#[test]
fn the_section_lists_every_template_with_its_documents_fields_and_type() {
    let may = invoice("0003-00001377", "15/05/2026", "01/04/2026", "30/04/2026", "98.000,00", "20.580,00", "118.580,00", "25/05/2026");
    let mut files: Vec<(String, Vec<u8>)> =
        vec![("marzo.md".into(), march().into_bytes()), ("abril.md".into(), april().into_bytes()), ("mayo.md".into(), may.into_bytes())];
    files.extend(receipts("A"));
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let (_dir, base, _) = base_with(&refs);
    let list = base.templates(&Visible::All).unwrap();
    assert_eq!(list.len(), 2, "{list:?}");
    let invoices = list.iter().find(|t| t.fields == 8).expect("the invoices' template");
    assert_eq!(invoices.documents, 3);
    assert_eq!(invoices.wording_fields, 0);
    assert_eq!(invoices.state, "proposed");
    assert_eq!(invoices.format, "text", "a file of plain text");
    assert_eq!(invoices.possibly_same, 0);
    // Without a type, it is named by nothing yet.
    assert_eq!(invoices.kind, super::template::TemplateKind::default());

    // Two templates whose principals have the same type are told apart by their order.
    for template in &list {
        base.conn()
            .execute(
                "INSERT INTO document_type (content_id, doc_type, source, basis, computed_at) VALUES (?1, 'recibo', 'model', 'x', 1)",
                [template.principal.content_id],
            )
            .unwrap();
    }
    let list = base.templates(&Visible::All).unwrap();
    let mut ranks: Vec<Option<i64>> = list.iter().map(|t| t.kind.type_rank).collect();
    ranks.sort();
    assert_eq!(ranks, [Some(1), Some(2)]);
    assert!(list.iter().all(|t| t.kind.doc_type.as_deref() == Some("recibo")));
    // The detail says the same.
    let detail = base.similar_template(&Visible::All, list[0].principal.location_id).unwrap().unwrap();
    assert_eq!(detail.kind, list[0].kind);
    assert_eq!(detail.format, "text");
}

#[test]
fn what_a_person_names_survives_every_build_of_the_template_even_with_another_principal() {
    use super::template::FieldChoice;
    let may = invoice("0003-00001377", "15/05/2026", "01/04/2026", "30/04/2026", "98.000,00", "20.580,00", "118.580,00", "25/05/2026");
    let (dir, base, origin) = base_with(&[
        ("marzo.md", march().into_bytes()),
        ("abril.md", april().into_bytes()),
        ("mayo.md", may.into_bytes()),
    ]);
    let first = base.templates(&Visible::All).unwrap().remove(0);
    let principal = first.principal.content_id;
    assert_eq!(first.name, None);
    base.name_template(principal, Some("  Factura mensual ")).unwrap();
    let choose = |position: i64, choice: FieldChoice| base.choose_template_field(principal, position, &choice);
    choose(0, FieldChoice { name: Some("Número de factura".into()), ..Default::default() }).unwrap();
    choose(4, FieldChoice { what: Some("importe".into()), fixed: true, fixed_value: Some("100,00".into()), ..Default::default() }).unwrap();
    // What it is must be something a field can be; a fixed one needs its value.
    assert!(choose(1, FieldChoice { what: Some("planeta".into()), ..Default::default() }).is_err());
    assert!(choose(1, FieldChoice { fixed: true, ..Default::default() }).is_err());
    assert!(base.choose_template_field(principal, 99, &FieldChoice::default()).is_err());

    let shown = |base: &crate::docbase::DocBase| {
        let list = base.templates(&Visible::All).unwrap();
        let template = base.similar_template(&Visible::All, list[0].principal.location_id).unwrap().unwrap();
        (list[0].name.clone(), template)
    };
    let (name, template) = shown(&base);
    assert_eq!(name.as_deref(), Some("Factura mensual"));
    assert_eq!(template.name.as_deref(), Some("Factura mensual"));
    assert!(template.whats.iter().any(|w| w == "persona") && template.whats.iter().any(|w| w == "fecha"));
    let number = &template.fields[0];
    assert_eq!((number.name.as_deref(), number.name_source.as_deref()), (Some("Número de factura"), Some("person")));
    assert_eq!((number.what.as_deref(), number.what_source.as_deref()), (Some("identificador"), Some("kind")));
    let net = &template.fields[4];
    assert_eq!((net.what.as_deref(), net.what_source.as_deref()), (Some("importe"), Some("person")));
    assert!(net.fixed);
    assert_eq!(net.fixed_value.as_deref(), Some("100,00"));
    // A field nobody named has the name its label gives it; the person's
    // name goes over the label.
    let issued = &template.fields[1];
    assert_eq!((issued.name.as_deref(), issued.name_source.as_deref()), (Some("Fecha de emisión"), Some("label")));

    // Built again with nothing changed: everything stays.
    base.conn().execute("UPDATE similar_template SET fingerprint = 'otra'", []).unwrap();
    assert_eq!(base.similar_templates_refresh().unwrap(), 1);
    let (name, again) = shown(&base);
    assert_eq!(name.as_deref(), Some("Factura mensual"));
    assert_eq!(again.fields[0].name.as_deref(), Some("Número de factura"));

    // A longer invoice joins the family and becomes its principal: the
    // template is built again under it, and what the person named follows.
    let june = format!(
        "{} Gracias por abonar en término; ante cualquier consulta comuníquese con la administración.",
        invoice("0003-00001455", "15/06/2026", "01/05/2026", "31/05/2026", "101.000,00", "21.210,00", "122.210,00", "25/06/2026")
    );
    dir.write("o/junio.md", june.as_bytes());
    settle(&base, &origin);
    let (name, after) = shown(&base);
    assert_eq!(after.documents.len(), 4);
    assert_ne!(after.principal, principal, "the longest invoice is the principal now");
    assert_eq!(name.as_deref(), Some("Factura mensual"));
    let number = after.fields.iter().find(|f| f.name.as_deref() == Some("Número de factura")).expect("the number keeps its name");
    assert_eq!(number.values.iter().filter(|v| v.starts_with("0003-")).count(), 4);
    assert!(after.fields.iter().any(|f| f.fixed && f.fixed_value.as_deref() == Some("100,00")));

    // Back to the automatic name and field.
    let principal = after.principal;
    base.name_template(principal, None).unwrap();
    let position = after.fields.iter().position(|f| f.name.as_deref() == Some("Número de factura")).unwrap() as i64;
    base.choose_template_field(principal, position, &FieldChoice::default()).unwrap();
    let (name, cleared) = shown(&base);
    assert_eq!(name, None);
    assert!(cleared.fields.iter().all(|f| f.name.as_deref() != Some("Número de factura")));
    // Back to its label.
    assert_eq!(cleared.fields[position as usize].name.as_deref(), Some("Factura B N°"));
}

/// How many fields of invented templates the text names at no cost: the
/// labels of an invoice and the names of a form; a receipt whose fields
/// are all in the middle of a sentence leaves them to the model.
#[test]
fn the_text_names_most_fields_of_an_invoice_and_a_form_and_none_in_a_sentence() {
    let may = invoice("0003-00001377", "15/05/2026", "01/04/2026", "30/04/2026", "98.000,00", "20.580,00", "118.580,00", "25/05/2026");
    let form = |date: &str| {
        format!(
            "Ficha de cliente persona humana. Fecha de alta: {date}. Complete todos los campos con los datos de la persona, \
             indique domicilio real y constituido, actividad principal, origen de los fondos y firme al pie de cada hoja."
        )
    };
    let mut files: Vec<(String, Vec<u8>)> =
        vec![("marzo.md".into(), march().into_bytes()), ("abril.md".into(), april().into_bytes()), ("mayo.md".into(), may.into_bytes())];
    files.extend(receipts("B"));
    files.push(("ficha uno.pdf".into(), filled_form(&form("01/02/2026"), "Persona Uno")));
    files.push(("ficha dos.pdf".into(), filled_form(&form("05/06/2026"), "Persona Dos")));
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let (_dir, base, _) = base_with(&refs);
    let mut named = 0;
    let mut total = 0;
    let mut by_template: Vec<(usize, usize)> = Vec::new();
    for summary in base.templates(&Visible::All).unwrap() {
        let template = base.similar_template(&Visible::All, summary.principal.location_id).unwrap().unwrap();
        let fields: Vec<_> = template.fields.iter().filter(|f| f.same_as.is_none()).collect();
        let free = fields.iter().filter(|f| f.name_source.as_deref() == Some("label")).count();
        by_template.push((free, fields.len()));
        named += free;
        total += fields.len();
    }
    by_template.sort();
    // The receipt: none of 3; the form: its date and its name, 2 of 2; the invoice: 7 of 8.
    assert_eq!(by_template, [(0, 3), (2, 2), (7, 8)]);
    assert_eq!((named, total), (9, 13));
}

/// The model names the fields the text does not, by itself, once per
/// template (a simulated engine, R-24): its answer is kept with its
/// version, a name that repeats a value is dropped, nothing is asked twice
/// and it survives a new build; a person's name goes over it.
#[test]
fn the_model_names_the_fields_without_a_name_once_and_a_person_goes_over_it() {
    use crate::docbase::knowledge::tests::FakeModel;
    use crate::docbase::progress::Scope;
    let files = receipts("C");
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let (_dir, base, _) = base_with(&refs);
    let principal = base.templates(&Visible::All).unwrap()[0].principal.clone();
    // Who pays, how much and the day: all in the middle of a sentence.
    assert_eq!(super::naming::waiting_templates(&base.conn()).unwrap(), [(principal.content_id, 3)]);
    assert_eq!(base.templates(&Visible::All).unwrap()[0].waiting_fields, 3);
    let stage = |base: &crate::docbase::DocBase| {
        let progress = base.progress(&Scope::default()).unwrap();
        let stage = progress.stages.into_iter().find(|s| s.id == "templates").unwrap();
        (stage.unit, stage.total, stage.done, stage.pending)
    };
    assert_eq!(stage(&base), ("templates", 1, 0, 1));

    let model = FakeModel::new("codex-cli:prueba", |prompt| {
        assert!(prompt.contains("[campo 1]") && prompt.contains("[campo 3]"), "{prompt}");
        serde_json::json!({ "campos": [
            { "campo": 1, "nombre": "Dos", "que_es": "persona" },
            { "campo": 2, "nombre": "Importe de los honorarios", "que_es": "importe" },
            { "campo": 3, "nombre": "Fecha de la consulta", "que_es": "Fecha" }
        ]})
        .to_string()
    });
    let report = base.templates_naming_run(&model, 5).unwrap();
    assert_eq!((report.asked, report.named, report.failed, report.left), (1, 2, 0, 0), "{:?}", report.errors);
    let template = base.similar_template(&Visible::All, principal.location_id).unwrap().unwrap();
    let names: Vec<(Option<&str>, Option<&str>, Option<&str>)> =
        template.fields.iter().map(|f| (f.name.as_deref(), f.name_source.as_deref(), f.what.as_deref())).collect();
    assert_eq!(
        names,
        [
            // "Dos" is what a document says there: not a name.
            (None, None, Some("persona")),
            (Some("Importe de los honorarios"), Some("model"), Some("importe")),
            (Some("Fecha de la consulta"), Some("model"), Some("fecha")),
        ]
    );
    // Kept with its version, as the other questions; «En curso» has it done.
    let kept: (i64, String) = base
        .conn()
        .query_row(
            "SELECT count(*), max(prompt_version) FROM model_answer WHERE task = ?1",
            [super::naming::FIELDS_TASK],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(kept, (1, super::naming::FIELDS_PROMPT.to_string()));
    assert!(base.last_model_question("template-fields").unwrap().unwrap().prompt.contains("[campo 2]"));
    assert_eq!(stage(&base), ("templates", 1, 1, 0));
    assert_eq!(base.templates(&Visible::All).unwrap()[0].waiting_fields, 0);

    // Nothing is asked again, not even after the template is built again.
    assert_eq!(base.templates_naming_run(&model, 5).unwrap().asked, 0);
    base.conn().execute("UPDATE similar_template SET fingerprint = 'otra'", []).unwrap();
    base.similar_templates_refresh().unwrap();
    assert_eq!(base.templates_naming_run(&model, 5).unwrap().asked, 0);
    assert_eq!(model.asked.lock().unwrap().len(), 1);
    let again = base.similar_template(&Visible::All, principal.location_id).unwrap().unwrap();
    assert_eq!(again.fields[2].name.as_deref(), Some("Fecha de la consulta"));

    // A person's name goes over the model's, and the model never writes over it.
    let choice = super::template::FieldChoice { name: Some("Día de la consulta".into()), ..Default::default() };
    base.choose_template_field(again.principal, 2, &choice).unwrap();
    let mine = base.similar_template(&Visible::All, principal.location_id).unwrap().unwrap();
    assert_eq!((mine.fields[2].name.as_deref(), mine.fields[2].name_source.as_deref()), (Some("Día de la consulta"), Some("person")));
    assert_eq!(mine.fields[2].what.as_deref(), Some("fecha"));
}

#[test]
fn a_provider_that_fails_leaves_the_template_for_later_and_stops_the_run() {
    use crate::source_volume::asking::DocumentAsker;
    struct AtItsLimit;
    impl DocumentAsker for AtItsLimit {
        fn engine_id(&self) -> String {
            "codex-cli:prueba".into()
        }
        fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
            Err("You've hit your usage limit".into())
        }
    }
    let mut files = receipts("D");
    files.extend(receipts("EEE"));
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let (_dir, base, _) = base_with(&refs);
    assert_eq!(base.templates(&Visible::All).unwrap().len(), 2);
    let report = base.templates_naming_run(&AtItsLimit, 5).unwrap();
    assert_eq!((report.asked, report.failed, report.left), (1, 1, 1));
    assert_eq!(super::naming::waiting_templates(&base.conn()).unwrap().iter().map(|(_, n)| n).sum::<usize>(), 6);
}

#[test]
fn two_copies_of_the_same_form_are_possibly_the_same_document() {
    let page = |date: &str| {
        format!(
            "Ficha de cliente persona humana. Fecha de alta: {date}. Complete todos los campos con los datos de la persona,              indique domicilio real y constituido, actividad principal, origen de los fondos, si reviste la condicion de              persona expuesta politicamente y firme al pie de cada hoja de este formulario."
        )
    };
    // The longest is the principal; the other two are the same person's,
    // one word apart.
    let (_dir, base, _) = base_with(&[
        ("ficha uno.pdf", filled_form(&page("01/02/2026"), "Ana Torres")),
        ("ficha dos.pdf", filled_form(&format!("{} Muchas gracias por su tiempo.", page("05/06/2026")), "Bruno Vega")),
        ("ficha tres.pdf", filled_form(&page("01/02/2026").replace("al pie", "al final"), "Ana Torres")),
    ]);
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].principal.name, "ficha dos.pdf");
    let template = base.similar_template(&Visible::All, groups[0].principal.location_id).unwrap().unwrap();
    assert_eq!(template.documents.len(), 3);
    // The name of the form is a field, beside those of the text.
    let form: Vec<_> = template.fields.iter().filter(|f| f.form_name.is_some()).collect();
    assert_eq!(form.len(), 1);
    assert_eq!(form[0].form_name.as_deref(), Some("Nombre"));
    assert_eq!(template.format, "pdf-form");
    // The two forms of the same person change nothing but a word: possibly the same document.
    let (uno, tres) = (content(&base, "ficha uno.pdf"), content(&base, "ficha tres.pdf"));
    assert_eq!(template.possibly_same, vec![[uno.min(tres), uno.max(tres)]]);
    assert_eq!(groups[0].possibly_same, 1);
    // Made with one template they would wait for no one; possibly the same
    // document, they wait for a person (D-497).
    let flagged = pair(&base, "ficha uno.pdf", "ficha tres.pdf").expect("the two forms are a pair");
    assert!(flagged.possibly_same);
    assert!(!pair(&base, "ficha dos.pdf", "ficha uno.pdf").unwrap().possibly_same);
    let open = base.similar_open_pairs(&Visible::All).unwrap();
    assert_eq!(open.len(), 1, "between two members, out of the principal's pairs");
    assert!(open[0].similar.possibly_same);
    // Once a person decides about the two, it is no longer shown.
    base.decide_similar(uno, tres, &Decision::Discard, None).unwrap();
    let template = base.similar_template(&Visible::All, groups[0].principal.location_id).unwrap().unwrap();
    assert!(template.possibly_same.is_empty());
}

// ── Documents generated from a template (D-488) ────────────────────────

/// Three invented receipts of one template, from three invented people.
fn fees() -> Vec<(String, Vec<u8>)> {
    [("Lucía Benítez", "1.500,00", "10/01/2026"), ("Marcos Quiroga", "2.750,00", "10/02/2026"), ("Nora Salvatierra", "980,50", "10/03/2026")]
        .iter()
        .enumerate()
        .map(|(i, (who, amount, date))| {
            let body = format!(
                "RECIBO DE HONORARIOS.\n\nRecibí de {who} la suma de pesos $ {amount} en concepto de honorarios profesionales \
                 por la consulta del día {date}, según lo convenido entre las partes en el acuerdo de servicios vigente.\n\n{}",
                words(0..30, 7)
            );
            (format!("honorarios {i}.md"), body.into_bytes())
        })
        .collect()
}

/// The base with the receipts, its work folder as an origin and the place
/// of the generated documents in it.
fn with_work_folder() -> (TempDir, DocBase, super::generate::Place, String) {
    let files = fees();
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let (dir, base, _) = base_with(&refs);
    let work = dir.folder("trabajo");
    let origin = crate::work_folder::register(&base, &work).unwrap().origin;
    crate::work_folder::prepare(&work).unwrap();
    let place = super::generate::Place::inside(&base, &work.join(crate::work_folder::GENERATED)).unwrap();
    (dir, base, place, origin.id)
}

fn given(position: usize, name: &str, value: &str, source: &str) -> super::generate::GivenValue {
    super::generate::GivenValue { position: position as i64, name: name.into(), value: value.into(), source: source.into(), reference: None }
}

/// The positions of the name, the amount and the date of the receipts' template.
fn fee_fields(template: &super::template::Template) -> (usize, usize, usize) {
    let at = |kind: &str| {
        template
            .fields
            .iter()
            .position(|f| serde_json::to_value(f.kind).unwrap() == kind)
            .unwrap_or_else(|| panic!("no {kind} field"))
    };
    (at("name"), at("amount"), at("date"))
}

/// The text of a Word, one line per paragraph.
fn word_text(path: &str) -> String {
    let docx = docx_rs::read_docx(&std::fs::read(path).unwrap()).expect("the Word opens");
    let mut out = Vec::new();
    for child in docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(paragraph) = child {
            let mut text = String::new();
            for child in &paragraph.children {
                if let docx_rs::ParagraphChild::Run(run) = child {
                    for child in &run.children {
                        if let docx_rs::RunChild::Text(t) = child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            out.push(text);
        }
    }
    out.join("\n")
}

fn fees_template(base: &DocBase) -> super::template::Template {
    let summary = base.templates(&Visible::All).unwrap().remove(0);
    base.similar_template(&Visible::All, summary.principal.location_id).unwrap().unwrap()
}

#[test]
fn a_document_generated_from_a_template_is_a_word_with_its_text_and_values_and_its_record() {
    let (_dir, base, place, work_origin) = with_work_folder();
    let template = fees_template(&base);
    let (name, amount, date) = fee_fields(&template);
    let request = super::generate::GenerateRequest {
        principal: template.principal,
        template_name: "Recibo de honorarios".into(),
        values: vec![
            given(name, "Quien paga", "Julián Ferreyra", "hand"),
            given(amount, "Importe", "3.333,33", "hand"),
            given(date, "Fecha", "22/10/2026", "date"),
        ],
        file_name: Some("Recibo de honorarios - Julián Ferreyra - 2026-10-22".into()),
        ..Default::default()
    };
    let generated = base.generate_document(&place, &request).unwrap();
    assert_eq!(generated.file_name, "Recibo de honorarios - Julián Ferreyra - 2026-10-22.docx");
    assert!(generated.missing.is_empty());

    // It opens as a Word: the template's text, with the values in place.
    let text = word_text(&generated.path);
    assert!(text.starts_with("RECIBO DE HONORARIOS.\nRecibí de Julián Ferreyra la suma de pesos $ 3.333,33"), "{text}");
    assert!(text.contains("por la consulta del día 22/10/2026, según lo convenido entre las partes"), "{text}");
    for other in ["Lucía Benítez", "Marcos Quiroga", "1.500,00", "10/01/2026"] {
        assert!(!text.contains(other), "no value of the template's documents: {other}");
    }

    // Its record: the template it came from, its version and its name then,
    // each value with where it came from, and where the file went.
    let (naming, template_name, engine, values, rel_path, from_document): (Option<i64>, String, String, String, String, Option<i64>) = base
        .conn()
        .query_row(
            "SELECT naming, template_name, template_engine, field_values, rel_path, from_document FROM generated_document WHERE id = ?1",
            [generated.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .unwrap();
    let template_naming: Option<i64> =
        base.conn().query_row("SELECT naming FROM similar_template WHERE principal = ?1", [template.principal], |row| row.get(0)).unwrap();
    assert!(naming.is_some());
    assert_eq!(naming, template_naming);
    assert_eq!(template_name, "Recibo de honorarios");
    assert_eq!(engine, super::template::ENGINE);
    assert_eq!(rel_path, "Generados/Recibo de honorarios - Julián Ferreyra - 2026-10-22.docx");
    assert_eq!(from_document, None);
    let values: Vec<super::generate::GivenValue> = serde_json::from_str(&values).unwrap();
    assert_eq!(values.len(), 3);
    let payer = values.iter().find(|v| v.position == name as i64).unwrap();
    assert_eq!((payer.name.as_str(), payer.value.as_str(), payer.source.as_str()), ("Quien paga", "Julián Ferreyra", "hand"));

    // The base has it at once, in the work folder's origin, and reads it as any document.
    let location = generated.location_id.expect("the base has the file");
    assert!(generated.content_id.is_some());
    settle(&base, &work_origin);
    let found = row(&base, &generated.file_name);
    assert_eq!(found.location_id, location);
    assert_eq!(found.folder, "Generados");
    assert_eq!(found.state, "processed");

    // Another one with the same name never writes over it.
    let again = base.generate_document(&place, &request).unwrap();
    assert_eq!(again.file_name, "Recibo de honorarios - Julián Ferreyra - 2026-10-22 (2).docx");
    assert!(std::path::Path::new(&generated.path).is_file());
}

#[test]
fn a_document_generated_with_a_field_left_empty_says_which_and_starts_from_a_document() {
    let (_dir, base, place, _) = with_work_folder();
    let template = fees_template(&base);
    let (name, amount, date) = fee_fields(&template);
    let column = 1;
    let source = template.documents[column].doc.content_id;
    let request = super::generate::GenerateRequest {
        principal: template.principal,
        template_name: "Recibo".into(),
        values: vec![
            given(name, "Quien paga", "", "hand"),
            given(amount, "Importe", &template.fields[amount].values[column], "document"),
            given(date, "Fecha", "23/10/2026", "hand"),
        ],
        from_document: Some(source),
        ..Default::default()
    };
    let generated = base.generate_document(&place, &request).unwrap();
    assert_eq!(generated.missing, ["Quien paga"]);
    assert!(generated.file_name.starts_with("Recibo 20"), "the template's name and the date: {}", generated.file_name);
    let text = word_text(&generated.path);
    assert!(text.contains("Recibí de [Quien paga] la suma de pesos"), "{text}");
    let from: Option<i64> =
        base.conn().query_row("SELECT from_document FROM generated_document WHERE id = ?1", [generated.id], |row| row.get(0)).unwrap();
    assert_eq!(from, Some(source));
    // A template that is no more cannot be generated from.
    let gone = super::generate::GenerateRequest { principal: 999_999, ..request };
    assert!(base.generate_document(&place, &gone).is_err());
}

#[test]
fn a_document_generated_for_a_matter_goes_into_it_as_a_persons_decision_and_the_template_does_not() {
    let (_dir, base, place, work_origin) = with_work_folder();
    let template = fees_template(&base);
    let (name, amount, date) = fee_fields(&template);
    let matter = base.knowledge_create_matter("Ferreyra c/ Quiroga", &[]).unwrap();
    let request = super::generate::GenerateRequest {
        principal: template.principal,
        template_name: "Recibo de honorarios".into(),
        values: vec![
            super::generate::GivenValue {
                position: name as i64,
                name: "Quien paga".into(),
                value: "Julián Ferreyra".into(),
                source: "entity".into(),
                reference: Some("entity:e-1".into()),
            },
            given(amount, "Importe", "3.333,33", "hand"),
            given(date, "Fecha", "22/10/2026", "date"),
        ],
        matter_id: Some(matter.clone()),
        ..Default::default()
    };
    let generated = base.generate_document(&place, &request).unwrap();
    let content = generated.content_id.expect("the base has its document");
    assert!(generated.in_matter, "it is in its matter at once");
    let choice: String = base
        .conn()
        .query_row(
            "SELECT choice FROM knowledge_matter_choice WHERE matter_id = ?1 AND content_id = ?2",
            rusqlite::params![matter, content],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(choice, "in", "a person's decision");
    let recorded: (Option<String>, Option<String>) = base
        .conn()
        .query_row("SELECT matter_id, matter_name FROM generated_document WHERE id = ?1", [generated.id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(recorded, (Some(matter.clone()), Some("Ferreyra c/ Quiroga".to_string())));
    let values: String =
        base.conn().query_row("SELECT field_values FROM generated_document WHERE id = ?1", [generated.id], |row| row.get(0)).unwrap();
    assert!(values.contains("\"reference\":\"entity:e-1\""), "{values}");
    // Only the generated document: none of the template's.
    let in_matter: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_matter_choice WHERE matter_id = ?1", [&matter], |row| row.get(0))
        .unwrap();
    assert_eq!(in_matter, 1);
    for document in &template.documents {
        let joined: i64 = base
            .conn()
            .query_row("SELECT count(*) FROM knowledge_matter_document WHERE content_id = ?1", [document.doc.content_id], |row| row.get(0))
            .unwrap();
        assert_eq!(joined, 0, "the template is not joined to the matter");
    }

    // A person edits the Word where it is: its new document goes into the matter.
    let edited = super::generate::word(
        &vec![vec![vec![super::generate::Piece::Text(format!(
            "RECIBO DE HONORARIOS corregido a mano. Recibí de Julián Ferreyra la suma de pesos $ 4.000,00. {}",
            words(0..30, 3)
        ))]]],
        "2026-10-23T10:00:00.000Z",
    )
    .unwrap();
    std::fs::write(&generated.path, edited).unwrap();
    settle(&base, &work_origin);
    base.generated_settle().unwrap();
    let (now, put): (Option<i64>, Option<i64>) = base
        .conn()
        .query_row("SELECT content_id, matter_content FROM generated_document WHERE id = ?1", [generated.id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert!(now.is_some() && now != Some(content), "the record follows the edited Word");
    assert_eq!(put, now);

    // Taken out of the matter by a person, it is not put back.
    base.knowledge_choose_matter_document(&matter, &format!("content:{}", now.unwrap()), "out").unwrap();
    assert_eq!(base.generated_settle().unwrap(), 0);
    let choice: String = base
        .conn()
        .query_row(
            "SELECT choice FROM knowledge_matter_choice WHERE matter_id = ?1 AND content_id = ?2",
            rusqlite::params![matter, now.unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(choice, "out");

    // A matter that is no more: nothing is generated for it.
    let gone = super::generate::GenerateRequest { matter_id: Some("mat-que-no-existe".into()), ..request };
    assert!(base.generate_document(&place, &gone).is_err());
}

#[test]
fn a_generated_document_is_known_as_such_never_the_principal_nor_possibly_the_same_and_not_of_the_template() {
    let (_dir, base, place, work_origin) = with_work_folder();
    let template = fees_template(&base);
    let (name, amount, date) = fee_fields(&template);
    let principal = template.principal;
    let matter = base.knowledge_create_matter("Benítez, honorarios", &[]).unwrap();
    // Started with the data of the principal and nothing changed: the same
    // text as a document of the template.
    let column = 0;
    let copy = super::generate::GenerateRequest {
        principal,
        template_name: "Recibo de honorarios".into(),
        values: [name, amount, date].iter().map(|&p| given(p, "Campo", &template.fields[p].values[column], "document")).collect(),
        matter_id: Some(matter.clone()),
        from_document: Some(principal),
        file_name: Some("Copia del principal".into()),
        ..Default::default()
    };
    let same = base.generate_document(&place, &copy).unwrap();
    // Another one with a long name: more words than any document of the template.
    let longer = super::generate::GenerateRequest {
        values: vec![
            given(name, "Quien paga", "Julián Ferreyra Montenegro Aldao de la Vega Arrieta y Quintana Ibáñez", "hand"),
            given(amount, "Importe", "123.456.789,00", "hand"),
            given(date, "Fecha", "22/10/2026", "date"),
        ],
        matter_id: None,
        from_document: None,
        file_name: Some("Recibo largo".into()),
        ..copy.clone()
    };
    let long = base.generate_document(&place, &longer).unwrap();
    settle(&base, &work_origin);
    base.generated_settle().unwrap();

    // The copy is a document of its own, not the principal with another file.
    let same_content = row(&base, "Copia del principal.docx").content_id.unwrap();
    let long_content = row(&base, "Recibo largo.docx").content_id.unwrap();
    assert_ne!(same_content, principal, "never the document whose data it started with");
    assert_eq!(Some(same_content), base.conn().query_row("SELECT content_id FROM generated_document WHERE id = ?1", [same.id], |r| r.get(0)).unwrap());
    let principal_files: i64 =
        base.conn().query_row("SELECT count(*) FROM file WHERE content_id = ?1", [principal], |row| row.get(0)).unwrap();
    assert_eq!(principal_files, 1);
    assert_eq!(row(&base, "Copia del principal.docx").state, "processed");

    // Never the principal of the family, nor a document of the template.
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert!(groups.iter().all(|g| g.principal.content_id != same_content && g.principal.content_id != long_content));
    let after = fees_template(&base);
    assert_eq!(after.principal, principal);
    let documents: Vec<i64> = after.documents.iter().map(|d| d.doc.content_id).collect();
    assert_eq!(documents.len(), 3);
    assert!(!documents.contains(&same_content) && !documents.contains(&long_content));
    assert!(after.possibly_same.is_empty());
    for field in &after.fields {
        assert!(!field.values.iter().any(|v| v.contains("Ferreyra")), "the template keeps no value of what was generated");
    }

    // With the documents of the template it is the same template, never possibly the same document.
    for generated in [same_content, long_content] {
        let (a, b) = (generated.min(principal), generated.max(principal));
        let relation: Option<String> = base
            .conn()
            .query_row("SELECT relation FROM similar_pair WHERE a = ?1 AND b = ?2", rusqlite::params![a, b], |row| row.get(0))
            .optional()
            .unwrap();
        if let Some(relation) = relation {
            assert_eq!(relation, "same-template");
        }
    }
    let near: Option<String> = base
        .conn()
        .query_row(
            "SELECT relation FROM similar_pair WHERE a = ?1 AND b = ?2",
            rusqlite::params![same_content.min(principal), same_content.max(principal)],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(near.as_deref(), Some("same-template"), "the copy pairs with the principal, as the same template");
    let pending = base.similar_pending(&Visible::All).unwrap();
    assert_eq!(pending.same_document_pairs, 0);

    // Its screen says it was generated, from which template and for which matter.
    let detail = base.document(&Visible::All, row(&base, "Copia del principal.docx").location_id).unwrap().unwrap();
    let line = detail.generated.expect("it says it was generated");
    assert_eq!(line.id, same.id);
    assert_eq!(line.template_name, "Recibo de honorarios");
    assert_eq!(line.matter_name.as_deref(), Some("Benítez, honorarios"));
    assert_eq!(line.template_location, Some(after.documents[0].doc.location_id));
    let source = base.document(&Visible::All, after.documents[0].doc.location_id).unwrap().unwrap();
    assert!(source.generated.is_none(), "the documents of the template were not generated");

    // The template lists what was generated with it, the latest first.
    assert_eq!(after.generated.len(), 2);
    assert_eq!(after.generated[0].id, long.id);
    assert_eq!(after.generated[0].file_name, "Recibo largo.docx");
    assert_eq!(after.generated[1].matter_name.as_deref(), Some("Benítez, honorarios"));
    assert!(after.generated.iter().all(|g| g.location_id.is_some()));
    let summary = base.templates(&Visible::All).unwrap().remove(0);
    assert_eq!((summary.documents, summary.generated), (3, 2));
}

// ── The key data of two documents (D-497) ──────────────────────────────

/// What the reading said of a document: that it names `who` with `role`,
/// as the knowledge layer keeps it.
fn names_in(base: &DocBase, name: &str, who: &str, role: &str) {
    let id = content(base, name);
    base.conn()
        .execute(
            "INSERT INTO knowledge_mention (content_id, ref, kind, written, normalized, role, quote)
             VALUES (?1, ?2, 'organizacion', ?2, ?3, ?4, ?2)",
            rusqlite::params![id, who, super::text::normalized(who), role],
        )
        .unwrap();
}

/// The contract with other parties: `first` instead of the first one.
fn contract_for(text: &str, first: &str) -> String {
    text.replace("Ejemplo Uno S.A.", first)
}

#[test]
fn two_contracts_of_one_template_with_other_parties_and_amounts_wait_for_no_one() {
    let price = |amount: &str| {
        CONTRACT.replace("abonará el precio pactado", &format!("abonará el precio pactado de USD {amount} mensuales"))
    };
    let (_dir, base, _) = base_with(&[
        ("contrato alfa.md", contract_for(&price("2.000.000"), "Alfa Servicios S.A.").into_bytes()),
        ("contrato beta.md", contract_for(&price("20"), "Beta Insumos S.A.").into_bytes()),
    ]);
    names_in(&base, "contrato alfa.md", "Alfa Servicios S.A.", "parte");
    names_in(&base, "contrato beta.md", "Beta Insumos S.A.", "parte");
    while base.similar_step(10).unwrap() > 0 {}
    // Two contracts, not two of the same: made with one template.
    assert_eq!(relation(&base, "contrato alfa.md", "contrato beta.md").as_deref(), Some("same-template"));
    let pending = base.similar_pending(&Visible::All).unwrap();
    assert_eq!((pending.pairs, pending.groups, pending.same_document_pairs), (0, 0, 0));
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!((groups.len(), groups[0].members.len(), groups[0].pending), (1, 2, 0));
    let template = base.similar_template(&Visible::All, row(&base, "contrato beta.md").location_id).unwrap();
    assert!(template.is_some_and(|t| t.documents.len() == 2), "both are documents of the template");
}

#[test]
fn a_reworded_clause_is_a_version_until_the_parties_say_they_are_two_contracts() {
    let (_dir, base, _) = base_with(&[
        ("contrato alfa.md", contract_for(CONTRACT, "Alfa Servicios S.A.").into_bytes()),
        ("contrato beta.md", contract_for(&contract_final(), "Beta Insumos S.A.").into_bytes()),
    ]);
    // Read alone, the texts say a clause was reworded: one contract, a
    // person decides.
    assert_eq!(relation(&base, "contrato alfa.md", "contrato beta.md").as_deref(), Some("version"));
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 1);

    // The reading names another party in each: two contracts of one
    // template, measured again by themselves.
    names_in(&base, "contrato alfa.md", "Alfa Servicios S.A.", "parte");
    names_in(&base, "contrato beta.md", "Beta Insumos S.A.", "parte");
    while base.similar_step(10).unwrap() > 0 {}
    let similar = pair(&base, "contrato alfa.md", "contrato beta.md").unwrap();
    assert_eq!(similar.relation, "same-template");
    assert_eq!(similar.description, "Mismo texto de base, pero son documentos distintos: cambian las partes.");
    assert_eq!(similar.evidence["keyData"]["differ"], serde_json::json!(["parties"]));
    assert_eq!(base.similar_pending(&Visible::All).unwrap().pairs, 0);
    let filter = DocumentFilter { similar_pending: true, ..Default::default() };
    assert!(base.documents(&Visible::All, &filter).unwrap().rows.is_empty(), "nothing waits in Documentos either");
}

#[test]
fn a_version_and_a_signed_copy_with_the_same_parties_still_wait_for_a_person() {
    let signed = format!("## Page 1\n\n{}\n\n## Page 2\n\n{AUDIT_PAGE}\n", contract_final());
    let (_dir, base, _) = base_with(&[
        ("borrador.md", CONTRACT.as_bytes().to_vec()),
        ("final.md", contract_final().into_bytes()),
        ("final firmado.md", signed.into_bytes()),
    ]);
    for name in ["borrador.md", "final.md", "final firmado.md"] {
        names_in(&base, name, "Ejemplo Uno S.A.", "parte");
        names_in(&base, name, "Ejemplo Dos S.R.L.", "parte");
    }
    // A party only the signed copy names (its signatory) is no other party.
    names_in(&base, "final firmado.md", "Firmante Inventado", "firmante");
    while base.similar_step(10).unwrap() > 0 {}
    assert_eq!(relation(&base, "borrador.md", "final.md").as_deref(), Some("version"));
    assert_eq!(relation(&base, "final.md", "final firmado.md").as_deref(), Some("signed-copy"));
    let pending = base.similar_pending(&Visible::All).unwrap();
    assert_eq!((pending.pairs, pending.same_document_pairs), (3, 3), "the draft, the final and the signed copy");
}

#[test]
fn two_documents_that_share_some_text_and_name_other_parties_are_no_pair() {
    let with = |party: &str, salt: usize| format!("{CONTRACT} Firma en representación de {party}. {}", words(0..45, salt));
    let (_dir, base, _) = base_with(&[
        ("nota alfa.md", with("Alfa Servicios S.A.", 1).into_bytes()),
        ("nota beta.md", with("Beta Insumos S.A.", 2).into_bytes()),
    ]);
    assert_eq!(relation(&base, "nota alfa.md", "nota beta.md").as_deref(), Some("similar"));
    names_in(&base, "nota alfa.md", "Alfa Servicios S.A.", "parte");
    names_in(&base, "nota beta.md", "Beta Insumos S.A.", "parte");
    while base.similar_step(10).unwrap() > 0 {}
    assert_eq!(relation(&base, "nota alfa.md", "nota beta.md"), None, "different documents, not similar ones");
    // When the reading no longer says so, they are measured again.
    base.conn().execute("DELETE FROM knowledge_mention", []).unwrap();
    while base.similar_step(10).unwrap() > 0 {}
    assert_eq!(relation(&base, "nota alfa.md", "nota beta.md").as_deref(), Some("similar"));
}
