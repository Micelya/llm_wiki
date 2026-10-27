//! Measures the relations between entities of a copy of a real base
//! against the answer sheets (R-17): the relations a document says, found
//! among its evidence and its claims, and the ones it does not say (that a
//! hasty reader could make up), not found or left in doubt. Before and
//! after checking the readings again from the answers kept and resolving
//! again: no question is asked to any model, and the run fails if one is.
//!
//! ```text
//! MICELYA_DOCBASE_COPY="C:\tmp\private.sqlite3" MICELYA_ENTITY_ANSWERS="C:\hoja\a.json;C:\hoja\b.json" \
//!     cargo test --lib docbase::knowledge::check_relations -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The base and the sheets are only read; what is written goes to a copy
//! in a scratch folder removed at the end (`MICELYA_KNOWLEDGE_SAVE` keeps
//! one). Nothing of a sheet is in the program (R-24): it names real people.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::check::{names_match, normalized, open_base, save_copy, sheets, Sheet, SheetEntity};
use super::extract::RereadTarget;
use crate::docbase::similar::check_types::{key, paths_by_content};
use crate::docbase::{DocBase, Visible};

/// How a relation of a sheet is said in the vocabulary.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sides {
    /// The same subject and object.
    Same,
    /// The other way round ("hijo de" is kept as "padre o madre de").
    Swapped,
    /// Either way (a spouse, a sibling).
    Either,
}

/// The predicates of the vocabulary that keep a relation of the sheet, and
/// how its sides are; `None`: the vocabulary has none.
fn relation_predicates(sheet: &str) -> Option<(&'static [&'static str], Sides)> {
    Some(match sheet {
        "padre_de" | "madre_de" | "progenitor_de" => (&["progenitor_de"], Sides::Same),
        "hijo_de" => (&["progenitor_de"], Sides::Swapped),
        "abuelo_de" => (&["abuelo_de"], Sides::Same),
        "nieto_de" => (&["abuelo_de"], Sides::Swapped),
        "tio_de" => (&["tio_de"], Sides::Same),
        "sobrino_de" => (&["tio_de"], Sides::Swapped),
        "hermano_de" => (&["hermano_de"], Sides::Either),
        "conyuge_de" => (&["conyuge_de"], Sides::Either),
        "conviviente_de" => (&["conviviente_de"], Sides::Either),
        "heredero_de" => (&["heredero_de", "heredero_por_representacion_de"], Sides::Same),
        "heredero_por_representacion_de" => (&["heredero_por_representacion_de"], Sides::Same),
        "causante" => (&["causante"], Sides::Same),
        "apoderado_de" | "representa_a" => (&["representa_a"], Sides::Same),
        "parte_en" => (&["parte_en"], Sides::Same),
        "socio_de" => (&["socio_de"], Sides::Same),
        "beneficiario_final_de" => (&["beneficiario_final_de"], Sides::Same),
        "deudor_de" => (&["deudor_de", "importe"], Sides::Same),
        "pago_a" => (&["importe"], Sides::Same),
        "contratado_por" | "empleado_de" => (&["empleado_de"], Sides::Same),
        _ => return None,
    })
}

#[derive(Debug, Clone)]
struct Mention {
    id: i64,
    kind: String,
    normalized: String,
    entity: Option<String>,
}

#[derive(Debug, Clone)]
struct Piece {
    subject: i64,
    predicate: String,
    object: Option<i64>,
    /// The level of certainty of its claim; `None`: it is not a claim.
    certainty: Option<String>,
}

/// What the record has, read once.
struct Record {
    mentions: BTreeMap<i64, Vec<Mention>>,
    pieces: BTreeMap<i64, Vec<Piece>>,
    by_path: HashMap<String, i64>,
    /// The claims between two entities: (subject, predicate, object) and
    /// the level of their certainty.
    claims: HashMap<(String, String, String), String>,
}

