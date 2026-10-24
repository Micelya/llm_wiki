//! What the app does with the person's data when a new version of the app
//! changes the shape of the base (D-496, P-297): before the first change it
//! keeps a backup, and without one it changes nothing.
//!
//! - The base is copied by SQLite itself (`VACUUM INTO`), never by copying
//!   the file of an open base: a consistent copy, with what the WAL holds.
//! - A backup is a folder `respaldos/antes-de-v{new}-{AAAA-MM-DD-HHMM}/`
//!   inside the data folder, with the layout of the data folder
//!   (`docbase/private.sqlite3`, and the settings and the chat as they
//!   were), the SHA-256 of each file (`huellas.sha256`), what it is
//!   (`respaldo.json`: the version of the base it holds, the one it was
//!   going to, when) and how to go back to it, for a person (`LEEME.txt`).
//! - The copy is checked before anything changes: it opens, it is at the
//!   version of the base, it passes `quick_check`, and each table has the
//!   rows the base has. Until it is complete its folder has another name
//!   (`sin-terminar-…`), so a backup cut half-way is never taken for one.
//! - The last `KEEP` backups are kept. The older ones go only after an
//!   update that went well, never the only one, and never a folder this
//!   module did not write.
//! - Going back to a backup is for an app that finds the base written by a
//!   newer version (the person went back to the version before): the base
//!   in use is set apart, never deleted, and the backup's takes its place.
//!
//! Nothing here depends on which version comes next: a new entry of
//! `schema::MIGRATIONS` is backed up the same way.
//!
//! Design: `docs/design/actualizaciones.md`.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{sql_err, PRIVATE_DB_FILE};

/// The folder of the backups, inside the data folder.
pub const BACKUPS_DIR: &str = "respaldos";
/// How many backups are kept.
pub const KEEP: usize = 3;
/// Where the app writes which version of it last opened the base
/// (`docbase_setting`), for the backups to say it.
const APP_VERSION_SETTING: &str = "app-version";

const PREFIX: &str = "antes-de-v";
const SET_ASIDE_PREFIX: &str = "apartada-";
const UNFINISHED_PREFIX: &str = "sin-terminar-";
const MANIFEST: &str = "respaldo.json";
const PRINTS: &str = "huellas.sha256";
const README: &str = "LEEME.txt";
const KIND: &str = "micelya-respaldo";

/// Besides the base, the data files of a person that a backup keeps as
/// they were: the settings, the chat, the readings asked to Core and the
/// catalogs that replace the app's. None of them changes shape today (they
/// are read leniently by older and newer versions); they are kept so that
/// going back leaves everything as it was, if it ever has to.
const ALSO_KEPT: &[&str] = &[
    "app-state.json",
    super::chat_files::CHAT_FOLDER,
    crate::micelya_core::ledger::LEDGER_FILE,
    "docbase/document-types.json",
    "docbase/knowledge.json",
];

/// What the app found and did when it opened its base, for Ajustes.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opening {
    /// The version of the base when the app found it (0: there was none).
    pub found_version: Option<i64>,
    /// The backup made before changing its shape, if it was changed.
    pub backup: Option<String>,
    /// The older backups deleted after it.
    pub pruned: Vec<String>,
    /// Why the base could not be opened, in words for a person.
    pub problem: Option<String>,
    /// The base was written by a newer version of the app: a backup can be
    /// restored.
    pub newer: bool,
}

/// What `respaldo.json` says.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub kind: String,
    pub format: u32,
    /// The version of the base kept here.
    pub base_version: i64,
    /// The version the app was taking it to.
    pub for_version: i64,
    /// The version of the app that last wrote the base, when the base says.
    pub written_by_app: Option<String>,
    /// The version of the app that made the backup.
    pub made_by_app: String,
    pub made_at_ms: i64,
    /// When, in the local time of the computer, for a person.
    pub made_at: String,
    pub files: Vec<KeptFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeptFile {
    /// Inside the backup, with forward slashes: the same place it has in
    /// the data folder.
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

/// A backup, for Ajustes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub name: String,
    pub base_version: i64,
    pub for_version: i64,
    pub written_by_app: Option<String>,
    pub made_at: String,
    pub made_at_ms: i64,
    pub bytes: u64,
    /// Every file is there with its size (the fingerprints are checked
    /// before going back to it).
    pub complete: bool,
}

