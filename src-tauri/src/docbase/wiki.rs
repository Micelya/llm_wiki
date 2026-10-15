//! An index of the general project's wiki: its pages, the links between
//! them and the document each page came from.
//!
//! The pages are text files and stay the truth. This is a copy that can
//! be asked questions without reading every page again, and that is
//! rebuilt from the files: a page is read again only when its size or
//! date changed, and a page that is gone is dropped.
//!
//! A page names its source by the path the wiki knows it by
//! (`<origin>/<path inside it>`). The index ties that to the file in the
//! inventory, so "which pages cite this document" is answered by what
//! the document is, not by what its file happens to be called.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use walkdir::WalkDir;

use super::{sql_err, DocBase, Visible};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiIndex {
    pub pages: i64,
    pub links: i64,
    /// Links that point at no page.
    pub broken: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRef {
    /// Path inside the project: `wiki/entities/boreal.md`.
    pub path: String,
    /// Where the page is on disk.
    pub file: String,
    pub title: String,
    /// `type` of its header: source, entity, concept…
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokenLink {
    pub from: PageRef,
    /// What the link says, as written.
    pub target: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ParsedPage {
    title: String,
    kind: String,
    sources: Vec<String>,
    /// (target as written, "body" or "related")
    links: Vec<(String, &'static str)>,
}

impl DocBase {
    /// Brings the index up to date with the pages on disk. Cheap when
    /// nothing changed. Returns the totals; all zero when the general
    /// project has no wiki.
    pub fn index_wiki(&self) -> Result<WikiIndex, String> {
        let Some(project) = self.general_project()? else {
            return Ok(WikiIndex::default());
        };
        let Some(root) = project.path.clone() else {
            return Ok(WikiIndex::default());
        };
        let root = Path::new(&root);

        // Reading the folder happens without the lock.
        let mut on_disk: Vec<(String, i64, i64)> = Vec::new();
        for entry in WalkDir::new(root.join("wiki")).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() || entry.path().extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let (Ok(rel), Ok(meta)) = (entry.path().strip_prefix(root), entry.metadata()) else {
                continue;
            };
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            on_disk.push((rel.to_string_lossy().replace('\\', "/"), meta.len() as i64, mtime_ms));
        }

        let known: HashMap<String, (i64, i64, i64)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare("SELECT path, id, size, mtime_ms FROM wiki_page WHERE project_id = ?1")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([&project.id], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?, row.get(3)?))))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };

        let mut changed: Vec<(String, i64, i64, ParsedPage)> = Vec::new();
        for (path, size, mtime_ms) in &on_disk {
            if known.get(path).is_some_and(|(_, s, m)| s == size && m == mtime_ms) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(root.join(path)) else {
                continue;
            };
            changed.push((path.clone(), *size, *mtime_ms, parse_page(&text, path)));
        }
        let gone: Vec<i64> = known
            .iter()
            .filter(|(path, _)| !on_disk.iter().any(|(disk, ..)| disk == *path))
            .map(|(_, (id, ..))| *id)
            .collect();

        // Which origin each mounted folder is, to tie sources to files.
        let mounts: Vec<(String, String)> = project
            .origins
            .iter()
            .filter_map(|origin| Some((origin.mounted_as.clone()?.to_lowercase(), origin.origin_id.clone())))
            .collect();

        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        for id in &gone {
            tx.execute("DELETE FROM wiki_page WHERE id = ?1", [id]).map_err(sql_err)?;
        }
        for (path, size, mtime_ms, page) in &changed {
            tx.execute(
                "INSERT INTO wiki_page (project_id, path, slug, title, type, size, mtime_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (project_id, path) DO UPDATE SET
                     title = excluded.title, type = excluded.type, size = excluded.size, mtime_ms = excluded.mtime_ms",
                params![project.id, path, slug_of(path), page.title, page.kind, size, mtime_ms],
            )
            .map_err(sql_err)?;
            let page_id: i64 = tx
                .query_row(
                    "SELECT id FROM wiki_page WHERE project_id = ?1 AND path = ?2",
                    params![project.id, path],
                    |row| row.get(0),
                )
                .map_err(sql_err)?;
            tx.execute("DELETE FROM wiki_link WHERE from_page = ?1", [page_id]).map_err(sql_err)?;
            tx.execute("DELETE FROM wiki_page_source WHERE page_id = ?1", [page_id]).map_err(sql_err)?;
            for (target, kind) in &page.links {
                tx.execute(
                    "INSERT OR IGNORE INTO wiki_link (from_page, target, kind) VALUES (?1, ?2, ?3)",
                    params![page_id, target, kind],
                )
                .map_err(sql_err)?;
            }
            for identity in &page.sources {
                tx.execute(
                    "INSERT OR IGNORE INTO wiki_page_source (page_id, identity) VALUES (?1, ?2)",
                    params![page_id, identity],
                )
                .map_err(sql_err)?;
            }
        }

        // Targets are worked out again in full: a new page can mend a
        // link that was broken, a deleted one can break others.
        let pages: Vec<(i64, String, String)> = {
            let mut statement = tx
                .prepare("SELECT id, slug, title FROM wiki_page WHERE project_id = ?1 ORDER BY path")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([&project.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let mut by_name: HashMap<String, i64> = HashMap::new();
        for (id, _, title) in &pages {
            by_name.entry(link_key(title)).or_insert(*id);
        }
        // A file name wins over a title that happens to read the same.
        for (id, slug, _) in pages.iter().rev() {
            by_name.insert(link_key(slug), *id);
        }
        let links: Vec<(i64, String, String)> = {
            let mut statement = tx
                .prepare(
                    "SELECT k.from_page, k.target, k.kind FROM wiki_link k
                     JOIN wiki_page p ON p.id = k.from_page WHERE p.project_id = ?1",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([&project.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (from_page, target, kind) in &links {
            tx.execute(
                "UPDATE wiki_link SET to_page = ?1 WHERE from_page = ?2 AND target = ?3 AND kind = ?4",
                params![by_name.get(&link_key(target)), from_page, target, kind],
            )
            .map_err(sql_err)?;
        }

        let sources: Vec<(i64, String)> = {
            let mut statement = tx
                .prepare(
                    "SELECT s.page_id, s.identity FROM wiki_page_source s
                     JOIN wiki_page p ON p.id = s.page_id WHERE p.project_id = ?1",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([&project.id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (page_id, identity) in &sources {
            let lower = identity.to_lowercase();
            let location: Option<i64> = match mounts
                .iter()
                .find(|(mount, _)| lower.starts_with(mount.as_str()) && lower.as_bytes().get(mount.len()) == Some(&b'/'))
            {
                Some((mount, origin_id)) => tx
                    .query_row(
                        "SELECT id FROM location WHERE origin_id = ?1 AND rel_path = ?2 COLLATE NOCASE",
                        params![origin_id, &identity[mount.len() + 1..]],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(sql_err)?,
                None => None,
            };
            tx.execute(
                "UPDATE wiki_page_source SET location_id = ?1 WHERE page_id = ?2 AND identity = ?3",
                params![location, page_id, identity],
            )
            .map_err(sql_err)?;
        }

        let totals = tx
            .query_row(
                "SELECT (SELECT count(*) FROM wiki_page WHERE project_id = ?1),
                        (SELECT count(*) FROM wiki_link k JOIN wiki_page p ON p.id = k.from_page WHERE p.project_id = ?1),
                        (SELECT count(*) FROM wiki_link k JOIN wiki_page p ON p.id = k.from_page
                          WHERE p.project_id = ?1 AND k.to_page IS NULL)",
                [&project.id],
                |row| {
                    Ok(WikiIndex {
                        pages: row.get(0)?,
                        links: row.get(1)?,
                        broken: row.get(2)?,
                    })
                },
            )
            .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        Ok(totals)
    }

    /// The wiki pages that came from the document at this location:
    /// from this file or from any visible copy of the same document.
    pub fn pages_citing(&self, visible: &Visible, location_id: i64) -> Result<Vec<PageRef>, String> {
        self.index_wiki()?;
        let root = self.wiki_root()?;
        let (condition, bound) = visible.condition("l.origin_id");
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT DISTINCT p.path, p.title, p.type
                 FROM wiki_page_source s
                 JOIN wiki_page p ON p.id = s.page_id
                 JOIN location l ON l.id = s.location_id
                 WHERE {condition} AND (
                     l.id = ?
                     OR l.file_hash IN (
                         SELECT f2.hash FROM file f2
                         WHERE f2.content_id = (
                             SELECT f.content_id FROM location mine JOIN file f ON f.hash = mine.file_hash
                             WHERE mine.id = ?)))
                 ORDER BY p.type, p.title COLLATE NOCASE"
            ))
            .map_err(sql_err)?;
        let bound: Vec<rusqlite::types::Value> = bound
            .into_iter()
            .map(rusqlite::types::Value::Text)
            .chain([location_id, location_id].map(rusqlite::types::Value::Integer))
            .collect();
        let rows = statement
            .query_map(rusqlite::params_from_iter(bound), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(sql_err)?;
        let mut pages = Vec::new();
        for row in rows {
            let (path, title, kind) = row.map_err(sql_err)?;
            pages.push(page_ref(&root, path, title, kind));
        }
        Ok(pages)
    }

    /// Links that point at no page, for the user to mend.
    pub fn broken_links(&self, limit: i64) -> Result<Vec<BrokenLink>, String> {
        self.index_wiki()?;
        let root = self.wiki_root()?;
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT p.path, p.title, p.type, k.target FROM wiki_link k
                 JOIN wiki_page p ON p.id = k.from_page
                 WHERE k.to_page IS NULL ORDER BY p.path, k.target LIMIT ?1",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([limit.clamp(1, 1000)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(sql_err)?;
        let mut links = Vec::new();
        for row in rows {
            let (path, title, kind, target) = row.map_err(sql_err)?;
            links.push(BrokenLink {
                from: page_ref(&root, path, title, kind),
                target,
            });
        }
        Ok(links)
    }

    /// The pages that link to the page at this path inside the project.
    pub fn backlinks(&self, page_path: &str) -> Result<Vec<PageRef>, String> {
        self.index_wiki()?;
        let root = self.wiki_root()?;
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT DISTINCT p.path, p.title, p.type FROM wiki_link k
                 JOIN wiki_page p ON p.id = k.from_page
                 JOIN wiki_page target ON target.id = k.to_page
                 WHERE target.path = ?1 COLLATE NOCASE AND p.id != target.id
                 ORDER BY p.title COLLATE NOCASE",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([page_path.replace('\\', "/")], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(sql_err)?;
        let mut pages = Vec::new();
        for row in rows {
            let (path, title, kind) = row.map_err(sql_err)?;
            pages.push(page_ref(&root, path, title, kind));
        }
        Ok(pages)
    }

    fn wiki_root(&self) -> Result<String, String> {
        Ok(self
            .general_project()?
            .and_then(|project| project.path)
            .unwrap_or_default())
    }
}

fn page_ref(root: &str, path: String, title: String, kind: String) -> PageRef {
    PageRef {
        file: format!("{}/{}", root.trim_end_matches('/'), path),
        path,
        title,
        kind,
    }
}

/// File name without folder or extension: what a link usually says.
fn slug_of(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

/// A link target or a page name reduced to what has to match: no
/// folder, extension, anchor or alias; case, spaces, hyphens and
/// underscores do not count.
fn link_key(text: &str) -> String {
    let text = text.split('|').next().unwrap_or(text);
    let text = text.split('#').next().unwrap_or(text);
    let text = text.trim().replace('\\', "/");
    let name = text.rsplit('/').next().unwrap_or(&text);
    let name = name.strip_suffix(".md").unwrap_or(name);
    name.to_lowercase()
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '_'))
        .collect()
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value);
    value.trim().to_string()
}

/// A list written on one line, `[a, "b, c"]`, or a single value.
fn inline_list(value: &str) -> Vec<String> {
    let value = value.trim();
    let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) else {
        return if value.is_empty() { Vec::new() } else { vec![unquote(value)] };
    };
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in inner.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == ',' => {
                items.push(std::mem::take(&mut current));
            }
            None => current.push(c),
        }
    }
    items.push(current);
    items
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn parse_page(text: &str, path: &str) -> ParsedPage {
    let text = text.replace("\r\n", "\n");
    let mut page = ParsedPage::default();
    let mut body = text.as_str();
    let mut related: Vec<String> = Vec::new();

    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            body = &rest[end + 4..];
            let mut list_key: Option<String> = None;
            for line in rest[..end].lines() {
                if let Some(item) = line.trim_start().strip_prefix("- ") {
                    if line.starts_with(' ') || line.starts_with('-') {
                        match list_key.as_deref() {
                            Some("sources") => page.sources.push(unquote(item)),
                            Some("related") => related.push(unquote(item)),
                            _ => {}
                        }
                        continue;
                    }
                }
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                if key.starts_with(' ') {
                    continue;
                }
                let key = key.trim();
                list_key = value.trim().is_empty().then(|| key.to_string());
                match key {
                    "title" => page.title = unquote(value),
                    "type" => page.kind = unquote(value),
                    "sources" => page.sources.extend(inline_list(value)),
                    "related" => related.extend(inline_list(value)),
                    _ => {}
                }
            }
        }
    }
    if page.title.is_empty() {
        page.title = slug_of(path);
    }
    for target in related {
        let target = target.trim_start_matches("[[").trim_end_matches("]]").to_string();
        if !target.is_empty() {
            page.links.push((target, "related"));
        }
    }

    let mut rest = body;
    while let Some(start) = rest.find("[[") {
        // `![[image]]` embeds a file; it is not a link to a page.
        let embed = rest[..start].ends_with('!');
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else {
            break;
        };
        let target = after[..end].split('|').next().unwrap_or("").trim();
        if !embed && !target.is_empty() && !target.contains('\n') {
            let target = target.to_string();
            if !page.links.iter().any(|(known, kind)| *kind == "body" && *known == target) {
                page.links.push((target, "body"));
            }
        }
        rest = &after[end + 2..];
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    fn project(dir: &TempDir) -> (DocBase, String) {
        dir.write("casos/BOREAL/carta.md", b"una carta documento");
        dir.write("casos/copias/la misma carta.md", b"una carta documento");
        dir.write("casos/BOREAL/contrato.md", b"un contrato");
        dir.folder("General/raw/sources");
        dir.write("General/schema.md", b"# schema");
        dir.write(
            "General/wiki/sources/carta-documento.md",
            b"---\ntype: source\ntitle: \"Carta documento: intimaci\xc3\xb3n\"\nsources: [\"Casos/BOREAL/carta.md\"]\nrelated: [boreal]\n---\n\nBOREAL intima a [[Andina]] por las [[facturas-impagas|facturas]]. Ver ![[foto.png]].\n",
        );
        dir.write(
            "General/wiki/entities/boreal.md",
            b"---\ntype: entity\ntitle: BOREAL\nsources:\n  - Casos/BOREAL/carta.md\n  - Casos/BOREAL/contrato.md\n---\n\nCliente. Ver [[carta-documento]] y [[BOREAL]].\n",
        );
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        base.link_general_project(&dir.path().join("General"), "default").unwrap();
        (base, origin.id)
    }

    fn location(base: &DocBase, rel_path: &str) -> i64 {
        base.conn()
            .query_row("SELECT id FROM location WHERE rel_path = ?1", [rel_path], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn a_page_gives_its_title_type_sources_and_links() {
        let page = parse_page(
            "---\ntype: source\ntitle: \"Carta: intimación\"\nsources: [\"Casos/BOREAL/carta, final.pdf\", otro.pdf]\nrelated:\n  - boreal\n  - \"[[andina]]\"\ntags: [a, b]\n---\n\nVer [[Andina S.A.|Andina]], [[boreal#historia]] y otra vez [[Andina S.A.]]. ![[foto.png]]\n",
            "wiki/sources/carta.md",
        );
        assert_eq!(page.title, "Carta: intimación");
        assert_eq!(page.kind, "source");
        assert_eq!(page.sources, ["Casos/BOREAL/carta, final.pdf", "otro.pdf"]);
        assert_eq!(
            page.links,
            [
                ("boreal".to_string(), "related"),
                ("andina".to_string(), "related"),
                ("Andina S.A.".to_string(), "body"),
                ("boreal#historia".to_string(), "body"),
            ]
        );
        // Without a header the file name is the title.
        assert_eq!(parse_page("Solo texto.", "wiki/concepts/mora.md").title, "mora");
    }

    #[test]
    fn the_index_knows_pages_links_and_which_are_broken() {
        let dir = TempDir::new();
        let (base, _) = project(&dir);
        let index = base.index_wiki().unwrap();
        // carta: related boreal, Andina, facturas-impagas. boreal: carta-documento, BOREAL.
        assert_eq!((index.pages, index.links, index.broken), (2, 5, 2));

        let broken: Vec<String> = base.broken_links(10).unwrap().into_iter().map(|link| link.target).collect();
        assert_eq!(broken, ["Andina", "facturas-impagas"]);
        let back: Vec<String> = base.backlinks("wiki/entities/boreal.md").unwrap().into_iter().map(|p| p.title).collect();
        assert_eq!(back, ["Carta documento: intimación"]);
    }

    #[test]
    fn a_new_page_mends_a_broken_link_and_a_deleted_one_breaks_it_again() {
        let dir = TempDir::new();
        let (base, _) = project(&dir);
        assert_eq!(base.index_wiki().unwrap().broken, 2);

        dir.write("General/wiki/entities/andina.md", b"---\ntype: entity\ntitle: Andina\n---\nProveedor.\n");
        let index = base.index_wiki().unwrap();
        assert_eq!((index.pages, index.broken), (3, 1));

        dir.remove("General/wiki/entities/andina.md");
        let index = base.index_wiki().unwrap();
        assert_eq!((index.pages, index.broken), (2, 2));
    }

    #[test]
    fn pages_are_found_by_the_document_they_cite_whatever_the_copy() {
        let dir = TempDir::new();
        let (base, _) = project(&dir);
        let titles = |rel_path: &str| -> Vec<String> {
            base.pages_citing(&Visible::All, location(&base, rel_path))
                .unwrap()
                .into_iter()
                .map(|page| page.title)
                .collect()
        };

        assert_eq!(titles("BOREAL/carta.md"), ["BOREAL", "Carta documento: intimación"]);
        // The copy under another name was never cited by name, yet it is the same document.
        assert_eq!(titles("copias/la misma carta.md"), ["BOREAL", "Carta documento: intimación"]);
        assert_eq!(titles("BOREAL/contrato.md"), ["BOREAL"]);

        let page = &base.pages_citing(&Visible::All, location(&base, "BOREAL/contrato.md")).unwrap()[0];
        assert_eq!(page.path, "wiki/entities/boreal.md");
        assert!(page.file.ends_with("General/wiki/entities/boreal.md"));
        assert!(base.pages_citing(&Visible::Origins(vec![]), location(&base, "BOREAL/carta.md")).unwrap().is_empty());
    }

    #[test]
    fn link_targets_match_however_they_are_written() {
        assert_eq!(link_key("Andina S.A."), link_key("andina-s.a."));
        assert_eq!(link_key("wiki/entities/BOREAL.md"), link_key("boreal"));
        assert_eq!(link_key("boreal#historia"), link_key("BOREAL|el cliente"));
        assert_ne!(link_key("boreal"), link_key("boreal-argentina"));
    }
}
