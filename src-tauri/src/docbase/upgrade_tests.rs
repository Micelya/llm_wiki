//! An update keeps the data of every earlier version (D-496, P-297): bases
//! as each version of the app left them, with invented rows in every table
//! they have, are brought up to the current version through the app's own
//! door (`DocBase::open_in_data_folder`), and nothing is lost; the backup is
//! made first and matches its fingerprints.
//!
//! Nothing here names a version or a table but the decisions of a person:
//! the invented rows are written from what each base says of itself (its
//! columns, their kinds, the values its checks admit, the rows its foreign
//! keys need), so a new version is tested as it is added.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OpenFlags};

use super::*;
use crate::docbase::schema::{self, CURRENT_VERSION};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, PRIVATE_DB_FILE};

/// Rows the invented base tries to have in each table (at least one).
const ROWS: usize = 3;

/// What a person decided or made, the most valuable of a base: an update
/// keeps these tables row by row, no more and no less.
const PERSON_DECISIONS: &[&str] = &[
    "origin",
    "exclusion",
    "label_group",
    "label",
    "label_rule",
    "label_manual",
    "type_decision",
    "similar_pair",
    "knowledge_decision",
    "knowledge_matter",
    "knowledge_matter_choice",
    "knowledge_matter_record",
    "knowledge_matter_question",
    "knowledge_note",
    "knowledge_query",
    "knowledge_research",
    "template_naming",
    "template_field_choice",
    "recognition_request",
    "generated_document",
    "operation_log",
    "local_program",
];

/// The data an update changes on purpose, for the version that does it to
/// say why: (version, table, why). The test leaves those rows alone. None
/// so far: every update only adds.
const CHANGED_ON_PURPOSE: &[(i64, &str, &str)] = &[];

/// The tables and columns an update renamed, (version, before, after): their
/// rows are looked for under the new name.
const RENAMED_TABLES: &[(i64, &str, &str)] = &[(3, "content", "version")];
const RENAMED_COLUMNS: &[(i64, &str, &str)] = &[(3, "content_id", "version_id")];

/// The name a table and its columns of a base at `version` have now.
fn named_now(version: i64, table: &str, names: &[String]) -> (String, Vec<String>) {
    let now = |renames: &[(i64, &str, &str)], name: &str| {
        renames
            .iter()
            .filter(|(v, _, _)| *v > version)
            .fold(name.to_string(), |name, (_, before, after)| if name == *before { after.to_string() } else { name })
    };
    (now(RENAMED_TABLES, table), names.iter().map(|n| now(RENAMED_COLUMNS, n)).collect())
}

struct Column {
    name: String,
    decl: String,
    nullable: bool,
    default: Option<String>,
}

struct ForeignKey {
    parent: String,
    from: Vec<String>,
    to: Vec<String>,
}

struct Attempt {
    defaults: bool,
    null: Option<usize>,
    all_null: bool,
}