/// The words a person reads when the base was written by a newer version.
pub fn newer_message(found: i64, current: i64) -> String {
    format!(
        "Esta base la escribió una versión más nueva de Micelya (versión {found} de la base; esta versión de la app llega hasta la {current}). \
         No se tocó nada. Para seguir con esta versión, volvé al respaldo de antes de la actualización en Ajustes → Datos y respaldos, \
         o instalá de nuevo la versión más nueva."
    )
}

/// The words a person reads when the backup could not be made.
pub fn no_backup_message(from: i64, to: i64, cause: &str) -> String {
    format!(
        "Antes de actualizar la base de la versión {from} a la {to}, Micelya tiene que guardar un respaldo y esta vez no pudo ({cause}). \
         No se cambió nada: tus datos están intactos, como los dejó la versión anterior de la app. \
         Qué hacer: fijate que haya lugar libre en el disco, cerrá Micelya del todo y volvé a abrirla. \
         Si vuelve a pasar, avisá a soporte con este mensaje."
    )
}

/// The words a person reads when the backup was made and the update failed.
pub fn update_failed_message(from: i64, to: i64, backup: &str, cause: &str) -> String {
    format!(
        "No se pudo terminar de actualizar la base de la versión {from} a la {to} ({cause}). \
         Tus datos están en el respaldo «{backup}», hecho justo antes, en la carpeta de respaldos. \
         Qué hacer: no borres nada; cerrá Micelya del todo y volvé a abrirla. Si vuelve a pasar, avisá a soporte con este mensaje."
    )
}

/// Keeps a backup of the data before the base at `from` is taken to `to`.
/// `conn` is the base, open and not yet changed. Returns the backup's
/// name; on any failure nothing is left half-written.
pub fn take(conn: &Connection, data_dir: &Path, from: i64, to: i64) -> Result<String, String> {
    let backups = data_dir.join(BACKUPS_DIR);
    fs::create_dir_all(&backups)
        .map_err(|e| format!("no se pudo crear la carpeta de respaldos «{}»: {e}", backups.display()))?;
    forget_unfinished(&backups);
    let now = chrono::Local::now();
    let name = free_name(&backups, &format!("{PREFIX}{to}-{}", now.format("%Y-%m-%d-%H%M")));
    let work = backups.join(format!("{UNFINISHED_PREFIX}{name}"));
    let made = write_backup(conn, data_dir, &work, from, to, &now)
        .and_then(|()| fs::rename(&work, backups.join(&name)).map_err(|e| format!("no se pudo cerrar el respaldo: {e}")));
    if made.is_err() {
        let _ = fs::remove_dir_all(&work);
    }
    made.map(|()| name)
}

fn write_backup(
    conn: &Connection,
    data_dir: &Path,
    work: &Path,
    from: i64,
    to: i64,
    now: &chrono::DateTime<chrono::Local>,
) -> Result<(), String> {
    let base_copy = work.join(PRIVATE_DB_FILE);
    let parent = base_copy.parent().unwrap_or(work);
    fs::create_dir_all(parent).map_err(|e| format!("no se pudo crear «{}»: {e}", parent.display()))?;
    let target = base_copy
        .to_str()
        .ok_or_else(|| format!("la carpeta «{}» tiene un nombre que la base no acepta", work.display()))?;
    conn.execute("VACUUM INTO ?1", [target])
        .map_err(|e| format!("la base no se pudo copiar: {e}"))?;
    check_copy(conn, &base_copy, from)?;
    for rel in ALSO_KEPT {
        copy_into(&data_dir.join(rel), &work.join(rel))?;
    }
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(work).sort_by_file_name() {
        let entry = entry.map_err(|e| format!("no se pudo leer el respaldo: {e}"))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = relative(work, entry.path());
        files.push(KeptFile {
            bytes: entry.metadata().map_err(|e| format!("no se pudo leer «{rel}»: {e}"))?.len(),
            sha256: sha256_of(entry.path())?,
            path: rel,
        });
    }
    let prints: String = files.iter().map(|file| format!("{}  {}\n", file.sha256, file.path)).collect();
    write(&work.join(PRINTS), prints.as_bytes())?;
    let manifest = Manifest {
        kind: KIND.to_string(),
        format: 1,
        base_version: from,
        for_version: to,
        written_by_app: app_version_of(conn),
        made_by_app: env!("CARGO_PKG_VERSION").to_string(),
        made_at_ms: now.timestamp_millis(),
        made_at: now.format("%d/%m/%Y %H:%M").to_string(),
        files,
    };
    let json = serde_json::to_vec_pretty(&manifest).map_err(|e| format!("no se pudo escribir {MANIFEST}: {e}"))?;
    write(&work.join(MANIFEST), &json)?;
    write(&work.join(README), readme(&manifest).as_bytes())
}

