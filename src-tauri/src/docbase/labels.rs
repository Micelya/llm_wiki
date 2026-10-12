//! Label groups, labels and the folder rules that assign them.
//!
//! A *group* is a question ("which client?") and a *label* an answer
//! ("BGH"); a label can also stand alone. Labels are rows of their own
//! and files point at them, so renaming one touches no file.
//!
//! Labels reach files in two ways. A *rule* sits on a folder and covers
//! everything under it: either "all of this carries that label", or
//! "each subfolder of this one is a … (client, then case)". Rules stay,
//! so a file that shows up later inherits them. On top, the user adds or
//! takes away a label on one file, and that wins over what was
//! inherited.
//!
//! What each file ends up with is kept in `location_label` and worked
//! out again whenever a sweep, a rule or a manual change may alter it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::{now_ms, sql_err, DocBase};

const COLORS: &[&str] = &[
    "#6b7280", "#8b2778", "#2563eb", "#059669", "#d97706", "#dc2626", "#7c3aed", "#0891b2", "#65a30d", "#db2777",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: i64,
    pub group_id: Option<i64>,
    /// Name of its group, when it has one.
    pub group: Option<String>,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelGroup {
    /// `None` for the labels that stand alone.
    pub id: Option<i64>,
    pub name: Option<String>,
    pub labels: Vec<LabelUse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelUse {
    #[serde(flatten)]
    pub label: Label,
    /// Files carrying it.
    pub files: i64,
}

/// A rule as the screens show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelRule {
    pub id: i64,
    pub origin_id: String,
    /// Folder it sits on; empty for the root of the origin.
    pub folder: String,
    /// `fixed`: everything under the folder carries `label`.
    /// `levels`: each subfolder is a label of `groups[0]`, each
    /// subfolder of those one of `groups[1]`, and so on.
    pub kind: String,
    pub label: Option<Label>,
    pub groups: Vec<String>,
}

impl DocBase {
    /// Every group with its labels; the labels that stand alone come
    /// last, under a group without id.
    pub fn label_groups(&self) -> Result<Vec<LabelGroup>, String> {
        let conn = self.conn();
        let mut groups: Vec<LabelGroup> = {
            let mut statement = conn
                .prepare("SELECT id, name FROM label_group ORDER BY name COLLATE NOCASE")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| {
                    Ok(LabelGroup {
                        id: Some(row.get(0)?),
                        name: Some(row.get(1)?),
                        labels: Vec::new(),
                    })
                })
                .map_err(sql_err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?
        };
        let mut loose = LabelGroup {
            id: None,
            name: None,
            labels: Vec::new(),
        };
        let mut statement = conn
            .prepare(&format!(
                "{LABEL_SELECT}, (SELECT count(*) FROM location_label ll WHERE ll.label_id = lb.id)
                 FROM label lb LEFT JOIN label_group g ON g.id = lb.group_id
                 ORDER BY lb.name COLLATE NOCASE"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok(LabelUse {
                    label: read_label(row)?,
                    files: row.get(5)?,
                })
            })
            .map_err(sql_err)?;
        for row in rows {
            let used = row.map_err(sql_err)?;
            match groups.iter_mut().find(|group| group.id == used.label.group_id) {
                Some(group) => group.labels.push(used),
                None => loose.labels.push(used),
            }
        }
        if !loose.labels.is_empty() {
            groups.push(loose);
        }
        Ok(groups)
    }

    /// The label with that name in that group (or standing alone),
    /// creating the group and the label when they do not exist.
    pub fn ensure_label(&self, group: Option<&str>, name: &str) -> Result<Label, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("La etiqueta necesita un nombre".to_string());
        }
        let conn = self.conn();
        let group_id = match group.map(str::trim).filter(|group| !group.is_empty()) {
            Some(group) => Some(ensure_group(&conn, group)?),
            None => None,
        };
        let id = ensure_label_row(&conn, group_id, name)?;
        label_by_id(&conn, id)?.ok_or_else(|| "No se pudo crear la etiqueta".to_string())
    }

    pub fn rename_label(&self, label_id: i64, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("La etiqueta necesita un nombre".to_string());
        }
        // Only what the user reads changes: `key` keeps tying the label
        // to the folder it came from.
        let changed = self
            .conn()
            .execute("UPDATE label SET name = ?1 WHERE id = ?2", params![name, label_id])
            .map_err(sql_err)?;
        if changed == 0 {
            return Err("Esa etiqueta ya no existe".to_string());
        }
        Ok(())
    }

    /// Deletes a label everywhere: from every file and from the rules
    /// that assigned it.
    pub fn delete_label(&self, label_id: i64) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM label WHERE id = ?1", [label_id])
            .map_err(sql_err)?;
        Ok(())
    }

    pub fn rename_group(&self, group_id: i64, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El grupo necesita un nombre".to_string());
        }
        self.conn()
            .execute("UPDATE label_group SET name = ?1 WHERE id = ?2", params![name, group_id])
            .map_err(|e| format!("No se pudo renombrar el grupo (¿ya hay otro con ese nombre?): {e}"))?;
        Ok(())
    }

    /// Deletes a group with all its labels, and the rules built on it.
    pub fn delete_group(&self, group_id: i64) -> Result<(), String> {
        let origins: Vec<String> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare("SELECT id, origin_id, groups FROM label_rule WHERE kind = 'levels'")
                .map_err(sql_err)?;
            let rules = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            let mut touched = Vec::new();
            for (rule_id, origin_id, groups) in rules {
                let groups: Vec<i64> = serde_json::from_str(&groups).unwrap_or_default();
                if groups.contains(&group_id) {
                    conn.execute("DELETE FROM label_rule WHERE id = ?1", [rule_id])
                        .map_err(sql_err)?;
                    touched.push(origin_id);
                }
            }
            conn.execute("DELETE FROM label_group WHERE id = ?1", [group_id])
                .map_err(sql_err)?;
            touched
        };
        for origin_id in origins {
            self.relabel_origin(&origin_id)?;
        }
        Ok(())
    }

    /// The rules of an origin, outermost folder first.
    pub fn label_rules(&self, origin_id: &str) -> Result<Vec<LabelRule>, String> {
        let conn = self.conn();
        let group_names = group_names(&conn)?;
        let mut statement = conn
            .prepare("SELECT id, folder, kind, label_id, groups FROM label_rule WHERE origin_id = ?1 ORDER BY folder, id")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([origin_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        let mut rules = Vec::new();
        for (id, folder, kind, label_id, groups) in rows {
            let groups: Vec<i64> = serde_json::from_str(&groups).unwrap_or_default();
            rules.push(LabelRule {
                id,
                origin_id: origin_id.to_string(),
                folder,
                kind,
                label: match label_id {
                    Some(label_id) => label_by_id(&conn, label_id)?,
                    None => None,
                },
                groups: groups
                    .iter()
                    .filter_map(|group| group_names.get(group).cloned())
                    .collect(),
            });
        }
        Ok(rules)
    }

    /// "Everything under this folder carries this label."
    pub fn add_fixed_rule(&self, origin_id: &str, folder: &str, label_id: i64) -> Result<(), String> {
        let folder = folder.trim_matches('/');
        {
            let conn = self.conn();
            let exists: Option<i64> = conn
                .query_row(
                    "SELECT id FROM label_rule WHERE origin_id = ?1 AND folder = ?2 AND kind = 'fixed' AND label_id = ?3",
                    params![origin_id, folder, label_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_err)?;
            if exists.is_none() {
                conn.execute(
                    "INSERT INTO label_rule (origin_id, folder, kind, label_id, created_at) VALUES (?1, ?2, 'fixed', ?3, ?4)",
                    params![origin_id, folder, label_id, now_ms()],
                )
                .map_err(sql_err)?;
            }
        }
        self.relabel_origin(origin_id)
    }

    /// "Each subfolder of this folder is a … (first group), each
    /// subfolder of those a … (second group)". Replaces the rule of this
    /// kind the folder had.
    pub fn set_levels_rule(&self, origin_id: &str, folder: &str, groups: &[String]) -> Result<(), String> {
        let folder = folder.trim_matches('/');
        {
            let conn = self.conn();
            conn.execute(
                "DELETE FROM label_rule WHERE origin_id = ?1 AND folder = ?2 AND kind = 'levels'",
                params![origin_id, folder],
            )
            .map_err(sql_err)?;
            let mut ids = Vec::new();
            for group in groups.iter().map(|group| group.trim()).filter(|group| !group.is_empty()) {
                ids.push(ensure_group(&conn, group)?);
            }
            if !ids.is_empty() {
                conn.execute(
                    "INSERT INTO label_rule (origin_id, folder, kind, groups, created_at) VALUES (?1, ?2, 'levels', ?3, ?4)",
                    params![origin_id, folder, serde_json::to_string(&ids).unwrap_or_default(), now_ms()],
                )
                .map_err(sql_err)?;
            }
        }
        self.relabel_origin(origin_id)
    }

    pub fn remove_rule(&self, rule_id: i64) -> Result<(), String> {
        let origin_id: Option<String> = {
            let conn = self.conn();
            let origin_id = conn
                .query_row("SELECT origin_id FROM label_rule WHERE id = ?1", [rule_id], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            conn.execute("DELETE FROM label_rule WHERE id = ?1", [rule_id])
                .map_err(sql_err)?;
            origin_id
        };
        match origin_id {
            Some(origin_id) => self.relabel_origin(&origin_id),
            None => Ok(()),
        }
    }

    /// Puts a label on one file, or takes it off, whatever the rules say.
    pub fn set_location_label(&self, location_id: i64, label_id: i64, on: bool) -> Result<(), String> {
        let origin_id: String = {
            let conn = self.conn();
            let origin_id: Option<String> = conn
                .query_row("SELECT origin_id FROM location WHERE id = ?1", [location_id], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            let Some(origin_id) = origin_id else {
                return Err("Ese archivo ya no está en el inventario".to_string());
            };
            conn.execute(
                "INSERT INTO label_manual (location_id, label_id, mode) VALUES (?1, ?2, ?3)
                 ON CONFLICT (location_id, label_id) DO UPDATE SET mode = excluded.mode",
                params![location_id, label_id, if on { "add" } else { "remove" }],
            )
            .map_err(sql_err)?;
            origin_id
        };
        self.relabel_origin(&origin_id)
    }

    /// Works out again which labels each file of an origin carries.
    pub fn relabel_origin(&self, origin_id: &str) -> Result<(), String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;

        struct Rule {
            folder: String,
            label_id: Option<i64>,
            groups: Vec<i64>,
        }
        let rules: Vec<Rule> = {
            let mut statement = tx
                .prepare("SELECT folder, label_id, groups FROM label_rule WHERE origin_id = ?1")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([origin_id], |row| {
                    let groups: String = row.get(2)?;
                    Ok(Rule {
                        folder: row.get(0)?,
                        label_id: row.get(1)?,
                        groups: serde_json::from_str(&groups).unwrap_or_default(),
                    })
                })
                .map_err(sql_err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?
        };
        let locations: Vec<(i64, String)> = {
            let mut statement = tx
                .prepare("SELECT id, folder FROM location WHERE origin_id = ?1")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([origin_id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?
        };

        // Labels by folder: every file of a folder inherits the same.
        let mut by_folder: BTreeMap<&str, BTreeSet<i64>> = BTreeMap::new();
        for (_, folder) in &locations {
            if by_folder.contains_key(folder.as_str()) {
                continue;
            }
            let mut labels = BTreeSet::new();
            for rule in &rules {
                let Some(below) = below(folder, &rule.folder) else {
                    continue;
                };
                if let Some(label_id) = rule.label_id {
                    labels.insert(label_id);
                }
                for (group_id, level) in rule.groups.iter().zip(below.split('/').filter(|part| !part.is_empty())) {
                    labels.insert(ensure_label_row(&tx, Some(*group_id), level)?);
                }
            }
            by_folder.insert(folder.as_str(), labels);
        }

        let mut manual: HashMap<i64, Vec<(i64, bool)>> = HashMap::new();
        {
            let mut statement = tx
                .prepare(
                    "SELECT m.location_id, m.label_id, m.mode FROM label_manual m
                     JOIN location l ON l.id = m.location_id WHERE l.origin_id = ?1",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([origin_id], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (location_id, label_id, mode) = row.map_err(sql_err)?;
                manual.entry(location_id).or_default().push((label_id, mode == "add"));
            }
        }

        tx.execute(
            "DELETE FROM location_label WHERE location_id IN (SELECT id FROM location WHERE origin_id = ?1)",
            [origin_id],
        )
        .map_err(sql_err)?;
        {
            let mut insert = tx
                .prepare("INSERT OR IGNORE INTO location_label (location_id, label_id) VALUES (?1, ?2)")
                .map_err(sql_err)?;
            for (location_id, folder) in &locations {
                let mut labels = by_folder.get(folder.as_str()).cloned().unwrap_or_default();
                for (label_id, add) in manual.get(location_id).map(Vec::as_slice).unwrap_or_default() {
                    if *add {
                        labels.insert(*label_id);
                    } else {
                        labels.remove(label_id);
                    }
                }
                for label_id in labels {
                    insert.execute(params![location_id, label_id]).map_err(sql_err)?;
                }
            }
        }
        tx.commit().map_err(sql_err)
    }
}

const LABEL_SELECT: &str = "SELECT lb.id, lb.group_id, g.name, lb.name, lb.color";

fn read_label(row: &rusqlite::Row<'_>) -> rusqlite::Result<Label> {
    Ok(Label {
        id: row.get(0)?,
        group_id: row.get(1)?,
        group: row.get(2)?,
        name: row.get(3)?,
        color: row.get(4)?,
    })
}

fn label_by_id(conn: &Connection, id: i64) -> Result<Option<Label>, String> {
    conn.query_row(
        &format!("{LABEL_SELECT} FROM label lb LEFT JOIN label_group g ON g.id = lb.group_id WHERE lb.id = ?1"),
        [id],
        read_label,
    )
    .optional()
    .map_err(sql_err)
}

/// The labels of each of these locations, in group and name order.
pub(crate) fn labels_of(conn: &Connection, location_ids: &[i64]) -> Result<HashMap<i64, Vec<Label>>, String> {
    let mut found: HashMap<i64, Vec<Label>> = HashMap::new();
    for chunk in location_ids.chunks(500) {
        let marks = vec!["?"; chunk.len()].join(", ");
        let mut statement = conn
            .prepare(&format!(
                "{LABEL_SELECT}, ll.location_id
                 FROM location_label ll
                 JOIN label lb ON lb.id = ll.label_id
                 LEFT JOIN label_group g ON g.id = lb.group_id
                 WHERE ll.location_id IN ({marks})
                 ORDER BY g.name IS NULL, g.name COLLATE NOCASE, lb.name COLLATE NOCASE"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(chunk.iter()), |row| {
                Ok((row.get::<_, i64>(5)?, read_label(row)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (location_id, label) = row.map_err(sql_err)?;
            found.entry(location_id).or_default().push(label);
        }
    }
    Ok(found)
}

fn group_names(conn: &Connection) -> Result<HashMap<i64, String>, String> {
    let mut statement = conn.prepare("SELECT id, name FROM label_group").map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(sql_err)?;
    rows.collect::<Result<HashMap<_, _>, _>>().map_err(sql_err)
}

fn ensure_group(conn: &Connection, name: &str) -> Result<i64, String> {
    let found: Option<i64> = conn
        .query_row("SELECT id FROM label_group WHERE name = ?1", [name], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    if let Some(id) = found {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO label_group (name, created_at) VALUES (?1, ?2)",
        params![name, now_ms()],
    )
    .map_err(sql_err)?;
    Ok(conn.last_insert_rowid())
}

/// The label a folder name or a typed name stands for, created the
/// first time. It is found by `key`, so a renamed label stays the same
/// label for the folder it came from.
fn ensure_label_row(conn: &Connection, group_id: Option<i64>, key: &str) -> Result<i64, String> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT id FROM label WHERE coalesce(group_id, 0) = ?1 AND key = ?2",
            params![group_id.unwrap_or(0), key],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    if let Some(id) = found {
        return Ok(id);
    }
    let count: i64 = conn
        .query_row("SELECT count(*) FROM label", [], |row| row.get(0))
        .map_err(sql_err)?;
    conn.execute(
        "INSERT INTO label (group_id, name, key, color, created_at) VALUES (?1, ?2, ?2, ?3, ?4)",
        params![group_id, key, COLORS[(count as usize) % COLORS.len()], now_ms()],
    )
    .map_err(sql_err)?;
    Ok(conn.last_insert_rowid())
}

/// What is left of `folder` below `rule_folder`: empty when they are the
/// same folder, `None` when `folder` is not under it.
fn below<'a>(folder: &'a str, rule_folder: &str) -> Option<&'a str> {
    if rule_folder.is_empty() {
        Some(folder)
    } else if folder == rule_folder {
        Some("")
    } else if folder.len() > rule_folder.len()
        && folder.starts_with(rule_folder)
        && folder.as_bytes()[rule_folder.len()] == b'/'
    {
        Some(&folder[rule_folder.len() + 1..])
    } else {
        None
    }
}

/// Turns the folder levels an origin was given under the first design
/// (`origin.convention`) into a rule on its root.
pub(crate) fn adopt_conventions(base: &DocBase) -> Result<(), String> {
    let pending: Vec<(String, Vec<String>)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT id, convention FROM origin WHERE convention != '[]'")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(sql_err)?;
        rows.filter_map(Result::ok)
            .map(|(id, convention)| (id, serde_json::from_str(&convention).unwrap_or_default()))
            .collect()
    };
    for (origin_id, levels) in pending {
        base.set_levels_rule(&origin_id, "", &levels)?;
        base.conn()
            .execute("UPDATE origin SET convention = '[]' WHERE id = ?1", [&origin_id])
            .map_err(sql_err)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::query::DocumentFilter;
    use crate::docbase::testing::TempDir;
    use crate::docbase::Visible;

    fn sample() -> (TempDir, DocBase, String) {
        let dir = TempDir::new();
        dir.write("o/BGH/Reclamo facturas/carta.md", b"carta");
        dir.write("o/BGH/Contrato/contrato.md", b"contrato");
        dir.write("o/BGH/suelto del cliente.md", b"suelto");
        dir.write("o/Sovra/Acuerdo/acuerdo.md", b"acuerdo");
        dir.write("o/Varios/nota.md", b"nota");
        dir.write("o/en la raiz.md", b"raiz");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        (dir, base, origin.id)
    }

    fn labels_of_file(base: &DocBase, name: &str) -> Vec<String> {
        base.documents(&Visible::All, &DocumentFilter::default())
            .unwrap()
            .rows
            .into_iter()
            .find(|row| row.name == name)
            .unwrap()
            .labels
            .into_iter()
            .map(|label| match label.group {
                Some(group) => format!("{group}: {}", label.name),
                None => label.name,
            })
            .collect()
    }

    #[test]
    fn each_subfolder_becomes_a_label_of_the_group_level_by_level() {
        let (_dir, base, origin) = sample();
        base.set_levels_rule(&origin, "", &["Cliente".into(), "Caso".into()]).unwrap();

        assert_eq!(labels_of_file(&base, "carta.md"), ["Caso: Reclamo facturas", "Cliente: BGH"]);
        // Left at the client level: a client and no case.
        assert_eq!(labels_of_file(&base, "suelto del cliente.md"), ["Cliente: BGH"]);
        assert!(labels_of_file(&base, "en la raiz.md").is_empty());
    }

    #[test]
    fn rules_can_sit_on_any_folder_and_differ_between_branches() {
        let (_dir, base, origin) = sample();
        // Only the first level is clients everywhere…
        base.set_levels_rule(&origin, "", &["Cliente".into()]).unwrap();
        // …and only BGH is ordered by case underneath.
        base.set_levels_rule(&origin, "BGH", &["Caso".into()]).unwrap();
        let urgent = base.ensure_label(None, "urgente").unwrap();
        base.add_fixed_rule(&origin, "Sovra", urgent.id).unwrap();

        assert_eq!(labels_of_file(&base, "contrato.md"), ["Caso: Contrato", "Cliente: BGH"]);
        assert_eq!(labels_of_file(&base, "acuerdo.md"), ["Cliente: Sovra", "urgente"]);
        assert_eq!(labels_of_file(&base, "nota.md"), ["Cliente: Varios"]);
    }

    #[test]
    fn a_file_that_appears_later_inherits_the_rules() {
        let (dir, base, origin) = sample();
        base.set_levels_rule(&origin, "", &["Cliente".into()]).unwrap();
        dir.write("o/Nuevo cliente/poder.md", b"poder");
        base.scan_origin(&origin).unwrap();

        assert_eq!(labels_of_file(&base, "poder.md"), ["Cliente: Nuevo cliente"]);
    }

    #[test]
    fn a_label_put_or_removed_by_hand_wins_over_what_was_inherited() {
        let (_dir, base, origin) = sample();
        base.set_levels_rule(&origin, "", &["Cliente".into()]).unwrap();
        let page = base.documents(&Visible::All, &DocumentFilter::default()).unwrap();
        let nota = page.rows.iter().find(|row| row.name == "nota.md").unwrap();
        let varios = nota.labels[0].id;
        let sovra = base.ensure_label(Some("Cliente"), "Sovra").unwrap();

        // "Varios" is not a client: this note belongs to Sovra.
        base.set_location_label(nota.location_id, varios, false).unwrap();
        base.set_location_label(nota.location_id, sovra.id, true).unwrap();
        assert_eq!(labels_of_file(&base, "nota.md"), ["Cliente: Sovra"]);

        // The correction survives a sweep.
        base.scan_origin(&origin).unwrap();
        assert_eq!(labels_of_file(&base, "nota.md"), ["Cliente: Sovra"]);
    }

    #[test]
    fn renaming_a_label_changes_it_everywhere_and_keeps_it_tied_to_its_folder() {
        let (dir, base, origin) = sample();
        base.set_levels_rule(&origin, "", &["Cliente".into()]).unwrap();
        let bgh = base.ensure_label(Some("Cliente"), "BGH").unwrap();
        base.rename_label(bgh.id, "BGH S.A.").unwrap();
        assert_eq!(labels_of_file(&base, "contrato.md"), ["Cliente: BGH S.A."]);

        // A new file in the same folder gets the renamed label, not a new one.
        dir.write("o/BGH/otro.md", b"otro");
        base.scan_origin(&origin).unwrap();
        assert_eq!(labels_of_file(&base, "otro.md"), ["Cliente: BGH S.A."]);
        let groups = base.label_groups().unwrap();
        let clients = groups.iter().find(|group| group.name.as_deref() == Some("Cliente")).unwrap();
        assert_eq!(clients.labels.len(), 3);
        assert_eq!(clients.labels.iter().find(|l| l.label.id == bgh.id).unwrap().files, 4);
    }

    #[test]
    fn documents_can_be_filtered_by_label() {
        let (_dir, base, origin) = sample();
        base.set_levels_rule(&origin, "", &["Cliente".into()]).unwrap();
        let sovra = base.ensure_label(Some("Cliente"), "Sovra").unwrap();
        let filter = DocumentFilter { label_id: Some(sovra.id), ..Default::default() };
        let page = base.documents(&Visible::All, &filter).unwrap();
        assert_eq!(page.rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(), ["acuerdo.md"]);
    }

    #[test]
    fn deleting_labels_groups_and_rules_takes_them_off_the_files() {
        let (_dir, base, origin) = sample();
        base.set_levels_rule(&origin, "", &["Cliente".into()]).unwrap();
        let urgent = base.ensure_label(None, "urgente").unwrap();
        base.add_fixed_rule(&origin, "BGH", urgent.id).unwrap();
        assert_eq!(labels_of_file(&base, "contrato.md"), ["Cliente: BGH", "urgente"]);
        assert_eq!(base.label_rules(&origin).unwrap().len(), 2);

        base.delete_label(urgent.id).unwrap();
        assert_eq!(labels_of_file(&base, "contrato.md"), ["Cliente: BGH"]);
        assert_eq!(base.label_rules(&origin).unwrap().len(), 1);

        let group = base.label_groups().unwrap()[0].id.unwrap();
        base.delete_group(group).unwrap();
        assert!(labels_of_file(&base, "contrato.md").is_empty());
        assert!(base.label_rules(&origin).unwrap().is_empty());
        assert!(base.label_groups().unwrap().is_empty());
    }

    #[test]
    fn folder_levels_given_when_incorporating_become_a_rule_on_the_root() {
        let dir = TempDir::new();
        dir.write("o/BGH/Reclamo/carta.md", b"carta");
        let base = DocBase::in_memory().unwrap();
        let origin = base
            .add_origin("O", &dir.path().join("o"), &["cliente".into(), "caso".into()])
            .unwrap();
        base.scan_origin(&origin.id).unwrap();

        assert_eq!(labels_of_file(&base, "carta.md"), ["caso: Reclamo", "cliente: BGH"]);
        let rules = base.label_rules(&origin.id).unwrap();
        assert_eq!((rules[0].kind.as_str(), rules[0].folder.as_str()), ("levels", ""));
        assert_eq!(rules[0].groups, ["cliente", "caso"]);
    }
}