fn record(base: &DocBase) -> Record {
    let conn = base.conn();
    let mut mentions: BTreeMap<i64, Vec<Mention>> = BTreeMap::new();
    {
        let mut statement = conn.prepare("SELECT version_id, id, kind, normalized, entity_id FROM knowledge_mention").unwrap();
        let rows = statement
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, Mention { id: r.get(1)?, kind: r.get(2)?, normalized: r.get(3)?, entity: r.get(4)? })))
            .unwrap();
        for row in rows {
            let (content, mention) = row.unwrap();
            mentions.entry(content).or_default().push(mention);
        }
    }
    let mut pieces: BTreeMap<i64, Vec<Piece>> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT e.version_id, e.subject_mention, e.predicate, e.object_mention, c.certainty, c.state
                 FROM knowledge_evidence e LEFT JOIN knowledge_claim c ON c.id = e.claim_id",
            )
            .unwrap();
        let rows = statement
            .query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<i64>>(3)?, r.get::<_, Option<String>>(4)?, r.get::<_, Option<String>>(5)?))
            })
            .unwrap();
        for row in rows {
            let (content, subject, predicate, object, certainty, state) = row.unwrap();
            let certainty = match state.as_deref() {
                None | Some("retired") => None,
                _ => Some(
                    certainty
                        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                        .and_then(|c| c["level"].as_str().map(str::to_string))
                        .unwrap_or_else(|| "quoted".to_string()),
                ),
            };
            pieces.entry(content).or_default().push(Piece { subject, predicate, object, certainty });
        }
    }
    let mut claims = HashMap::new();
    {
        let mut statement = conn
            .prepare("SELECT subject, predicate, object_entity, certainty FROM knowledge_claim WHERE object_entity IS NOT NULL AND state != 'retired'")
            .unwrap();
        let rows = statement
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?)))
            .unwrap();
        for row in rows {
            let (subject, predicate, object, certainty) = row.unwrap();
            let level = certainty
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .and_then(|c| c["level"].as_str().map(str::to_string))
                .unwrap_or_else(|| "quoted".to_string());
            claims.insert((subject, predicate, object), level);
        }
    }
    drop(conn);
    let (by_path, _) = paths_by_content(base);
    Record { mentions, pieces, by_path, claims }
}

/// The entity of the record most of the mentions of an entity of the sheet
/// went to.
fn main_entity(record: &Record, sheet: &Sheet, entity: &SheetEntity) -> Option<String> {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for mention in &entity.mentions {
        let Some(content) = record.by_path.get(&key(&format!("{}/{}", sheet.root, mention.path))).copied() else { continue };
        let written = normalized(&mention.as_written);
        for found in record.mentions.get(&content).into_iter().flatten() {
            if found.kind == entity.kind && (found.normalized == written || names_match(&written, &found.normalized)) {
                if let Some(e) = &found.entity {
                    *count.entry(e.clone()).or_default() += 1;
                }
            }
        }
    }
    count.into_iter().max_by_key(|(_, n)| *n).map(|(e, _)| e)
}

/// How a relation that is not stands anywhere in the record: a claim of it
/// between the entities its two sides went to, whatever the document.
fn found_anywhere(record: &Record, sheet: &Sheet, relation: &super::check::SheetRelation) -> Option<Found> {
    let (predicates, sides) = relation_predicates(&relation.predicate)?;
    let entity = |id: &str| sheet.entities.iter().find(|e| e.id == id);
    let (Some(subject), Some(object)) = (entity(&relation.subject), entity(&relation.object)) else { return None };
    let (Some(s), Some(o)) = (main_entity(record, sheet, subject), main_entity(record, sheet, object)) else { return Some(Found::Absent) };
    let mut best = Found::Absent;
    for predicate in predicates {
        let mut levels = Vec::new();
        if matches!(sides, Sides::Same | Sides::Either) {
            levels.extend(record.claims.get(&(s.clone(), predicate.to_string(), o.clone())));
        }
        if matches!(sides, Sides::Swapped | Sides::Either) {
            levels.extend(record.claims.get(&(o.clone(), predicate.to_string(), s.clone())));
        }
        for level in levels {
            best = best.max(if level == "doubtful" { Found::Doubtful } else { Found::Firm });
        }
    }
    Some(best)
}