/// The copy opens, is at the version of the base, is sound, and each table
/// has the rows of the base. It is left as one file, without a journal.
fn check_copy(conn: &Connection, copy: &Path, from: i64) -> Result<(), String> {
    let kept = Connection::open(copy).map_err(|e| format!("la copia de la base no abre: {e}"))?;
    kept.pragma_update(None, "journal_mode", "DELETE")
        .map_err(|e| format!("la copia de la base no se pudo cerrar: {e}"))?;
    let version: i64 = kept.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(sql_err)?;
    if version != from {
        return Err(format!("la copia está en la versión {version} y la base en la {from}"));
    }
    let check: String = kept.query_row("PRAGMA quick_check", [], |row| row.get(0)).map_err(sql_err)?;
    if check != "ok" {
        return Err(format!("la copia de la base no está sana: {check}"));
    }
    for table in tables(conn)? {
        let (base, copy) = (count(conn, &table)?, count(&kept, &table)?);
        if base != copy {
            return Err(format!("la copia de la tabla {table} tiene {copy} filas y la base {base}"));
        }
    }
    Ok(())
}

/// The tables of a base that hold data: its own and the virtual ones (the
/// text indexes), not the ones SQLite keeps for them.
pub(crate) fn tables(conn: &Connection) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(
            "SELECT name FROM pragma_table_list
             WHERE schema = 'main' AND type IN ('table', 'virtual') AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\'
             ORDER BY name",
        )
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| row.get(0)).map_err(sql_err)?;
    rows.collect::<Result<Vec<String>, _>>().map_err(sql_err)
}

fn count(conn: &Connection, table: &str) -> Result<i64, String> {
    conn.query_row(&format!("SELECT count(*) FROM {}", quoted(table)), [], |row| row.get(0))
        .map_err(sql_err)
}

pub(crate) fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn app_version_of(conn: &Connection) -> Option<String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [APP_VERSION_SETTING], |row| row.get(0))
        .ok()
}

/// Writes in the base the version of the app that opened it, for the next
/// backup to say which version wrote the base it keeps.
pub(crate) fn note_app_version(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
        [APP_VERSION_SETTING, env!("CARGO_PKG_VERSION")],
    )
    .map(|_| ())
    .map_err(sql_err)
}