/// The tables with data, and whether each is virtual (a text index).
fn data_tables(conn: &Connection) -> Vec<(String, bool)> {
    let mut statement = conn
        .prepare(
            "SELECT name, type FROM pragma_table_list
             WHERE schema = 'main' AND type IN ('table', 'virtual') AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\'
             ORDER BY name",
        )
        .unwrap();
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)? == "virtual"))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn columns(conn: &Connection, table: &str) -> Vec<Column> {
    let mut statement = conn.prepare("SELECT name, type, \"notnull\", dflt_value, pk FROM pragma_table_info(?1) ORDER BY cid").unwrap();
    let rows = statement
        .query_map([table], |row| {
            let not_null: bool = row.get(2)?;
            let pk: i64 = row.get(4)?;
            Ok(Column { name: row.get(0)?, decl: row.get(1)?, nullable: !not_null && pk == 0, default: row.get(3)? })
        })
        .unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn primary_key(conn: &Connection, table: &str) -> Vec<String> {
    let mut statement = conn.prepare("SELECT name FROM pragma_table_info(?1) WHERE pk > 0 ORDER BY pk").unwrap();
    let rows = statement.query_map([table], |row| row.get(0)).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn foreign_keys(conn: &Connection, table: &str) -> Vec<ForeignKey> {
    let mut statement =
        conn.prepare("SELECT id, \"table\", \"from\", \"to\" FROM pragma_foreign_key_list(?1) ORDER BY id, seq").unwrap();
    let rows: Vec<(i64, String, String, Option<String>)> = statement
        .query_map([table], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut keys: Vec<(i64, ForeignKey)> = Vec::new();
    for (id, parent, from, to) in rows {
        if keys.last().map(|(last, _)| *last) != Some(id) {
            keys.push((id, ForeignKey { parent: parent.clone(), from: Vec::new(), to: Vec::new() }));
        }
        let key = &mut keys.last_mut().unwrap().1;
        key.from.push(from);
        if let Some(to) = to {
            key.to.push(to);
        }
    }
    keys.into_iter()
        .map(|(_, mut key)| {
            if key.to.is_empty() {
                key.to = primary_key(conn, &key.parent);
            }
            key
        })
        .collect()
}

/// The values a check of the table admits for a column (`CHECK (c IN (…))`).
fn admitted(sql: &str, column: &str) -> Option<Vec<String>> {
    let flat = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let start = format!("CHECK ({column} IN (");
    let at = flat.find(&start)? + start.len();
    let end = flat[at..].find(')')? + at;
    Some(flat[at..end].split(',').map(|value| value.trim().trim_matches('\'').to_string()).collect())
}

fn text_default(column: &Column) -> Option<String> {
    let default = column.default.as_deref()?;
    let inner = default.strip_prefix('\'')?.strip_suffix('\'')?;
    Some(inner.replace("''", "'"))
}

fn invented(table: &str, column: &Column, row: usize, admitted: Option<&Vec<String>>, defaults: bool) -> Value {
    if let Some(values) = admitted {
        return Value::Text(values[row % values.len()].clone());
    }
    if defaults {
        if let Some(text) = text_default(column) {
            return Value::Text(text);
        }
    }
    let decl = column.decl.to_uppercase();
    if decl.contains("INT") {
        Value::Integer(row as i64 + 1)
    } else if decl.contains("REAL") || decl.contains("FLOA") || decl.contains("DOUB") {
        Value::Real(row as f64 + 0.5)
    } else if decl.contains("BLOB") {
        Value::Blob(vec![1, 2, row as u8])
    } else {
        Value::Text(format!("{table}.{}-{}", column.name, row + 1))
    }
}

fn plain_rows(conn: &Connection, table: &str, columns: &[String]) -> Vec<Vec<Value>> {
    let list = columns.iter().map(|c| quoted(c)).collect::<Vec<_>>().join(", ");
    let mut statement = conn.prepare(&format!("SELECT {list} FROM {} ORDER BY {list}", quoted(table))).unwrap();
    let rows = statement
        .query_map([], |row| (0..columns.len()).map(|i| row.get::<_, Value>(i)).collect::<Result<Vec<_>, _>>())
        .unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// Writes invented rows in every table of the base, parents first; each
/// table gets at least one, or the test says which table could not.
fn fill(conn: &Connection) {
    let tables: Vec<String> = data_tables(conn).into_iter().map(|(name, _)| name).collect();
    let keys: BTreeMap<String, Vec<ForeignKey>> = tables.iter().map(|t| (t.clone(), foreign_keys(conn, t))).collect();
    let mut done: BTreeSet<String> = BTreeSet::new();
    while done.len() < tables.len() {
        let ready: Vec<String> = tables
            .iter()
            .filter(|t| !done.contains(*t))
            .filter(|t| keys[*t].iter().all(|k| &k.parent == *t || done.contains(&k.parent) || !tables.contains(&k.parent)))
            .cloned()
            .collect();
        assert!(!ready.is_empty(), "the foreign keys of these tables go round in a circle: {:?}", tables.iter().filter(|t| !done.contains(*t)).collect::<Vec<_>>());
        for table in ready {
            fill_table(conn, &table, &keys[&table]);
            done.insert(table);
        }
    }
}

fn fill_table(conn: &Connection, table: &str, keys: &[ForeignKey]) {
    let sql: String = conn.query_row("SELECT sql FROM sqlite_master WHERE name = ?1", [table], |row| row.get(0)).unwrap();
    let columns = columns(conn, table);
    let checks: Vec<Option<Vec<String>>> = columns.iter().map(|c| admitted(&sql, &c.name)).collect();
    let mut attempts = vec![
        Attempt { defaults: true, null: None, all_null: false },
        Attempt { defaults: false, null: None, all_null: false },
    ];
    for (n, column) in columns.iter().enumerate() {
        if column.nullable {
            attempts.push(Attempt { defaults: true, null: Some(n), all_null: false });
        }
    }
    attempts.push(Attempt { defaults: true, null: None, all_null: true });
    // In the order of their columns (SQLite lists them the other way round),
    // so that a pair (`a < b`) gets its documents in order.
    let position = |name: &str| columns.iter().position(|c| c.name == name).unwrap();
    let mut keys: Vec<&ForeignKey> = keys.iter().collect();
    keys.sort_by_key(|key| position(&key.from[0]));
    let names = columns.iter().map(|c| quoted(&c.name)).collect::<Vec<_>>().join(", ");
    let marks = vec!["?"; columns.len()].join(", ");
    let insert = format!("INSERT INTO {} ({names}) VALUES ({marks})", quoted(table));
    for row in 0..ROWS {
        let mut last_error = String::new();
        let mut written = false;
        for attempt in &attempts {
            let mut values: Vec<Value> = columns
                .iter()
                .zip(&checks)
                .map(|(column, check)| invented(table, column, row, check.as_ref(), attempt.defaults))
                .collect();
            // The rows a foreign key needs: the k-th key to the same parent
            // takes another of its rows (a pair of two documents).
            let mut per_parent: BTreeMap<&str, usize> = BTreeMap::new();
            for key in &keys {
                let k = per_parent.entry(key.parent.as_str()).or_insert(0);
                let parents = plain_rows(conn, &key.parent, &key.to);
                let chosen = if key.parent == table {
                    parents.first()
                } else {
                    Some(&parents[(row + *k) % parents.len()])
                };
                *k += 1;
                for (j, from) in key.from.iter().enumerate() {
                    values[position(from)] = chosen.map(|parent| parent[j].clone()).unwrap_or(Value::Null);
                }
            }
            if let Some(n) = attempt.null {
                values[n] = Value::Null;
            }
            if attempt.all_null {
                for (n, column) in columns.iter().enumerate() {
                    if column.nullable {
                        values[n] = Value::Null;
                    }
                }
            }
            match conn.execute(&insert, params_from_iter(values.iter())) {
                Ok(_) => {
                    written = true;
                    break;
                }
                Err(error) => last_error = error.to_string(),
            }
        }
        assert!(
            written || row > 0,
            "no invented row could be written in the table {table} ({last_error}): teach `fill` the kind of value it asks for"
        );
    }
}

/// Everything a base holds, table by table: its columns and each row (a
/// text index also with the rowid, which is the id of its document).
fn snapshot(conn: &Connection) -> BTreeMap<String, (Vec<String>, Vec<String>)> {
    data_tables(conn)
        .into_iter()
        .map(|(table, is_virtual)| {
            let mut names: Vec<String> = columns(conn, &table).into_iter().map(|c| c.name).collect();
            if is_virtual {
                names.insert(0, "rowid".to_string());
            }
            let rows = rows_of(conn, &table, &names);
            (table, (names, rows))
        })
        .collect()
}

/// The rows of a table in these columns, each as text, sorted.
fn rows_of(conn: &Connection, table: &str, names: &[String]) -> Vec<String> {
    let list = names.iter().map(|c| if c == "rowid" { c.clone() } else { quoted(c) }).collect::<Vec<_>>().join(", ");
    let mut statement = conn
        .prepare(&format!("SELECT {list} FROM {}", quoted(table)))
        .unwrap_or_else(|e| panic!("the table {table} lost something of {names:?}: {e}"));
    let rows = statement
        .query_map([], |row| (0..names.len()).map(|i| row.get::<_, Value>(i)).collect::<Result<Vec<_>, _>>())
        .unwrap();
    let mut rows: Vec<String> = rows.map(|row| format!("{:?}", row.unwrap())).collect();
    rows.sort();
    rows
}

/// What is in `before` and not in `after` (rows counted).
fn missing(before: &[String], after: &[String]) -> Vec<String> {
    let mut left: BTreeMap<&String, usize> = BTreeMap::new();
    for row in after {
        *left.entry(row).or_default() += 1;
    }
    before
        .iter()
        .filter(|row| match left.get_mut(row) {
            Some(n) if *n > 0 => {
                *n -= 1;
                false
            }
            _ => true,
        })
        .cloned()
        .collect()
}

/// A base at `version` in the data folder `data`, as that version of the
/// app left it, with invented rows in every table. Returns what it holds.
fn old_base(data: &std::path::Path, version: i64) -> BTreeMap<String, (Vec<String>, Vec<String>)> {
    let path = data.join(PRIVATE_DB_FILE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut conn = Connection::open(&path).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    // Only to write the invented base quickly; the app's door opens it as
    // it opens any other.
    conn.pragma_update(None, "synchronous", "OFF").unwrap();
    schema::build_at(&mut conn, version);
    conn.execute_batch("BEGIN").unwrap();
    fill(&conn);
    conn.execute_batch("COMMIT").unwrap();
    snapshot(&conn)
}

fn version_on_disk(data: &std::path::Path) -> i64 {
    let conn = Connection::open_with_flags(data.join(PRIVATE_DB_FILE), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    schema::version(&conn).unwrap()
}

fn prints_of(folder: &std::path::Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(folder.join(PRINTS))
        .unwrap()
        .lines()
        .map(|line| {
            let (hash, path) = line.split_once("  ").unwrap();
            (path.to_string(), hash.to_string())
        })
        .collect()
}

fn folders_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

/// Every earlier version, from the first one: brought up to the current one
/// by opening it, it keeps every row of every table (the decisions of a
/// person, exactly), and the backup made before holds the base as it was,
/// with its fingerprints.
#[test]
fn every_earlier_version_is_brought_up_to_date_without_losing_anything_and_with_its_backup() {
    assert!(CURRENT_VERSION > 1);
    // One version per thread, each in its own folder: what fails says its
    // version.
    std::thread::scope(|scope| {
        for version in 1..CURRENT_VERSION {
            scope.spawn(move || bring_up(version));
        }
    });
}

fn bring_up(version: i64) {
    let dir = TempDir::new();
    let data = dir.path();
    let before = old_base(data, version);
    for table in PERSON_DECISIONS.iter().filter(|t| before.contains_key(**t)) {
        assert!(!before[*table].1.is_empty(), "version {version}: no invented decision in {table}");
    }
    // The other data of the person, invented.
    dir.write("app-state.json", br#"{"outputLanguage":"es","uiLanguage":"es"}"#);
    dir.write("chat/conversations.json", br#"[{"id":"c1","title":"Consulta inventada"}]"#);
    dir.write("chat/chats/c1.json", br#"[{"role":"user","content":"Hola"}]"#);

    let (base, opening) = DocBase::open_in_data_folder(data);
    let base = base.unwrap_or_else(|e| panic!("version {version}: the update failed: {e}"));
    assert_eq!(opening.found_version, Some(version));
    assert!(opening.problem.is_none() && !opening.newer && opening.pruned.is_empty());
    let backup = opening.backup.clone().unwrap_or_else(|| panic!("version {version}: no backup was made"));
    assert!(backup.starts_with(&format!("antes-de-v{CURRENT_VERSION}-")), "{backup}");

    {
        let conn = base.conn();
        assert_eq!(schema::version(&conn).unwrap(), CURRENT_VERSION);
        for (table, (names, rows)) in &before {
            if CHANGED_ON_PURPOSE.iter().any(|(v, t, _)| *v > version && *t == table.as_str()) {
                continue;
            }
            let (now_table, now_names) = named_now(version, table, names);
            let kept = rows_of(&conn, &now_table, &now_names);
            let lost = missing(rows, &kept);
            assert!(lost.is_empty(), "version {version}: the table {table} lost or changed {lost:?}");
            if PERSON_DECISIONS.contains(&table.as_str()) {
                assert_eq!(kept.len(), rows.len(), "version {version}: the decisions of {table} are not the same");
            }
        }
        let broken: i64 = conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| row.get(0)).unwrap();
        assert_eq!(broken, 0, "version {version}: rows point to rows that are not there");
        let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0)).unwrap();
        assert_eq!(check, "ok", "version {version}");
    }
    drop(base);

    // The backup: complete, with its fingerprints, and the base as it was.
    let folder = data.join(BACKUPS_DIR).join(&backup);
    let manifest = check(&folder).unwrap_or_else(|e| panic!("version {version}: {e}"));
    assert_eq!((manifest.base_version, manifest.for_version), (version, CURRENT_VERSION));
    let prints = prints_of(&folder);
    for rel in ["app-state.json", "chat/conversations.json", "chat/chats/c1.json"] {
        assert_eq!(prints.get(rel), Some(&sha256_of(&data.join(rel)).unwrap()), "version {version}: {rel}");
    }
    assert_eq!(prints.get(PRIVATE_DB_FILE), Some(&sha256_of(&folder.join(PRIVATE_DB_FILE)).unwrap()));
    let kept = Connection::open_with_flags(folder.join(PRIVATE_DB_FILE), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(schema::version(&kept).unwrap(), version);
    assert!(snapshot(&kept) == before, "version {version}: the backup is not the base as it was");
    assert_eq!(list(data).len(), 1);
    assert!(list(data)[0].complete);
}

/// Without a backup the base is not changed: it stays at its version, and
/// the person reads what happened, that the data are intact and what to do.
#[test]
fn without_a_backup_the_base_is_not_changed() {
    let dir = TempDir::new();
    let data = dir.path();
    let version = CURRENT_VERSION - 1;
    let before = old_base(data, version);
    // Where the backups go there is a file: the folder cannot be made.
    dir.write(BACKUPS_DIR, b"no es una carpeta");
    let (base, opening) = DocBase::open_in_data_folder(data);
    let problem = base.unwrap_err();
    assert_eq!(opening.problem.as_deref(), Some(problem.as_str()));
    assert!(opening.backup.is_none() && !opening.newer);
    assert!(problem.contains("No se cambió nada") && problem.contains("intactos") && problem.contains("Qué hacer"), "{problem}");
    assert_eq!(version_on_disk(data), version);
    let conn = Connection::open_with_flags(data.join(PRIVATE_DB_FILE), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert!(snapshot(&conn) == before);
}

/// A new base, and a base already at the current version, need no backup.
#[test]
fn a_new_base_and_one_up_to_date_get_no_backup() {
    let dir = TempDir::new();
    let data = dir.path();
    for found in [0, CURRENT_VERSION] {
        let (base, opening) = DocBase::open_in_data_folder(data);
        let base = base.unwrap();
        assert_eq!(opening.found_version, Some(found));
        assert!(opening.backup.is_none());
        assert_eq!(base.schema_version().unwrap(), CURRENT_VERSION);
    }
    assert!(!data.join(BACKUPS_DIR).exists());
}

/// The last backups are kept; the older ones go, never one that is not the
/// app's, and one cut half-way is not taken for a backup.
#[test]
fn the_last_backups_are_kept_and_only_the_apps_own_are_deleted() {
    let dir = TempDir::new();
    let data = dir.path();
    dir.write("respaldos/mi-copia/private.sqlite3", b"una copia hecha a mano");
    dir.write("respaldos/antes-de-v3-a-mano/private.sqlite3", b"otra, sin respaldo.json");
    dir.write("respaldos/sin-terminar-antes-de-v9-2020-01-01-0000/docbase/private.sqlite3", b"cortado");
    let mut made = Vec::new();
    for round in 0..(KEEP + 2) {
        for file in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{file}", data.join(PRIVATE_DB_FILE).display()));
        }
        old_base(data, CURRENT_VERSION - 1);
        {
            // The version of the app that wrote it, as the app notes it.
            let conn = Connection::open(data.join(PRIVATE_DB_FILE)).unwrap();
            conn.execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES ('app-version', ?1)", [format!("0.0.{round}")])
                .unwrap();
        }
        let (base, opening) = DocBase::open_in_data_folder(data);
        base.unwrap();
        made.push(opening.backup.clone().unwrap());
        let expected_pruned = if round >= KEEP { vec![made[round - KEEP].clone()] } else { Vec::new() };
        assert_eq!(opening.pruned, expected_pruned, "round {round}");
    }
    let kept: Vec<String> = list(data).into_iter().map(|b| b.name).collect();
    let newest: Vec<String> = made.iter().rev().take(KEEP).cloned().collect();
    assert_eq!(kept, newest);
    let summaries = list(data);
    assert_eq!(summaries[0].written_by_app.as_deref(), Some(format!("0.0.{}", KEEP + 1).as_str()));
    assert!(summaries.iter().all(|b| b.complete && b.base_version == CURRENT_VERSION - 1));
    let folders = folders_in(&data.join(BACKUPS_DIR));
    assert!(folders.contains(&"mi-copia".to_string()) && folders.contains(&"antes-de-v3-a-mano".to_string()));
    assert!(!folders.iter().any(|f| f.starts_with("sin-terminar-")), "{folders:?}");
    // And the only one is never deleted.
    assert_eq!(prune(data, 0).len(), KEEP - 1);
    assert_eq!(list(data).len(), 1);
    // The base now says which version of the app opened it.
    let conn = Connection::open(data.join(PRIVATE_DB_FILE)).unwrap();
    let noted: String = conn.query_row("SELECT value FROM docbase_setting WHERE key = 'app-version'", [], |row| row.get(0)).unwrap();
    assert_eq!(noted, env!("CARGO_PKG_VERSION"));
}

/// When the update fails after the backup, the backup stays (no older one
/// is deleted) and the person reads its name.
#[test]
fn a_failed_update_keeps_its_backup_and_names_it() {
    let dir = TempDir::new();
    let data = dir.path();
    let before = CURRENT_VERSION - 1;
    old_base(data, before);
    {
        // A table the next version adds is already there: its change fails.
        let conn = Connection::open(data.join(PRIVATE_DB_FILE)).unwrap();
        conn.execute_batch("CREATE TABLE appearance (x)").unwrap();
    }
    let (base, opening) = DocBase::open_in_data_folder(data);
    let problem = base.unwrap_err();
    let backup = opening.backup.clone().unwrap();
    assert!(problem.contains(&backup) && problem.contains("no borres nada"), "{problem}");
    assert_eq!(version_on_disk(data), before);
    assert_eq!(check(&data.join(BACKUPS_DIR).join(&backup)).unwrap().base_version, before);
}

/// The person went back to the version before: the base, written by a newer
/// version, is not touched; going back to the backup sets it apart (never
/// deleted) and the backup's opens with everything it had.
#[test]
fn a_base_of_a_newer_version_is_left_alone_and_the_backup_takes_its_place() {
    let dir = TempDir::new();
    let data = dir.path();
    let version = CURRENT_VERSION - 1;
    let before = old_base(data, version);
    let (base, opening) = DocBase::open_in_data_folder(data);
    drop(base.unwrap());
    let backup = opening.backup.unwrap();
    {
        // What a newer version of the app would leave.
        let conn = Connection::open(data.join(PRIVATE_DB_FILE)).unwrap();
        conn.pragma_update(None, "user_version", CURRENT_VERSION + 1).unwrap();
    }
    let print = sha256_of(&data.join(PRIVATE_DB_FILE)).unwrap();
    let (base, opening) = DocBase::open_in_data_folder(data);
    let problem = base.unwrap_err();
    assert!(opening.newer && opening.backup.is_none());
    assert!(problem.contains("más nueva") && problem.contains("No se tocó nada") && problem.contains("Datos y respaldos"), "{problem}");
    assert_eq!(sha256_of(&data.join(PRIVATE_DB_FILE)).unwrap(), print, "the base was touched");

    // A name that is not a backup, or a backup that changed, is refused,
    // and the base stays where it is.
    assert!(restore(data, "../docbase", CURRENT_VERSION).is_err());
    assert!(restore(data, "mi-copia", CURRENT_VERSION).is_err());
    let copy_of_backup = data.join(BACKUPS_DIR).join(&backup).join(PRIVATE_DB_FILE);
    let good = std::fs::read(&copy_of_backup).unwrap();
    let mut bad = good.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    std::fs::write(&copy_of_backup, &bad).unwrap();
    let refused = restore(data, &backup, CURRENT_VERSION).unwrap_err();
    assert!(refused.contains("huella"), "{refused}");
    assert_eq!(sha256_of(&data.join(PRIVATE_DB_FILE)).unwrap(), print);
    std::fs::write(&copy_of_backup, &good).unwrap();
    // Nor one this version cannot open.
    assert!(restore(data, &backup, version - 1).unwrap_err().contains("llega hasta"));

    let aside = restore(data, &backup, CURRENT_VERSION).unwrap();
    assert!(aside.starts_with("apartada-"));
    let set_apart = data.join(BACKUPS_DIR).join(&aside).join(PRIVATE_DB_FILE);
    assert_eq!(sha256_of(&set_apart).unwrap(), print, "the base in use is set apart as it was");
    assert!(data.join(BACKUPS_DIR).join(&aside).join(README).exists());
    assert_eq!(version_on_disk(data), version);

    // Opened again, it is brought up as any base of that version.
    let (base, opening) = DocBase::open_in_data_folder(data);
    let base = base.unwrap();
    assert_eq!(opening.found_version, Some(version));
    assert!(opening.backup.is_some());
    let conn = base.conn();
    for (table, (names, rows)) in &before {
        let (now_table, now_names) = named_now(version, table, names);
        assert!(missing(rows, &rows_of(&conn, &now_table, &now_names)).is_empty(), "{table}");
    }
    assert_eq!(list(data).len(), 2);
}

#[test]
fn the_values_a_check_admits_are_read_from_the_table() {
    let sql = "CREATE TABLE t (kind TEXT NOT NULL CHECK (kind IN ('private', 'shared')), hash_state TEXT,\n state  TEXT CHECK (state IN ('a')))";
    assert_eq!(admitted(sql, "kind"), Some(vec!["private".to_string(), "shared".to_string()]));
    assert_eq!(admitted(sql, "state"), Some(vec!["a".to_string()]));
    assert_eq!(admitted(sql, "hash_state"), None);
}

/// The backups of the base of the earlier numbering (another file, D-548)
/// are left on disk as they are: not listed, never deleted, and going
/// back to one is refused before anything is touched.
#[test]
fn a_backup_of_the_earlier_base_is_left_alone() {
    let dir = TempDir::new();
    let data = dir.path();
    let old = "respaldos/antes-de-v29-2026-01-01-0000";
    let body = b"una base de la numeracion anterior";
    dir.write(&format!("{old}/docbase/private.sqlite3"), body);
    let print = sha256_of(&data.join(old).join("docbase/private.sqlite3")).unwrap();
    dir.write(&format!("{old}/huellas.sha256"), format!("{print}  docbase/private.sqlite3\n").as_bytes());
    let manifest = Manifest {
        kind: KIND.to_string(),
        format: 1,
        base_version: 28,
        for_version: 29,
        written_by_app: None,
        made_by_app: "0.0.1".to_string(),
        made_at_ms: 1,
        made_at: "01/01/2026 00:00".to_string(),
        files: vec![KeptFile { path: "docbase/private.sqlite3".to_string(), bytes: body.len() as u64, sha256: print }],
    };
    dir.write(&format!("{old}/respaldo.json"), &serde_json::to_vec(&manifest).unwrap());
    old_base(data, CURRENT_VERSION - 1);
    let live = sha256_of(&data.join(PRIVATE_DB_FILE)).unwrap();
    assert!(list(data).is_empty());
    assert!(prune(data, 1).is_empty());
    let name = old.trim_start_matches("respaldos/");
    let refused = restore(data, name, CURRENT_VERSION).unwrap_err();
    assert!(refused.contains("base anterior"), "{refused}");
    assert_eq!(sha256_of(&data.join(PRIVATE_DB_FILE)).unwrap(), live);
    assert_eq!(folders_in(&data.join(BACKUPS_DIR)), vec![name.to_string()]);
    // Opening the base takes its own backup, and only that one is listed.
    let (base, opening) = DocBase::open_in_data_folder(data);
    base.unwrap();
    let listed: Vec<String> = list(data).into_iter().map(|b| b.name).collect();
    assert_eq!(listed, vec![opening.backup.unwrap()]);
    assert!(data.join(old).join("docbase/private.sqlite3").is_file());
}