/// The names an entity of the sheet is written with in a document (and its
/// own name and aliases).
fn names_in(entity: &SheetEntity, path: &str) -> Vec<String> {
    let mut names: Vec<String> = vec![normalized(&entity.name)];
    names.extend(entity.aliases.iter().map(|a| normalized(a)));
    names.extend(entity.mentions.iter().filter(|m| m.path == path).map(|m| normalized(&m.as_written)));
    names.retain(|n| !n.is_empty());
    names
}

/// The mentions of a document that are an entity of the sheet.
fn mentions_of(record: &Record, content: i64, entity: &SheetEntity, path: &str) -> Vec<i64> {
    let names = names_in(entity, path);
    record
        .mentions
        .get(&content)
        .into_iter()
        .flatten()
        .filter(|m| m.kind == entity.kind && names.iter().any(|n| names_match(n, &m.normalized)))
        .map(|m| m.id)
        .collect()
}

/// How a relation of the sheet stands in the record: the best of its
/// pieces of evidence in its document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Found {
    /// No piece of evidence says it.
    Absent,
    /// Evidence that is no claim (its quote does not say it, or a person
    /// marked it wrong).
    NotAClaim,
    /// A claim left in doubt.
    Doubtful,
    /// A claim with its quote checked.
    Firm,
}

