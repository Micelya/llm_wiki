//! The page of each document (D-340): one page per document, and one page
//! for the versions of one same document (a draft, its signed copy, a
//! preview), written by the app from the base and nothing else.
//!
//! Which documents are versions of one is what the base already knows:
//! the relations between documents of `pages.versionRelations`, proposed
//! by the computation of similar documents or confirmed by a person, never
//! the discarded ones. Documents of one template are pages apart, linked.
//! On the page every point of the summary, every datum and every amount
//! says which version it comes from when it is not in all of them, so two
//! versions that differ show both (D-293).

use super::*;

/// A document of the archive's wiki, or the versions of one document.
#[derive(Debug, Clone)]
pub(super) struct Family {
    /// `doc-<smallest content id>`: the page keeps it while the family
    /// keeps that version.
    pub id: String,
    /// Its versions, the one the page is named after first.
    pub members: Vec<i64>,
    /// The relations between its versions.
    pub ties: Vec<Tie>,
}

/// A relation between two documents, as the base keeps it.
#[derive(Debug, Clone)]
pub(super) struct Tie {
    pub a: i64,
    pub b: i64,
    pub relation: String,
    pub subject: Option<i64>,
    pub certainty: String,
    pub joined: bool,
}

#[derive(Debug, Default)]
pub(super) struct Families {
    pub list: Vec<Family>,
    /// The family of each document.
    pub of: HashMap<i64, usize>,
    /// Relations with documents of other pages.
    pub across: Vec<Tie>,
    /// What the wiki calls each file of each document (`<origin>/<path>`).
    pub identities: BTreeMap<i64, Vec<String>>,
    /// The readable title of each family's page (`titles`), once the
    /// record is loaded.
    pub titles: BTreeMap<String, String>,
}

impl Families {
    pub fn family_of(&self, content: i64) -> Option<&Family> {
        self.of.get(&content).map(|&i| &self.list[i])
    }

    /// The title of a family's page: its readable title, or the name of
    /// the file of its principal version.
    pub fn title(&self, conn: &Connection, record: &Record, family: &Family) -> Result<String, String> {
        match self.titles.get(&family.id) {
            Some(title) => Ok(title.clone()),
            None => file_title(conn, record, family.members[0]),
        }
    }

    /// What the pages of documents depend on besides the record: which
    /// documents and files there are and how they are related.
    pub fn digest(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for family in &self.list {
            let ties: Vec<String> = family.ties.iter().map(|t| format!("{}-{}-{}-{:?}-{}-{}", t.a, t.b, t.relation, t.subject, t.certainty, t.joined)).collect();
            parts.push(format!("{}:{:?}:{}", family.id, family.members, ties.join(",")));
        }
        for t in &self.across {
            parts.push(format!("x{}-{}-{}-{:?}-{}-{}", t.a, t.b, t.relation, t.subject, t.certainty, t.joined));
        }
        for (content, names) in &self.identities {
            parts.push(format!("{content}={}", names.join("|")));
        }
        parts.join("\n")
    }
}

