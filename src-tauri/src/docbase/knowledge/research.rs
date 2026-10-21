//! The researches on the internet a person asked for (D-397, D-414): each
//! one kept in the base with its sources, of a person, an organization or
//! a matter (`object_id`) or loose, and always apart from what the
//! documents say: no page, no relation and no datum of the record comes
//! from them.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::articles::kind_of;
use crate::docbase::{now_ms, sql_err, DocBase};

/// A source a research cites: what the search on the internet returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSource {
    /// Its number in the text (`[n]`).
    pub index: usize,
    pub title: String,
    pub url: String,
    /// Who found it (the search provider, or a local search).
    #[serde(default)]
    pub source: String,
}

/// A research kept in the base.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Research {
    pub id: i64,
    /// `ent-…`, `mat-…` or `doc-…`; none for a loose one.
    pub object_id: Option<String>,
    pub topic: String,
    /// The synthesis, in Markdown, citing its sources as `[n]`.
    pub body: String,
    pub sources: Vec<ResearchSource>,
    pub created_at: i64,
}

const COLUMNS: &str = "id, object_id, topic, body, sources, created_at";

fn row(row: &rusqlite::Row) -> rusqlite::Result<Research> {
    Ok(Research {
        id: row.get(0)?,
        object_id: row.get(1)?,
        topic: row.get(2)?,
        body: row.get(3)?,
        sources: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
        created_at: row.get(5)?,
    })
}

impl DocBase {
    /// Keeps a research, of `object_id` or loose.
    pub fn save_research(
        &self,
        object_id: Option<&str>,
        topic: &str,
        body: &str,
        sources: &[ResearchSource],
    ) -> Result<Research, String> {
        let object_id = object_id.map(str::trim).filter(|id| !id.is_empty());
        if let Some(id) = object_id {
            if kind_of(id).is_none() {
                return Err(format!("'{id}' no es una persona, organización, asunto ni documento"));
            }
        }
        let topic = topic.trim();
        if topic.is_empty() || body.trim().is_empty() {
            return Err("Una investigación necesita un tema y un texto".into());
        }
        let conn = self.conn();
        conn.execute(
            "INSERT INTO knowledge_research (object_id, topic, body, sources, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![object_id, topic, body, serde_json::to_string(sources).map_err(|e| e.to_string())?, now_ms()],
        )
        .map_err(sql_err)?;
        let id = conn.last_insert_rowid();
        conn.query_row(&format!("SELECT {COLUMNS} FROM knowledge_research WHERE id = ?1"), [id], row)
            .map_err(sql_err)
    }

    /// The researches of an object, the latest first.
    pub fn research_of(&self, object_id: &str) -> Result<Vec<Research>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM knowledge_research WHERE object_id = ?1 ORDER BY created_at DESC, id DESC"
            ))
            .map_err(sql_err)?;
        let rows = statement.query_map([object_id], row).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// The loose researches (of no object), the latest first.
    pub fn loose_research(&self) -> Result<Vec<Research>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM knowledge_research WHERE object_id IS NULL ORDER BY created_at DESC, id DESC"
            ))
            .map_err(sql_err)?;
        let rows = statement.query_map([], row).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// One research by its number.
    pub fn research(&self, id: i64) -> Result<Option<Research>, String> {
        self.conn()
            .query_row(&format!("SELECT {COLUMNS} FROM knowledge_research WHERE id = ?1"), [id], row)
            .optional()
            .map_err(sql_err)
    }

    pub fn delete_research(&self, id: i64) -> Result<(), String> {
        self.conn().execute("DELETE FROM knowledge_research WHERE id = ?1", [id]).map(|_| ()).map_err(sql_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(index: usize) -> ResearchSource {
        ResearchSource {
            index,
            title: format!("Nota {index}"),
            url: format!("https://ejemplo.test/{index}"),
            source: "prueba".into(),
        }
    }

    #[test]
    fn a_research_is_kept_of_its_object_or_loose_with_its_sources() {
        let base = DocBase::in_memory().unwrap();
        let of_person = base.save_research(Some("ent-a"), "Ana Prueba", "Lo publicado [1].", &[source(1)]).unwrap();
        base.save_research(Some("mat-1"), "Un asunto", "Texto [1] [2].", &[source(1), source(2)]).unwrap();
        let loose = base.save_research(None, "Tema suelto", "Texto.", &[]).unwrap();
        assert_eq!(base.research_of("ent-a").unwrap(), vec![of_person.clone()]);
        assert_eq!(base.research_of("mat-1").unwrap()[0].sources.len(), 2);
        assert_eq!(base.loose_research().unwrap(), vec![loose.clone()]);
        assert_eq!(base.research(of_person.id).unwrap(), Some(of_person.clone()));
        base.delete_research(of_person.id).unwrap();
        assert!(base.research_of("ent-a").unwrap().is_empty());
        // An empty object is a loose one.
        assert!(base.save_research(Some(" "), "Otro", "Texto.", &[]).unwrap().object_id.is_none());
    }

    #[test]
    fn a_research_needs_a_topic_a_text_and_a_known_kind_of_object() {
        let base = DocBase::in_memory().unwrap();
        assert!(base.save_research(Some("ent-a"), " ", "Texto.", &[]).is_err());
        assert!(base.save_research(Some("ent-a"), "Tema", "  ", &[]).is_err());
        assert!(base.save_research(Some("otro-1"), "Tema", "Texto.", &[]).is_err());
    }

    #[test]
    fn a_research_is_never_part_of_what_the_documents_say() {
        let base = DocBase::in_memory().unwrap();
        base.save_research(Some("ent-a"), "Ana Prueba", "Dato de internet [1].", &[source(1)]).unwrap();
        let conn = base.conn();
        let claims: i64 = conn.query_row("SELECT count(*) FROM knowledge_claim", [], |r| r.get(0)).unwrap();
        let pages: i64 = conn.query_row("SELECT count(*) FROM knowledge_page", [], |r| r.get(0)).unwrap();
        assert_eq!((claims, pages), (0, 0));
    }
}