fn found(record: &Record, sheet: &Sheet, relation: &super::check::SheetRelation) -> Option<Found> {
    let (predicates, sides) = relation_predicates(&relation.predicate)?;
    let entity = |id: &str| sheet.entities.iter().find(|e| e.id == id);
    let (Some(subject), Some(object)) = (entity(&relation.subject), entity(&relation.object)) else { return None };
    let content = *record.by_path.get(&key(&format!("{}/{}", sheet.root, relation.path)))?;
    let subjects = mentions_of(record, content, subject, &relation.path);
    let objects = mentions_of(record, content, object, &relation.path);
    let mut best = Found::Absent;
    for piece in record.pieces.get(&content).into_iter().flatten() {
        if !predicates.contains(&piece.predicate.as_str()) {
            continue;
        }
        let Some(other) = piece.object else { continue };
        let same = subjects.contains(&piece.subject) && objects.contains(&other);
        let swapped = objects.contains(&piece.subject) && subjects.contains(&other);
        let fits = match sides {
            Sides::Same => same,
            Sides::Swapped => swapped,
            Sides::Either => same || swapped,
        };
        if !fits {
            continue;
        }
        let now = match piece.certainty.as_deref() {
            None => Found::NotAClaim,
            Some("doubtful") => Found::Doubtful,
            Some(_) => Found::Firm,
        };
        best = best.max(now);
    }
    Some(best)
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Tally {
    firm: usize,
    doubtful: usize,
    not_a_claim: usize,
    absent: usize,
}

impl Tally {
    fn add(&mut self, found: Found) {
        match found {
            Found::Firm => self.firm += 1,
            Found::Doubtful => self.doubtful += 1,
            Found::NotAClaim => self.not_a_claim += 1,
            Found::Absent => self.absent += 1,
        }
    }

    fn total(&self) -> usize {
        self.firm + self.doubtful + self.not_a_claim + self.absent
    }

    fn line(&self) -> String {
        format!(
            "{} measured: {} firm, {} doubtful, {} kept as evidence only, {} absent",
            self.total(),
            self.firm,
            self.doubtful,
            self.not_a_claim,
            self.absent
        )
    }
}

/// The measurement of one sheet: the relations it says, the ones it says
/// are not, by predicate, and the entities whose mentions went to several
/// entities of the record.
struct Measure {
    relations: Tally,
    not_relations: Tally,
    /// The relations that are not, as claims between their entities, in
    /// any document.
    not_anywhere: Tally,
    wrong_anywhere: Vec<(String, String, String)>,
    by_predicate: BTreeMap<String, (Tally, Tally)>,
    unmeasured: BTreeMap<String, usize>,
    /// The relations that are not, still firm: (document, predicate, subject, object).
    wrong: Vec<(i64, String, String, String)>,
    /// Entities of the sheet (people and organizations) and how many of
    /// them are split into several entities of the record.
    entities: usize,
    split: Vec<(String, usize)>,
    /// The same, with the best match of each mention of the sheet only.
    split_best: Vec<(String, usize)>,
}

fn measure(record: &Record, sheet: &Sheet) -> Measure {
    let mut out = Measure {
        relations: Tally::default(),
        not_relations: Tally::default(),
        not_anywhere: Tally::default(),
        wrong_anywhere: Vec::new(),
        by_predicate: BTreeMap::new(),
        unmeasured: BTreeMap::new(),
        wrong: Vec::new(),
        entities: 0,
        split: Vec::new(),
        split_best: Vec::new(),
    };
    for relation in &sheet.relations {
        match found(record, sheet, relation) {
            Some(f) => {
                out.relations.add(f);
                out.by_predicate.entry(relation.predicate.clone()).or_default().0.add(f);
            }
            None => *out.unmeasured.entry(relation.predicate.clone()).or_default() += 1,
        }
    }
    for relation in &sheet.not_relations {
        match found(record, sheet, relation) {
            Some(f) => {
                out.not_relations.add(f);
                out.by_predicate.entry(relation.predicate.clone()).or_default().1.add(f);
                if f == Found::Firm {
                    let content = record.by_path.get(&key(&format!("{}/{}", sheet.root, relation.path))).copied().unwrap_or_default();
                    out.wrong.push((content, relation.predicate.clone(), relation.subject.clone(), relation.object.clone()));
                }
            }
            None => *out.unmeasured.entry(format!("{} (not)", relation.predicate)).or_default() += 1,
        }
    }
    // The same, as claims between the entities, from any document (what a
    // page shows); each pair once.
    let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
    for relation in &sheet.not_relations {
        if !seen.insert((relation.predicate.clone(), relation.subject.clone(), relation.object.clone())) {
            continue;
        }
        if let Some(f) = found_anywhere(record, sheet, relation) {
            out.not_anywhere.add(f);
            if f == Found::Firm {
                out.wrong_anywhere.push((relation.predicate.clone(), relation.subject.clone(), relation.object.clone()));
            }
        }
    }
    // The entities of the record each entity of the sheet went to: every
    // mention whose name matches (`split`), and the best match of each
    // mention of the sheet (`split_best`).
    for entity in sheet.entities.iter().filter(|e| e.kind != "asunto") {
        let mut held: BTreeSet<String> = BTreeSet::new();
        let mut best: BTreeSet<String> = BTreeSet::new();
        for mention in &entity.mentions {
            let Some(content) = record.by_path.get(&key(&format!("{}/{}", sheet.root, mention.path))).copied() else { continue };
            let written = normalized(&mention.as_written);
            let found: Vec<&Mention> =
                record.mentions.get(&content).into_iter().flatten().filter(|f| f.kind == entity.kind && names_match(&written, &f.normalized)).collect();
            held.extend(found.iter().filter_map(|f| f.entity.clone()));
            best.extend(best_match(sheet, entity, &mention.path, &written, &found));
        }
        if held.is_empty() {
            continue;
        }
        out.entities += 1;
        if held.len() > 1 {
            out.split.push((entity.id.clone(), held.len()));
        }
        if best.len() > 1 {
            out.split_best.push((entity.id.clone(), best.len()));
        }
    }
    out
}

/// The entities of the record a mention of the sheet is, among the mentions
/// of its document whose name matches: the ones written the same, if any;
/// else those whose words the entity's names have more of than any other
/// entity of the sheet has ("Persona Dos Ejemplo" is not "Persona Uno Dos Ejemplo"
/// when the sheet has both; a short name of the sheet, "Registro de la
/// Propiedad", does not take in another registry of the same document).
fn best_match(sheet: &Sheet, entity: &SheetEntity, path: &str, written: &str, found: &[&Mention]) -> BTreeSet<String> {
    let exact: BTreeSet<String> = found.iter().filter(|f| f.normalized == written).filter_map(|f| f.entity.clone()).collect();
    if !exact.is_empty() {
        return exact;
    }
    // How many of the words of a name of the record the names of an entity
    // of the sheet have.
    let covered = |e: &SheetEntity, name: &str| -> usize {
        let names = names_in(e, path);
        if !names.iter().any(|n| names_match(n, name)) {
            return 0;
        }
        let words: BTreeSet<&str> = names.iter().flat_map(|n| n.split(' ')).collect();
        name.split(' ').filter(|w| words.contains(w)).count()
    };
    found
        .iter()
        .filter(|f| {
            let own = covered(entity, &f.normalized);
            own > 0 && sheet.entities.iter().filter(|o| o.id != entity.id && o.kind == entity.kind).all(|o| covered(o, &f.normalized) < own)
        })
        .filter_map(|f| f.entity.clone())
        .collect()
}

fn print(label: &str, sheet: &Sheet, m: &Measure, noisy: usize) {
    println!("\n== {label}: sheet of {} ({} entities) ==", sheet.root, sheet.entities.len());
    println!("   relations of the sheet: {}", m.relations.line());
    println!("   relations that are not (should be absent or doubtful): {}", m.not_relations.line());
    for (predicate, (yes, no)) in &m.by_predicate {
        println!(
            "      {predicate}: says {}/{} firm ({} doubtful); is not: {} firm of {} ({} doubtful)",
            yes.firm,
            yes.total(),
            yes.doubtful,
            no.firm,
            no.total(),
            no.doubtful
        );
    }
    println!(
        "   not measured (no predicate of the vocabulary): {}",
        m.unmeasured.iter().map(|(p, n)| format!("{p} ({n})")).collect::<Vec<_>>().join(", ")
    );
    println!("   relations that are not, as claims between their entities from any document: {}", m.not_anywhere.line());
    println!("      still firm: {:?}", m.wrong_anywhere);
    println!("   relations that are not, still firm in their document: {}", m.wrong.len());
    for (content, predicate, subject, object) in &m.wrong {
        println!("      #{content} {predicate} {subject} -> {object}");
    }
    println!(
        "   entities of the sheet found: {}; split into several entities of the record: {} {:?}",
        m.entities,
        m.split.len(),
        m.split
    );
    println!("   split, by the best match of each mention of the sheet: {} {:?}", m.split_best.len(), m.split_best);
    println!("   readings that lost many quotes (attention): {noisy}");
}

/// The folder of the origin a sheet's documents are in: the first folder of
/// the paths of its mentions, when they all share one.
fn sheet_folder(sheet: &Sheet) -> Option<String> {
    let mut folders = sheet
        .entities
        .iter()
        .flat_map(|e| e.mentions.iter())
        .map(|m| m.path.replace('\\', "/").split_once('/').map(|(first, _)| first.to_string()));
    let first = folders.next()??;
    folders.all(|f| f.as_deref() == Some(first.as_str())).then_some(first)
}

fn answers(base: &DocBase) -> i64 {
    base.conn().query_row("SELECT count(*) FROM model_answer", [], |r| r.get(0)).unwrap()
}

#[test]
#[ignore = "reads a copy of a real base and the answer sheets"]
fn relations_against_the_sheets() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
        return;
    };
    let sheets = sheets();
    if sheets.is_empty() {
        println!("MICELYA_ENTITY_ANSWERS is not set or not readable: nothing to measure");
        return;
    }
    let noisy = |base: &DocBase| base.knowledge_attention(&Visible::All).unwrap().noisy.len();
    let before = record(&base);
    let noisy_before = noisy(&base);
    for sheet in &sheets {
        print("before (the copy as it is)", sheet, &measure(&before, sheet), noisy_before);
    }
    let status = base.knowledge_status(&Visible::All).unwrap();
    println!("\n   readings: {} done, {} of an earlier version, {} to ask again", status.extracted, status.older, status.stale);
    // What reading the folder of each sheet again would take, if a person
    // asked (D-432, D-459): nothing is asked here.
    for sheet in &sheets {
        let origin: Option<String> =
            base.conn().query_row("SELECT id FROM origin WHERE location = ?1", [&sheet.root], |row| row.get(0)).ok();
        let (Some(origin), Some(folder)) = (origin, sheet_folder(sheet)) else { continue };
        let target = RereadTarget { location_id: None, origin_id: Some(origin), folder: Some(folder.clone()), ..Default::default() };
        let plan = base.knowledge_reread_plan(&target).unwrap();
        println!(
            "   reading again the folder «{folder}» of a sheet, if a person asks: {} questions for {} documents ({} asked already)",
            plan.questions, plan.documents, plan.requested
        );
    }

    // The check of now over the answers kept, and the identities again
    // with the verdicts kept: nothing is asked.
    let kept = answers(&base);
    let started = std::time::Instant::now();
    let checked = base.knowledge_recheck(usize::MAX).unwrap();
    let report = base.knowledge_resolve(None, 0).unwrap();
    assert_eq!(answers(&base), kept, "no question to the model");
    assert_eq!(report.asked, 0);
    let plan = base.knowledge_plan().unwrap();
    println!(
        "\n   checked again from the answers kept: {checked} readings in {:.1}s; resolved: {} entities, {} claims, {} doubts; documents the automatic reading would ask about now: {}",
        started.elapsed().as_secs_f32(),
        report.entities,
        report.claims,
        report.doubts,
        plan.documents
    );
    let status = base.knowledge_status(&Visible::All).unwrap();
    println!(
        "   readings now: {} done, {} of an earlier version, {} to ask again; questions about identities left for the model: {}",
        status.extracted, status.older, status.stale, report.left
    );
    // What the pages of what changed would ask the model (their syntheses),
    // answers kept aside: the normal cost of data that changed.
    let engine = crate::docbase::similar::classify::ask_engine(&base.conn()).unwrap().map(|c| crate::source_volume::asking::engine_id(&c));
    if let (Some(engine), true) = (&engine, report.left > 0) {
        // The questions about identities the app would ask (each with its
        // principal, cut short); until they are answered the pages wait
        // for the resolution. On this copy the wait is lifted to count what
        // the pages would ask with the record as it is now.
        let questions = base.identity_questions(engine).unwrap();
        println!("   questions about identities the resolution would ask now: {}", questions.len());
        for question in &questions {
            let principal = question.prompt.split("Principal:\n").nth(1).unwrap_or_default().lines().next().unwrap_or_default();
            println!("      principal: {}", principal.chars().take(120).collect::<String>());
        }
        base.conn().execute("DELETE FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')", []).unwrap();
    }
    if let Some(engine) = engine {
        let questions = base.synthesis_questions(&engine).unwrap();
        let conn = base.conn();
        let new = questions
            .iter()
            .filter(|q| crate::docbase::similar::classify::cached_answer(&conn, &q.key, Some(&engine)).unwrap().is_none())
            .count();
        println!("   syntheses of pages the projection would ask now: {} ({} without an answer kept)", questions.len(), new);
    }
    let after = record(&base);
    let noisy_after = noisy(&base);
    for sheet in &sheets {
        print("after", sheet, &measure(&after, sheet), noisy_after);
    }
    save_copy(&base);
}
