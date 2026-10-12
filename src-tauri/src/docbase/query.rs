//! Read-only views of the base for the screens.

use rusqlite::params_from_iter;
use serde::Serialize;

use super::origins::{self, Origin};
use super::{sql_err, DocBase, Visible};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginSummary {
    #[serde(flatten)]
    pub origin: Origin,
    /// Files in the inventory, shortcuts included.
    pub files: i64,
    /// Files still waiting to be read.
    pub pending: i64,
    /// Files that could not be read.
    pub errors: i64,
    /// Cloud shortcuts: detected, not processable.
    pub links: i64,
}

impl DocBase {
    pub fn origin_summaries(&self, visible: &Visible) -> Result<Vec<OriginSummary>, String> {
        let conn = self.conn();
        let origins = origins::list(&conn, visible)?;
        let (condition, bound) = visible.condition("origin_id");
        let mut statement = conn
            .prepare(&format!(
                "SELECT origin_id,
                        count(*),
                        sum(hash_state = 'pending'),
                        sum(hash_state = 'error'),
                        sum(kind = 'link')
                 FROM location WHERE {condition} GROUP BY origin_id"
            ))
            .map_err(sql_err)?;
        let counts = statement
            .query_map(params_from_iter(bound.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;

        Ok(origins
            .into_iter()
            .map(|origin| {
                let (files, pending, errors, links) = counts
                    .iter()
                    .find(|(id, ..)| *id == origin.id)
                    .map(|(_, files, pending, errors, links)| (*files, *pending, *errors, *links))
                    .unwrap_or((0, 0, 0, 0));
                OriginSummary {
                    origin,
                    files,
                    pending,
                    errors,
                    links,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    #[test]
    fn an_origin_summary_counts_its_files_by_state() {
        let dir = TempDir::new();
        dir.write("o/a.pdf", b"a");
        dir.write("o/b.pdf", b"b");
        dir.write("o/notas.gdoc", b"{}");
        dir.folder("vacio");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.add_origin("Vacío", &dir.path().join("vacio"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        base.hash_pending(1).unwrap();

        let summaries = base.origin_summaries(&Visible::All).unwrap();
        let o = summaries.iter().find(|s| s.origin.name == "O").unwrap();
        assert_eq!((o.files, o.pending, o.errors, o.links), (3, 1, 0, 1));
        let empty = summaries.iter().find(|s| s.origin.name == "Vacío").unwrap();
        assert_eq!((empty.files, empty.origin.reachable), (0, None));
    }
}
