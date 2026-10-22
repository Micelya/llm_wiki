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
    // Two different documents: it waits for a person, but it is not
    // escalated as something that changes the wiki.
    let pending = base.similar_pending(&Visible::All).unwrap();
    assert_eq!((pending.pairs, pending.same_document_pairs), (1, 0));
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!((groups[0].pending, groups[0].pending_same_document), (1, 0));
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
fn a_family_of_the_same_template_is_confirmed_at_once_and_the_rest_stays_open() {
    let may = invoice("0003-00001377", "15/05/2026", "01/04/2026", "30/04/2026", "98.000,00", "20.580,00", "118.580,00", "25/05/2026");
    let (_dir, base, _) = base_with(&[
        ("marzo.md", march().into_bytes()),
        ("abril.md", april().into_bytes()),
        ("mayo.md", may.into_bytes()),
    ]);
    let groups = base.similar_groups(&Visible::All).unwrap();
    assert_eq!(groups.len(), 1, "the three invoices are one family");
    let group = &groups[0];
    assert_eq!((group.members.len(), group.pending, group.pending_same_document), (3, 2, 0));

    // The pair between the two members is not the principal's: it stays open.
    let open = base.similar_open_pairs(&Visible::All).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].similar.relation, "same-template");
    assert_ne!(open[0].doc.content_id, group.principal.content_id);
    assert_ne!(open[0].similar.other.content_id, group.principal.content_id);

    // Confirming the family joins every member with its principal, in one go.
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
    assert_eq!(after[0].pending, 0);
    assert_eq!(base.similar_open_pairs(&Visible::All).unwrap().len(), 1);

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
    // Once a person decides about the two, it is no longer shown.
    base.decide_similar(uno, tres, &Decision::Discard, None).unwrap();
    let template = base.similar_template(&Visible::All, groups[0].principal.location_id).unwrap().unwrap();
    assert!(template.possibly_same.is_empty());
}