/// The documents of the base's origins (what is left out of the base is
/// not), with the name of each of their files (`<origin>/<path inside
/// it>`), the joined copies a person passed over included: they are
/// versions too. Every origin counts, mounted in a wiki or not (D-402).
fn identities(base: &DocBase) -> Result<BTreeMap<i64, Vec<String>>, String> {
    let mut out: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    let conn = base.conn();
    for origin in crate::docbase::origins::list(&conn, &Visible::All)? {
        let mount = origin.name.as_str();
        let mut statement = conn
            .prepare(&format!(
                "SELECT l.rel_path, f.content_id FROM location l JOIN file f ON f.hash = l.file_hash
                 WHERE l.origin_id = ?1 AND l.kind = 'file' AND f.content_id IS NOT NULL AND NOT {} ORDER BY l.rel_path",
                crate::docbase::excluded_sql!()
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([&origin.id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (rel_path, content) = row.map_err(sql_err)?;
            out.entry(content).or_default().push(format!("{mount}/{rel_path}"));
        }
    }
    Ok(out)
}

/// The families of documents: each document with its versions.
pub(super) fn families(base: &DocBase, vocabulary: &Vocabulary) -> Result<Families, String> {
    let identities = identities(base)?;
    let words = &vocabulary.pages;
    let ties: Vec<(Tie, Option<i64>)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT a, b, coalesce(chosen_relation, relation), subject, certainty, state, wiki_source
                 FROM similar_pair WHERE state != 'discarded' ORDER BY a, b",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    Tie {
                        a: row.get(0)?,
                        b: row.get(1)?,
                        relation: row.get(2)?,
                        subject: row.get(3)?,
                        certainty: row.get(4)?,
                        joined: row.get::<_, String>(5)? == "joined",
                    },
                    row.get::<_, Option<i64>>(6)?,
                ))
            })
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?
    };
    let contents: Vec<i64> = identities.keys().copied().collect();
    let mut parent: HashMap<i64, i64> = contents.iter().map(|c| (*c, *c)).collect();
    fn find(parent: &mut HashMap<i64, i64>, x: i64) -> i64 {
        let mut at = x;
        while parent[&at] != at {
            at = parent[&at];
        }
        let root = at;
        let mut at = x;
        while parent[&at] != root {
            let next = parent[&at];
            parent.insert(at, root);
            at = next;
        }
        root
    }
    // A version relation, or two documents a person joined as one with one
    // of them as the wiki's (D-207).
    let is_version = |tie: &Tie, source: Option<i64>| words.version_relations.contains(&tie.relation) || (tie.joined && source.is_some());
    for (tie, source) in &ties {
        if !parent.contains_key(&tie.a) || !parent.contains_key(&tie.b) || !is_version(tie, *source) {
            continue;
        }
        let (ra, rb) = (find(&mut parent, tie.a), find(&mut parent, tie.b));
        if ra != rb {
            parent.insert(ra.max(rb), ra.min(rb));
        }
    }
    let mut grouped: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for content in &contents {
        let root = find(&mut parent, *content);
        grouped.entry(root).or_default().push(*content);
    }
    let mut out = Families { identities, ..Default::default() };
    for (smallest, mut members) in grouped {
        members.sort();
        let inside: Vec<(Tie, Option<i64>)> =
            ties.iter().filter(|(t, s)| members.contains(&t.a) && members.contains(&t.b) && is_version(t, *s)).cloned().collect();
        // The one the page is named after: the one a person chose as the
        // wiki's, or the signed copy, or the first.
        let principal = inside
            .iter()
            .find_map(|(t, source)| source.filter(|s| t.joined && members.contains(s)))
            .or_else(|| {
                inside
                    .iter()
                    .find_map(|(t, _)| t.subject.filter(|s| words.principal_relations.contains(&t.relation) && members.contains(s)))
            })
            .unwrap_or(members[0]);
        members.retain(|c| *c != principal);
        members.insert(0, principal);
        let index = out.list.len();
        for c in &members {
            out.of.insert(*c, index);
        }
        out.list.push(Family { id: format!("doc-{smallest}"), members, ties: inside.into_iter().map(|(t, _)| t).collect() });
    }
    for (tie, source) in ties {
        if let (Some(a), Some(b)) = (out.of.get(&tie.a), out.of.get(&tie.b)) {
            if a != b && !is_version(&tie, source) {
                out.across.push(tie);
            }
        }
    }
    Ok(out)
}

/// The name of the file of a document, without its extension: what its
/// page's path is made of (D-340), and its title when the base knows
/// nothing else of it.
pub(super) fn file_title(conn: &Connection, record: &Record, content: i64) -> Result<String, String> {
    let name = match record.documents.get(&content) {
        Some(document) if !document.name.is_empty() => document.name.clone(),
        _ => classify::name_of(conn, content)?,
    };
    Ok(title_of_name(name, content))
}