/// Copies a file, or a folder with everything in it, if it is there.
fn copy_into(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry.map_err(|e| format!("no se pudo leer «{}»: {e}", from.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let target = match entry.path().strip_prefix(from) {
            Ok(rest) if rest.as_os_str().is_empty() => to.to_path_buf(),
            Ok(rest) => to.join(rest),
            Err(_) => continue,
        };
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("no se pudo crear «{}»: {e}", parent.display()))?;
        }
        fs::copy(entry.path(), &target).map_err(|e| format!("no se pudo copiar «{}»: {e}", entry.path().display()))?;
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| format!("no se pudo escribir «{}»: {e}", path.display()))
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// SHA-256 of a file, in lowercase hex, whatever its size.
pub(crate) fn sha256_of(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("no se pudo abrir «{}»: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 256 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|e| format!("no se pudo leer «{}»: {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// `wanted`, or `wanted-2`, `wanted-3`… if a folder (finished or not)
/// already has that name.
fn free_name(backups: &Path, wanted: &str) -> String {
    let taken = |name: &str| backups.join(name).exists() || backups.join(format!("{UNFINISHED_PREFIX}{name}")).exists();
    if !taken(wanted) {
        return wanted.to_string();
    }
    (2..).map(|n| format!("{wanted}-{n}")).find(|name| !taken(name)).unwrap_or_else(|| wanted.to_string())
}

/// A backup cut half-way (the app closed while making it) is not a backup.
fn forget_unfinished(backups: &Path) {
    let Ok(entries) = fs::read_dir(backups) else { return };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(UNFINISHED_PREFIX) && entry.path().is_dir() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

fn readme(manifest: &Manifest) -> String {
    let others = if manifest.files.len() > 1 { ", y los ajustes y el chat como estaban" } else { "" };
    format!(
        "Respaldo de los datos de Micelya Desktop\r\n\
         \r\n\
         Lo hizo la app sola el {made_at}, antes de actualizar la base de la versión {from} a la {to}.\r\n\
         Tiene la base tal como estaba (docbase/private.sqlite3){others}.\r\n\
         La huella SHA-256 de cada archivo está en {PRINTS}. No cambies nada de esta carpeta ni su nombre.\r\n\
         \r\n\
         Para volver a este respaldo, con la versión de la app de antes de la actualización:\r\n\
         - Si la app lo ofrece: Ajustes > Datos y respaldos > «Volver a este respaldo».\r\n\
         - A mano:\r\n\
         \x20 1. Cerrá Micelya del todo (también desde el ícono junto al reloj).\r\n\
         \x20 2. En la carpeta de datos, abrí «docbase» y mové a otra carpeta los archivos private.sqlite3,\r\n\
         \x20    private.sqlite3-wal y private.sqlite3-shm que haya (no los borres).\r\n\
         \x20 3. Copiá en «docbase» el archivo private.sqlite3 de la carpeta «docbase» de este respaldo.\r\n\
         \x20 4. Abrí Micelya.\r\n\
         Se pierde lo que se hizo en la app después de la fecha de este respaldo.\r\n",
        made_at = manifest.made_at,
        from = manifest.base_version,
        to = manifest.for_version,
    )
}

fn read_manifest(folder: &Path) -> Option<Manifest> {
    let raw = fs::read(folder.join(MANIFEST)).ok()?;
    let manifest: Manifest = serde_json::from_slice(&raw).ok()?;
    (manifest.kind == KIND).then_some(manifest)
}

/// The backups this module wrote, newest first.
fn ours(data_dir: &Path) -> Vec<(String, Manifest)> {
    let Ok(entries) = fs::read_dir(data_dir.join(BACKUPS_DIR)) else { return Vec::new() };
    let mut found: Vec<(String, Manifest)> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(PREFIX) {
                return None;
            }
            read_manifest(&entry.path()).map(|manifest| (name, manifest))
        })
        .collect();
    found.sort_by(|a, b| b.1.made_at_ms.cmp(&a.1.made_at_ms).then_with(|| b.0.cmp(&a.0)));
    found
}

/// The backups there are, newest first.
pub fn list(data_dir: &Path) -> Vec<BackupSummary> {
    let backups = data_dir.join(BACKUPS_DIR);
    ours(data_dir)
        .into_iter()
        .map(|(name, manifest)| {
            let folder = backups.join(&name);
            let complete = manifest.files.iter().any(|file| file.path == PRIVATE_DB_FILE)
                && manifest
                    .files
                    .iter()
                    .all(|file| fs::metadata(folder.join(&file.path)).map(|m| m.len() == file.bytes).unwrap_or(false));
            BackupSummary {
                bytes: manifest.files.iter().map(|file| file.bytes).sum(),
                name,
                base_version: manifest.base_version,
                for_version: manifest.for_version,
                written_by_app: manifest.written_by_app,
                made_at: manifest.made_at,
                made_at_ms: manifest.made_at_ms,
                complete,
            }
        })
        .collect()
}

/// Deletes the backups beyond the newest `keep` (at least one stays).
/// Returns the names deleted.
pub fn prune(data_dir: &Path, keep: usize) -> Vec<String> {
    let backups = data_dir.join(BACKUPS_DIR);
    let mut deleted = Vec::new();
    for (name, _) in ours(data_dir).into_iter().skip(keep.max(1)) {
        match fs::remove_dir_all(backups.join(&name)) {
            Ok(()) => deleted.push(name),
            Err(e) => eprintln!("[docbase] the old backup {name} could not be deleted: {e}"),
        }
    }
    deleted
}

/// The fingerprints of a backup (`huellas.sha256`): each file listed is
/// there and is what it was. Returns what the backup is.
pub fn check(folder: &Path) -> Result<Manifest, String> {
    let manifest = read_manifest(folder).ok_or_else(|| format!("«{}» no es un respaldo de Micelya", folder.display()))?;
    let prints = fs::read_to_string(folder.join(PRINTS)).map_err(|e| format!("el respaldo no tiene sus huellas: {e}"))?;
    let mut listed = 0;
    let mut has_base = false;
    for line in prints.lines().filter(|line| !line.trim().is_empty()) {
        let (hash, path) = line.split_once("  ").ok_or_else(|| format!("una línea de las huellas no se entiende: {line}"))?;
        let path = path.trim_start_matches('*');
        let inside = Path::new(path).components().all(|part| matches!(part, std::path::Component::Normal(_)));
        if !inside {
            return Err(format!("las huellas nombran un archivo fuera del respaldo: {path}"));
        }
        let actual = sha256_of(&folder.join(path))?;
        if actual != hash {
            return Err(format!("«{path}» no es el que se guardó: su huella cambió"));
        }
        has_base |= path == PRIVATE_DB_FILE;
        listed += 1;
    }
    if !has_base || listed == 0 {
        return Err("las huellas del respaldo no nombran la base".to_string());
    }
    Ok(manifest)
}

/// Goes back to a backup: the base in use (with its journal) is set apart
/// in the folder of the backups, never deleted, and the backup's takes its
/// place. Only for a base this app can open (`current`). Returns the name
/// of the folder where the base in use was left.
pub fn restore(data_dir: &Path, name: &str, current: i64) -> Result<String, String> {
    if !name.starts_with(PREFIX) || name.contains(['/', '\\']) {
        return Err(format!("«{name}» no es un respaldo"));
    }
    let backups = data_dir.join(BACKUPS_DIR);
    let folder = backups.join(name);
    let manifest = check(&folder)?;
    if manifest.base_version > current {
        return Err(format!(
            "ese respaldo tiene la base en la versión {}, y esta versión de la app llega hasta la {current}",
            manifest.base_version
        ));
    }
    let live = data_dir.join(PRIVATE_DB_FILE);
    let aside_name = free_name(&backups, &format!("{SET_ASIDE_PREFIX}{}", chrono::Local::now().format("%Y-%m-%d-%H%M%S")));
    let aside = backups.join(&aside_name);
    let aside_base = aside.join(PRIVATE_DB_FILE);
    let aside_dir = aside_base.parent().unwrap_or(&aside);
    fs::create_dir_all(aside_dir).map_err(|e| format!("no se pudo crear «{}»: {e}", aside_dir.display()))?;
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let put_back = |moved: &[(PathBuf, PathBuf)]| {
        for (from, to) in moved.iter().rev() {
            let _ = fs::rename(to, from);
        }
    };
    for suffix in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{}{suffix}", live.display()));
        if !from.exists() {
            continue;
        }
        let to = PathBuf::from(format!("{}{suffix}", aside_base.display()));
        if let Err(e) = fs::rename(&from, &to) {
            put_back(&moved);
            return Err(format!("la base en uso no se pudo apartar (¿está abierta?): {e}"));
        }
        moved.push((from, to));
    }
    let part = PathBuf::from(format!("{}.restaurando", live.display()));
    let expected = manifest.files.iter().find(|file| file.path == PRIVATE_DB_FILE).map(|file| file.sha256.clone());
    let placed = fs::create_dir_all(live.parent().unwrap_or(data_dir))
        .and_then(|()| fs::copy(folder.join(PRIVATE_DB_FILE), &part))
        .map_err(|e| format!("la base del respaldo no se pudo copiar: {e}"))
        .and_then(|_| {
            let copied = sha256_of(&part)?;
            if Some(copied) == expected {
                Ok(())
            } else {
                Err("la copia de la base del respaldo no salió igual".to_string())
            }
        })
        .and_then(|()| fs::rename(&part, &live).map_err(|e| format!("la base del respaldo no se pudo poner en su lugar: {e}")));
    if let Err(e) = placed {
        let _ = fs::remove_file(&part);
        put_back(&moved);
        return Err(e);
    }
    let note = format!(
        "La base que estaba en uso cuando se volvió al respaldo «{name}» ({}).\r\n\
         La app no la borra: si estás seguro de que no la necesitás, podés borrar esta carpeta.\r\n",
        chrono::Local::now().format("%d/%m/%Y %H:%M")
    );
    let _ = fs::write(aside.join(README), note);
    Ok(aside_name)
}

#[cfg(test)]
#[path = "upgrade_tests.rs"]
mod tests;