/// The name of a document's type: the one proposed with a type the
/// catalog did not have ("Resolución judicial"), or its id in words with
/// its first letter in capitals ("Carta documento").
fn type_name(conn: &Connection, content: i64) -> Result<Option<String>, String> {
    let row: Option<(String, Option<String>)> = conn
        .query_row("SELECT doc_type, proposed_name FROM document_type WHERE content_id = ?1", [content], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(sql_err)?;
    Ok(row.filter(|(id, _)| !id.trim().is_empty()).map(|(id, proposed)| {
        match proposed.map(|p| one_line(&p)).filter(|p| !p.is_empty()) {
            Some(name) => name,
            None => {
                let words = id.replace('-', " ");
                let mut chars = words.chars();
                chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
            }
        }
    }))
}

/// The data of a document as the base keeps it: (field, value), in order.
fn data_of(conn: &Connection, content: i64) -> Result<Vec<(String, String)>, String> {
    let row: Option<String> =
        conn.query_row("SELECT data FROM document_data WHERE content_id = ?1", [content], |row| row.get(0)).optional().map_err(sql_err)?;
    let list: Vec<serde_json::Value> = row.and_then(|d| serde_json::from_str(&d).ok()).unwrap_or_default();
    Ok(list
        .iter()
        .filter_map(|item| Some((item.get("field")?.as_str()?.to_string(), one_line(item.get("value")?.as_str()?))))
        .filter(|(_, value)| !value.is_empty())
        .collect())
}

/// The kind of a datum of a type of document (`identifier`, `date`…), as
/// the catalog of types says it.
fn data_kind(doc_type: Option<&str>, field: &str) -> Option<String> {
    let doc_type = doc_type?;
    let rule = crate::docbase::similar::types::catalog().types.iter().find(|t| t.id == doc_type)?.data.iter().find(|d| d.id == field)?;
    Some(rule.kind.clone())
}

/// Who a document's title names (P-182): by their roles in it, who it is
/// from and to whom, or between whom it is (two at most a side: more are
/// not named); empty when it names none of them.
fn parties_of(vocabulary: &Vocabulary, record: &Record, content: i64) -> String {
    let words = &vocabulary.pages;
    let roles = &words.title_roles;
    let named = |wanted: &[String], not: &[String]| -> Vec<String> {
        let mut list: Vec<String> = record
            .entities
            .values()
            .filter(|e| e.state == "active" && e.kind != vocabulary.matters.kind && vocabulary.is_a_name(&e.kind, &e.name))
            .filter(|e| e.documents.get(&content).is_some_and(|r| r.iter().any(|role| wanted.contains(role))))
            .map(|e| one_line(&e.name))
            .filter(|name| !not.contains(name))
            .collect();
        list.sort();
        list.dedup();
        list
    };
    let and = |list: &[String]| list.join(words.word("partiesAnd"));
    let fits = |list: &[String]| !list.is_empty() && list.len() <= 2;
    let from = named(&roles.from, &[]);
    let to = named(&roles.to, &from);
    let mut parts: Vec<String> = Vec::new();
    if fits(&from) {
        parts.push(format!("{} {}", words.word("docTitleFrom"), and(&from)));
    }
    if fits(&to) {
        parts.push(format!("{} {}", words.word("docTitleTo"), and(&to)));
    }
    if parts.is_empty() {
        let mut not = from.clone();
        not.extend(to);
        let between = named(&roles.between, &not);
        match between.len() {
            1 => parts.push(format!("{} {}", words.word("docTitleFrom"), between[0])),
            2 => parts.push(format!("{} {}", words.word("docTitleBetween"), and(&between))),
            _ => {}
        }
    }
    parts.join(" ")
}

/// The readable title of a document (P-182, D-442), from what the base
/// knows of it, without the model: its kind, its number, who it is from
/// and to whom (or between whom) and its date ("Factura 0001-00000001 de
/// Ejemplo Uno S.A. a Persona Dos · 1 de marzo de 2024"); the topic of its
/// summary when its kind is not known; none when the base knows neither.
fn readable_title(conn: &Connection, vocabulary: &Vocabulary, record: &Record, content: i64) -> Result<Option<String>, String> {
    let doc_type: Option<String> = conn
        .query_row("SELECT doc_type FROM document_type WHERE content_id = ?1", [content], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    let Some(doc_type) = doc_type.filter(|t| !t.trim().is_empty()) else {
        let topic: Option<String> = conn
            .query_row("SELECT topic FROM knowledge_summary WHERE content_id = ?1", [content], |row| row.get(0))
            .optional()
            .map_err(sql_err)?
            .flatten();
        return Ok(topic.map(|t| localized_dates(vocabulary, one_line(&t).trim_end_matches('.'))).filter(|t| !t.is_empty()));
    };
    let mut title = type_name(conn, content)?.unwrap_or_else(|| doc_type.replace('-', " "));
    let data = data_of(conn, content)?;
    // Its number: the first one the catalog of its type gives it once.
    let single = |field: &str| {
        crate::docbase::similar::types::catalog()
            .types
            .iter()
            .find(|t| t.id == doc_type)
            .and_then(|t| t.data.iter().find(|d| d.id == field))
            .is_some_and(|d| !d.many)
    };
    let kind_of = |field: &str| data_kind(Some(&doc_type), field);
    // A long one (the id of an envelope of a signing service) says nothing
    // to a reader.
    let short = |value: &str| value.chars().count() <= 24;
    if let Some((_, number)) = data.iter().find(|(field, value)| kind_of(field).as_deref() == Some("identifier") && single(field) && short(value)) {
        title.push_str(&format!(" {number}"));
    }
    let parties = parties_of(vocabulary, record, content);
    if !parties.is_empty() {
        title.push_str(&format!(" {parties}"));
    }
    let read: Option<String> = conn
        .query_row("SELECT doc_date FROM knowledge_extraction WHERE content_id = ?1", [content], |row| row.get(0))
        .optional()
        .map_err(sql_err)?
        .flatten();
    let date = read
        .filter(|d| !d.trim().is_empty())
        .or_else(|| data.iter().find(|(field, _)| kind_of(field).as_deref() == Some("date")).map(|(_, value)| value.clone()));
    if let Some(date) = date.and_then(|d| title_date(vocabulary, &d)) {
        title.push_str(&format!(" · {date}"));
    }
    Ok(Some(title.replace(['[', ']', '|'], " ").split_whitespace().collect::<Vec<_>>().join(" ")))
}

/// The readable title of each family's page, from the version it is named
/// after (`readable_title`), or its file's name. Two pages that would read
/// the same say their file's name too. The file's name stays in the data.
pub(super) fn titles(conn: &Connection, vocabulary: &Vocabulary, record: &Record, families: &Families) -> Result<BTreeMap<String, String>, String> {
    let mut out: BTreeMap<String, (String, String)> = BTreeMap::new();
    for family in &families.list {
        let principal = family.members[0];
        let file = file_title(conn, record, principal)?;
        let title = readable_title(conn, vocabulary, record, principal)?.unwrap_or_else(|| file.clone());
        out.insert(family.id.clone(), (title, file));
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (title, _) in out.values() {
        *seen.entry(title.to_lowercase()).or_default() += 1;
    }
    Ok(out
        .into_iter()
        .map(|(id, (title, file))| {
            let shown = if seen[&title.to_lowercase()] > 1 && title != file { format!("{title} ({file})") } else { title };
            (id, shown)
        })
        .collect())
}

/// The title of a document without the record: its file's name without
/// the extension (the graph names documents so, D-410).
pub(crate) fn document_title(conn: &Connection, content: i64) -> Result<String, String> {
    Ok(title_of_name(classify::name_of(conn, content)?, content))
}

fn title_of_name(name: String, content: i64) -> String {
    let stem = match name.rfind('.') {
        Some(dot) if dot > 0 && name.len() - dot <= 6 => name[..dot].to_string(),
        _ => name,
    };
    if stem.trim().is_empty() {
        format!("#{content}")
    } else {
        stem
    }
}

/// Where the page of a family goes: where it was (the links to it stay),
/// or a new logical path of its title.
pub(super) fn page_paths(
    conn: &Connection,
    vocabulary: &Vocabulary,
    record: &Record,
    families: &Families,
) -> Result<BTreeMap<String, String>, String> {
    let folder = vocabulary.pages.documents_folder();
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for family in &families.list {
        if let Some(page) = record.pages.get(&family.id) {
            if taken.insert(page.path.to_lowercase()) {
                out.insert(family.id.clone(), page.path.clone());
            }
        }
    }
    // Every path a page holds now: a new page never takes one (the table
    // keeps paths unique). The pages of a document the old wiki wrote are
    // no longer taken over: their notes came into the base once (D-403).
    let owned: BTreeSet<String> = record.pages.values().map(|p| p.path.to_lowercase()).collect();
    for family in &families.list {
        if out.contains_key(&family.id) {
            continue;
        }
        let title = file_title(conn, record, family.members[0])?;
        let mut path = format!("{folder}/{}.md", slug(&title, "documento"));
        if taken.contains(&path.to_lowercase()) || owned.contains(&path.to_lowercase()) {
            path = format!("{folder}/{}-{}.md", slug(&title, "documento"), family.id.trim_start_matches("doc-"));
        }
        taken.insert(path.to_lowercase());
        out.insert(family.id.clone(), path);
    }
    Ok(out)
}

/// How each version is called on the page: its stage, or "version"; a
/// number when two would read the same.
fn version_labels(conn: &Connection, vocabulary: &Vocabulary, family: &Family) -> Result<Vec<String>, String> {
    let mut labels: Vec<String> = Vec::new();
    for content in &family.members {
        let stage: Option<String> = conn
            .query_row("SELECT stage FROM document_type WHERE content_id = ?1", [content], |row| row.get(0))
            .optional()
            .map_err(sql_err)?
            .flatten();
        labels.push(stage.map(|s| s.replace('-', " ")).unwrap_or_else(|| vocabulary.pages.word("docVersion").to_string()));
    }
    let mut out = labels.clone();
    for (i, label) in labels.iter().enumerate() {
        if labels.iter().filter(|l| *l == label).count() > 1 {
            let n = labels[..=i].iter().filter(|l| *l == label).count();
            out[i] = format!("{label} {n}");
        }
    }
    Ok(out)
}

/// "(firmado, borrador)" for what is in some versions only; nothing for
/// what is in all of them or when there is one.
fn versions_mark(labels: &[String], family: &Family, contents: &BTreeSet<i64>) -> String {
    if family.members.len() < 2 || family.members.iter().all(|c| contents.contains(c)) {
        return String::new();
    }
    let named: Vec<&str> = family.members.iter().zip(labels).filter(|(c, _)| contents.contains(c)).map(|(_, l)| l.as_str()).collect();
    if named.is_empty() {
        String::new()
    } else {
        format!(" ({})", named.join(", "))
    }
}

/// The type, stage and date of a document, as the base knows them.
fn facts_of(conn: &Connection, vocabulary: &Vocabulary, content: i64) -> Result<Vec<String>, String> {
    let words = &vocabulary.pages;
    let (doc_type, stage): (Option<String>, Option<String>) = conn
        .query_row("SELECT doc_type, stage FROM document_type WHERE content_id = ?1", [content], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(sql_err)?
        .unwrap_or((None, None));
    let date: Option<String> = conn
        .query_row("SELECT doc_date FROM knowledge_extraction WHERE content_id = ?1", [content], |row| row.get(0))
        .optional()
        .map_err(sql_err)?
        .flatten();
    let mut facts: Vec<String> = Vec::new();
    if let Some(kind) = doc_type {
        let kind = type_name(conn, content)?.unwrap_or_else(|| kind.replace('-', " "));
        facts.push(format!("{}: {}", words.word("docKind"), stage.map(|s| format!("{kind} ({})", s.replace('-', " "))).unwrap_or(kind)));
    }
    if let Some(date) = date {
        facts.push(format!("{}: {}", words.word("docDate"), spelled_date(vocabulary, &date)));
    }
    Ok(facts)
}

/// The words of a document's text, to know which versions say a quote.
fn text_words(conn: &Connection, content: i64) -> Result<Vec<String>, String> {
    let body: Option<String> = conn
        .query_row("SELECT body FROM derivative WHERE content_id = ?1 AND kind = 'text'", [content], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(quote::words(&body.unwrap_or_default()))
}

/// How a relation reads: "es el ejemplar firmado de" for one with a
/// direction, "misma plantilla" for one without.
fn relation_words(vocabulary: &Vocabulary, relation: &str) -> (String, bool) {
    let directed = crate::docbase::similar::types::catalog().relations.iter().find(|r| r.id == relation).is_some_and(|r| r.directed);
    let key = format!("rel-{relation}");
    let label = vocabulary.pages.words.get(&key).cloned().unwrap_or_else(|| relation.replace('-', " "));
    (label, directed)
}

fn tie_state(vocabulary: &Vocabulary, tie: &Tie) -> String {
    let words = &vocabulary.pages;
    if tie.joined {
        format!(" *({})*", words.word("docJoined"))
    } else {
        format!(" *({})*", words.word("docProposed"))
    }
}

/// Where the block of entities goes inside the text of a document's page
/// before it is put there (the links of the text are added first).
pub(super) const BLOCK_PLACE: &str = "\u{1}micelya-bloque\u{1}";

/// The text the base writes of a family's page (without the notes), with
/// `BLOCK_PLACE` where the block of entities goes.
pub(super) fn render_document(
    conn: &Connection,
    vocabulary: &Vocabulary,
    record: &Record,
    paths: &BTreeMap<String, String>,
    families: &Families,
    family: &Family,
) -> Result<String, String> {
    let words = &vocabulary.pages;
    let principal = family.members[0];
    let title = families.title(conn, record, family)?;
    let file = classify::name_of(conn, principal)?;
    let labels = version_labels(conn, vocabulary, family)?;
    let label_of = |content: i64| family.members.iter().position(|c| *c == content).map(|i| labels[i].clone()).unwrap_or_default();
    let several = family.members.len() > 1;
    let mut sources: Vec<String> = Vec::new();
    for content in &family.members {
        for identity in families.identities.get(content).into_iter().flatten() {
            if !sources.contains(identity) {
                sources.push(identity.clone());
            }
        }
    }
    let quoted: Vec<String> = sources.iter().map(|s| serde_json::to_string(s).unwrap_or_default()).collect();
    let mut out = format!(
        "---\ntype: source\ntitle: {}\nfile: {}\ndocument_id: {}\nsources: [{}]\ngenerated_by: {GENERATED_BY}\n---\n\n# {title}\n\n",
        yaml_string(&title),
        yaml_string(&file),
        family.id,
        quoted.join(", ")
    );
    // The quotes of its summary, as notes (D-441).
    let mut notes = Notes::default();

    // The summaries of the one reading, by version.
    let mut summaries: Vec<(i64, Option<String>, Vec<super::super::extract::SummaryPoint>)> = Vec::new();
    for &content in &family.members {
        let row: Option<(Option<String>, String)> = conn
            .query_row("SELECT topic, points FROM knowledge_summary WHERE content_id = ?1", [content], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .map_err(sql_err)?;
        if let Some((topic, points)) = row {
            summaries.push((content, topic, serde_json::from_str(&points).unwrap_or_default()));
        }
    }
    // Its topic, unless it is the title already.
    if let Some(topic) = summaries.iter().find_map(|(_, t, _)| t.clone()).map(|t| localized_dates(vocabulary, &one_line(&t))) {
        if topic.trim_end_matches('.') != title {
            out.push_str(&format!("{topic}\n\n"));
        }
    }
    let facts = facts_of(conn, vocabulary, principal)?;
    if !facts.is_empty() {
        out.push_str(&format!("{}\n\n", facts.join(" · ")));
    }
    out.push_str(BLOCK_PLACE);
    out.push_str("\n\n");

    // Its versions and where each one is; or where it is.
    if several {
        out.push_str(&format!("## {}\n\n", words.word("docVersions")));
        for (content, label) in family.members.iter().zip(&labels) {
            let facts = facts_of(conn, vocabulary, *content)?;
            let places: Vec<String> =
                families.identities.get(content).into_iter().flatten().map(|p| format!("`{p}`")).collect();
            let mut line = format!("- **{label}**");
            if !facts.is_empty() {
                line.push_str(&format!(" — {}", facts.join(" · ")));
            }
            if !places.is_empty() {
                line.push_str(&format!(" — {}: {}", words.word("docLocations"), places.join(", ")));
            }
            out.push_str(&line);
            out.push('\n');
        }
        for tie in &family.ties {
            let (label, directed) = relation_words(vocabulary, &tie.relation);
            let line = match tie.subject.filter(|_| directed) {
                Some(subject) => {
                    let other = if subject == tie.a { tie.b } else { tie.a };
                    format!("- {} {label} {}", label_of(subject), label_of(other))
                }
                None => format!("- {} · {}: {label}", label_of(tie.a), label_of(tie.b)),
            };
            out.push_str(&format!("{line}{}\n", tie_state(vocabulary, tie)));
        }
        out.push_str(&format!("\n*{}*\n\n", words.word("docVersionsNote")));
    } else {
        let places: Vec<String> = families.identities.get(&principal).into_iter().flatten().map(|p| format!("- `{p}`\n")).collect();
        if !places.is_empty() {
            out.push_str(&format!("## {}\n\n{}\n", words.word("docLocations"), places.concat()));
        }
    }

    // The data of its type, each with the versions that have it.
    let mut data: BTreeMap<(String, String), BTreeSet<i64>> = BTreeMap::new();
    let mut order: Vec<(String, String)> = Vec::new();
    let principal_type: Option<String> =
        conn.query_row("SELECT doc_type FROM document_type WHERE content_id = ?1", [principal], |row| row.get(0)).optional().map_err(sql_err)?;
    for &content in &family.members {
        for (field, value) in data_of(conn, content)? {
            let value = if data_kind(principal_type.as_deref(), &field).as_deref() == Some("date") { spelled_date(vocabulary, &value) } else { value };
            let key = (field, value);
            if !data.contains_key(&key) {
                order.push(key.clone());
            }
            data.entry(key).or_default().insert(content);
        }
    }
    if !order.is_empty() {
        // One line per field, its values in the order the base keeps them.
        out.push_str(&format!("## {}\n\n", words.word("docData")));
        let mut fields: Vec<&str> = Vec::new();
        for key in &order {
            if !fields.contains(&key.0.as_str()) {
                fields.push(&key.0);
            }
        }
        for field in fields {
            let values: Vec<String> = order
                .iter()
                .filter(|k| k.0 == field)
                .map(|k| format!("{}{}", k.1, versions_mark(&labels, family, &data[k])))
                .collect();
            out.push_str(&format!("- {}: {}\n", field.replace('-', " "), values.join(", ")));
        }
        out.push('\n');
    }

    // What the one reading summed up: each point once, with the versions
    // whose text has its quote.
    let texts: Vec<(i64, Vec<String>)> = if several {
        family.members.iter().map(|c| text_words(conn, *c).map(|w| (*c, w))).collect::<Result<_, _>>()?
    } else {
        Vec::new()
    };
    let claims: HashMap<i64, &Claim> = record.entities.values().flat_map(|e| e.claims.iter().map(|c| (c.id, c))).collect();
    let mut lines: Vec<String> = Vec::new();
    let mut seen: BTreeSet<Vec<String>> = BTreeSet::new();
    let mut shown: Vec<String> = Vec::new();
    for (index, (content, _, points)) in summaries.iter().enumerate() {
        let mut of_evidence: HashMap<i64, i64> = HashMap::new();
        {
            let mut statement =
                conn.prepare("SELECT id, claim_id FROM knowledge_evidence WHERE content_id = ?1 AND claim_id IS NOT NULL").map_err(sql_err)?;
            let rows = statement.query_map([content], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).map_err(sql_err)?;
            for row in rows {
                let (evidence, claim) = row.map_err(sql_err)?;
                of_evidence.insert(evidence, claim);
            }
        }
        for point in points {
            let said = quote::words(&point.quote);
            if !seen.insert(said.clone()) {
                continue;
            }
            // Another version says it in the same passage: said once, its
            // versions are the ones whose text has the quote.
            if index > 0 && shown.iter().any(|q| super::super::extract::same_passage(q, &point.quote)) {
                continue;
            }
            shown.push(point.quote.clone());
            let mut marks: Vec<String> = Vec::new();
            for evidence in &point.evidence {
                if let Some(claim) = of_evidence.get(evidence).and_then(|c| claims.get(c)) {
                    let mark = certainty_mark(vocabulary, claim);
                    if !mark.is_empty() && !marks.contains(&mark) {
                        marks.push(mark);
                    }
                }
            }
            let mut with: BTreeSet<i64> = texts.iter().filter(|(_, w)| quote::contains(w, &said)).map(|(c, _)| *c).collect();
            with.insert(*content);
            let page = point.page.map(|p| format!(", {} {p}", words.word("page"))).unwrap_or_default();
            let note = notes.mark(&format!("«{}»{page}{}", one_line(&point.quote), versions_mark(&labels, family, &with)));
            lines.push(format!("- {}{}{note}\n", localized_dates(vocabulary, &one_line(&point.text)), marks.concat()));
        }
    }
    if !lines.is_empty() {
        out.push_str(&format!("## {}\n\n{}\n", words.word("docSummary"), lines.concat()));
    } else if summaries.is_empty() {
        out.push_str(&format!("{}\n\n", words.word("docPending")));
    }

    // The amounts between parties the record keeps from its versions,
    // whatever the summary chose to say (D-331).
    let members: BTreeSet<i64> = family.members.iter().copied().collect();
    let mut amounts: Vec<String> = Vec::new();
    for entity in record.entities.values().filter(|e| e.state == "active") {
        for claim in entity.claims.iter().filter(|c| c.amount.is_some() && !c.mirrored) {
            let from: BTreeSet<i64> = claim.evidence.iter().map(|(c, ..)| *c).filter(|c| members.contains(c)).collect();
            if from.is_empty() {
                continue;
            }
            let other = claim.object_entity.as_ref().and_then(|o| record.entities.get(o)).map(|o| o.name.clone()).unwrap_or_default();
            let when = claim.valid_from.as_deref().map(|t| format!(" · {}", localized_dates(vocabulary, &one_line(t)))).unwrap_or_default();
            amounts.push(format!(
                "- {} → {other}: {}{when}{}{}\n",
                entity.name,
                amount_text(vocabulary, claim),
                certainty_mark(vocabulary, claim),
                versions_mark(&labels, family, &from)
            ));
        }
    }
    if !amounts.is_empty() {
        amounts.sort();
        amounts.dedup();
        out.push_str(&format!("## {}\n\n{}\n", words.word("docAmounts"), amounts.concat()));
    }

    // The documents it is related to, with a link to their page.
    let mut related: Vec<String> = Vec::new();
    for tie in &families.across {
        let (mine, other) = if members.contains(&tie.a) {
            (tie.a, tie.b)
        } else if members.contains(&tie.b) {
            (tie.b, tie.a)
        } else {
            continue;
        };
        let Some(other_family) = families.family_of(other) else { continue };
        let Some(path) = paths.get(&other_family.id) else { continue };
        let target = link(path, &families.title(conn, record, other_family)?);
        let (label, directed) = relation_words(vocabulary, &tie.relation);
        let this = if several { format!("{} ({})", words.word("docThis"), label_of(mine)) } else { words.word("docThis").to_string() };
        let line = match tie.subject.filter(|_| directed) {
            Some(subject) if subject == mine => format!("- {this} {label} {target}"),
            Some(_) => format!("- {target} {label} {}", lowercase_first(&this)),
            None => format!("- {label}: {target}"),
        };
        related.push(format!("{line}{}\n", tie_state(vocabulary, tie)));
    }
    if !related.is_empty() {
        related.dedup();
        out.push_str(&format!("## {}\n\n{}\n", words.word("docRelated"), related.concat()));
    }
    if !lines.is_empty() || !amounts.is_empty() {
        out.push_str(&format!("*{}*\n", words.word("certainLegend")));
    }
    let block = notes.block();
    if !block.is_empty() {
        out.push('\n');
        out.push_str(&block);
    }
    Ok(out)
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
