//! The programs a person lets use the base (6.2 of the contract's design):
//! an assistant such as Claude Code, or a script. Each one has its own
//! token, given once in Ajustes and kept in the system keychain so it can
//! be copied again; the base keeps only the list, with each program's
//! permission, its last use and the SHA-256 of its token, to know it
//! without the keychain. By default a program only reads; with «leer y
//! pedir cambios» it can also ask for changes, which wait for a person in
//! Atención. A token is renewed (the old one stops working) or the program
//! removed without touching the others.
//!
//! Whether programs may use the server at all is a switch of Ajustes
//! (`docbase_setting`); the web clipper does not depend on it.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::docbase::{now_ms, sql_err, DocBase};
use crate::secrets::{SecretStore, PROGRAM_TOKEN_PREFIX};

use super::auth::same;

/// The switch of Ajustes (`docbase_setting`): `off` when a person turned
/// the programs off; anything else, on.
const SWITCH: &str = "local-programs";
/// The longest name of a program.
const MAX_NAME: usize = 80;
/// What every token starts with, to recognize one copied somewhere.
const TOKEN_MARK: &str = "mcy_";

/// What a program may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Only the readings of the contract (the default).
    Read,
    /// The readings, and asking for changes that wait for a person.
    Change,
}

impl Permission {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text.trim() {
            "read" => Ok(Self::Read),
            "change" => Ok(Self::Change),
            other => Err(format!("«{other}» no es un permiso: es «read» (solo leer) o «change» (leer y pedir cambios)")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Change => "change",
        }
    }

    /// In words, for a program that asks.
    pub fn words(self) -> &'static str {
        match self {
            Self::Read => "solo leer",
            Self::Change => "leer y pedir cambios",
        }
    }
}

/// A program, as Ajustes shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    pub id: String,
    pub name: String,
    pub permission: Permission,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    /// How many operations it used in the last 30 days.
    pub uses: i64,
}

/// A program just added or with its token renewed: the token, to copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WithToken {
    pub program: Program,
    pub token: String,
    /// Whether the keychain kept it, to copy it again later; when it did
    /// not, it is shown only now.
    pub kept: bool,
}

fn key_of(id: &str) -> String {
    format!("{PROGRAM_TOKEN_PREFIX}{id}")
}

/// The SHA-256 of a token, in hexadecimal.
pub fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// A new token: 32 random bytes (from the system's generator, through two
/// version 4 UUIDs), in base64 without padding, after `mcy_`.
fn new_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    format!("{TOKEN_MARK}{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn new_id() -> String {
    let simple = uuid::Uuid::new_v4().simple().to_string();
    format!("prg-{}", &simple[..10])
}

fn program(row: &Row) -> rusqlite::Result<Program> {
    let permission: String = row.get(2)?;
    Ok(Program {
        id: row.get(0)?,
        name: row.get(1)?,
        permission: if permission == "change" { Permission::Change } else { Permission::Read },
        created_at: row.get(3)?,
        last_used_at: row.get(4)?,
        uses: row.get(5)?,
    })
}

const SELECT: &str = "SELECT p.id, p.name, p.permission, p.created_at, p.last_used_at,
        (SELECT COALESCE(sum(u.count), 0) FROM local_program_use u WHERE u.program_id = p.id AND u.day >= ?1)
     FROM local_program p";

fn month_ago() -> String {
    (chrono::Local::now() - chrono::Duration::days(30)).format("%Y-%m-%d").to_string()
}

/// The programs a person added and did not remove, the oldest first.
pub fn list(base: &DocBase) -> Result<Vec<Program>, String> {
    let conn = base.conn();
    let mut statement = conn.prepare(&format!("{SELECT} WHERE p.revoked_at IS NULL ORDER BY p.created_at, p.id")).map_err(sql_err)?;
    let rows = statement.query_map([month_ago()], program).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// One program that was not removed.
pub fn get(base: &DocBase, id: &str) -> Result<Option<Program>, String> {
    let conn = base.conn();
    conn.query_row(&format!("{SELECT} WHERE p.id = ?2 AND p.revoked_at IS NULL"), params![month_ago(), id], program)
        .optional()
        .map_err(sql_err)
}

/// The name of a program, also of one removed (for the record).
#[cfg_attr(not(test), allow(dead_code))]
pub fn name_of(base: &DocBase, id: &str) -> Result<Option<String>, String> {
    base.conn().query_row("SELECT name FROM local_program WHERE id = ?1", [id], |row| row.get(0)).optional().map_err(sql_err)
}

fn checked_name(base: &DocBase, name: &str, except: Option<&str>) -> Result<String, String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("Falta el nombre del programa".to_string());
    }
    if name.chars().count() > MAX_NAME {
        return Err(format!("El nombre es demasiado largo: hasta {MAX_NAME} letras"));
    }
    let taken = list(base)?.into_iter().any(|p| Some(p.id.as_str()) != except && p.name.to_lowercase() == name.to_lowercase());
    if taken {
        return Err(format!("Ya hay un programa que se llama «{name}»"));
    }
    Ok(name)
}

/// Keeps the token in the keychain; whether it kept it, read back the same.
fn keep_token(store: &dyn SecretStore, id: &str, token: &str) -> bool {
    let key = key_of(id);
    store.set(&key, token).is_ok() && matches!(store.get(&key), Ok(Some(read)) if read == token)
}

/// Adds a program with its own token.
pub fn add(base: &DocBase, store: &dyn SecretStore, name: &str, permission: Permission) -> Result<WithToken, String> {
    let name = checked_name(base, name, None)?;
    let id = new_id();
    let token = new_token();
    base.conn()
        .execute(
            "INSERT INTO local_program (id, name, permission, token_hash, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, permission.as_str(), token_hash(&token), now_ms()],
        )
        .map_err(sql_err)?;
    let kept = keep_token(store, &id, &token);
    let program = get(base, &id)?.ok_or_else(|| "El programa no quedó guardado".to_string())?;
    Ok(WithToken { program, token, kept })
}

/// The token of a program, to copy it again.
pub fn token(base: &DocBase, store: &dyn SecretStore, id: &str) -> Result<String, String> {
    let program = get(base, id)?.ok_or_else(|| "Ese programa ya no está".to_string())?;
    match store.get(&key_of(&program.id))? {
        Some(token) if token_hash(&token) == stored_hash(base, &program.id)? => Ok(token),
        _ => Err("El token de este programa no está en el llavero del sistema: renovalo para tener uno nuevo".to_string()),
    }
}

fn stored_hash(base: &DocBase, id: &str) -> Result<String, String> {
    base.conn().query_row("SELECT token_hash FROM local_program WHERE id = ?1", [id], |row| row.get(0)).map_err(sql_err)
}

/// A new token for a program: the old one stops working at once.
pub fn renew(base: &DocBase, store: &dyn SecretStore, id: &str) -> Result<WithToken, String> {
    get(base, id)?.ok_or_else(|| "Ese programa ya no está".to_string())?;
    let token = new_token();
    base.conn()
        .execute("UPDATE local_program SET token_hash = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, token_hash(&token)])
        .map_err(sql_err)?;
    let kept = keep_token(store, id, &token);
    let program = get(base, id)?.ok_or_else(|| "Ese programa ya no está".to_string())?;
    Ok(WithToken { program, token, kept })
}

pub fn set_permission(base: &DocBase, id: &str, permission: Permission) -> Result<Program, String> {
    let changed = base
        .conn()
        .execute("UPDATE local_program SET permission = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, permission.as_str()])
        .map_err(sql_err)?;
    if changed == 0 {
        return Err("Ese programa ya no está".to_string());
    }
    get(base, id)?.ok_or_else(|| "Ese programa ya no está".to_string())
}

pub fn rename(base: &DocBase, id: &str, name: &str) -> Result<Program, String> {
    let name = checked_name(base, name, Some(id))?;
    base.conn().execute("UPDATE local_program SET name = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, name]).map_err(sql_err)?;
    get(base, id)?.ok_or_else(|| "Ese programa ya no está".to_string())
}

/// Removes a program: its token stops working and leaves the keychain. What
/// it asked for stays in the record, and what still waits can be decided.
pub fn remove(base: &DocBase, store: &dyn SecretStore, id: &str) -> Result<(), String> {
    base.conn()
        .execute("UPDATE local_program SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL", params![id, now_ms()])
        .map_err(sql_err)?;
    if let Err(error) = store.delete(&key_of(id)) {
        eprintln!("[local-server] the token of a removed program could not leave the keychain: {error}");
    }
    Ok(())
}

/// The program whose token this is; none for a wrong one, a renewed one or
/// one of a removed program. Every token is compared whole, without
/// telling how much matched.
pub fn by_token(base: &DocBase, token: &str) -> Result<Option<Program>, String> {
    let given = token_hash(token.trim());
    let hashes: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT id, token_hash FROM local_program WHERE revoked_at IS NULL").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut found = None;
    for (id, hash) in hashes {
        if same(hash.as_bytes(), given.as_bytes()) {
            found = Some(id);
        }
    }
    match found {
        Some(id) => get(base, &id),
        None => Ok(None),
    }
}

/// A program used the server: its last use, and one more use of the
/// operation today (none for asking what there is).
pub fn note_use(base: &DocBase, id: &str, operation: Option<&str>) -> Result<(), String> {
    let conn = base.conn();
    conn.execute("UPDATE local_program SET last_used_at = ?2 WHERE id = ?1", params![id, now_ms()]).map_err(sql_err)?;
    if let Some(operation) = operation {
        let day = chrono::Local::now().format("%Y-%m-%d").to_string();
        conn.execute(
            "INSERT INTO local_program_use (program_id, day, operation, count) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT (program_id, day, operation) DO UPDATE SET count = count + 1",
            params![id, day, operation],
        )
        .map_err(sql_err)?;
    }
    Ok(())
}

/// Whether programs may use the server (the switch of Ajustes; on unless
/// a person turned it off).
pub fn enabled(base: &DocBase) -> Result<bool, String> {
    let value: Option<String> = base
        .conn()
        .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [SWITCH], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(value.as_deref() != Some("off"))
}

pub fn set_enabled(base: &DocBase, on: bool) -> Result<(), String> {
    base.conn()
        .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![SWITCH, if on { "on" } else { "off" }])
        .map_err(sql_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::fake::FakeKeychain;

    #[test]
    fn a_program_has_its_own_token_that_is_copied_again_renewed_or_removed() {
        let base = DocBase::in_memory().unwrap();
        let keychain = FakeKeychain::default();
        let added = add(&base, &keychain, "  Asistente   de prueba ", Permission::Read).unwrap();
        assert_eq!(added.program.name, "Asistente de prueba");
        assert_eq!(added.program.permission, Permission::Read);
        assert!(added.kept);
        assert!(added.token.starts_with("mcy_") && added.token.len() > 40, "{}", added.token);
        // The base keeps only its fingerprint; the keychain, the token.
        let hash: String = base.conn().query_row("SELECT token_hash FROM local_program", [], |row| row.get(0)).unwrap();
        assert_eq!(hash, token_hash(&added.token));
        assert_ne!(hash, added.token);
        assert_eq!(token(&base, &keychain, &added.program.id).unwrap(), added.token);
        assert_eq!(by_token(&base, &added.token).unwrap().map(|p| p.id), Some(added.program.id.clone()));
        assert_eq!(by_token(&base, "mcy_otro").unwrap(), None);
        // Another program, with another token.
        let other = add(&base, &keychain, "Script de prueba", Permission::Change).unwrap();
        assert_ne!(other.token, added.token);
        assert!(add(&base, &keychain, "script de PRUEBA", Permission::Read).is_err());
        assert!(add(&base, &keychain, " ", Permission::Read).is_err());
        // Renewed: the old token no longer works, the new one does.
        let renewed = renew(&base, &keychain, &added.program.id).unwrap();
        assert_eq!(by_token(&base, &added.token).unwrap(), None);
        assert_eq!(by_token(&base, &renewed.token).unwrap().map(|p| p.id), Some(added.program.id.clone()));
        assert_eq!(token(&base, &keychain, &added.program.id).unwrap(), renewed.token);
        // Its permission and its uses.
        assert_eq!(set_permission(&base, &added.program.id, Permission::Change).unwrap().permission, Permission::Change);
        note_use(&base, &added.program.id, Some("buscar")).unwrap();
        note_use(&base, &added.program.id, Some("buscar")).unwrap();
        note_use(&base, &added.program.id, None).unwrap();
        let used = get(&base, &added.program.id).unwrap().unwrap();
        assert_eq!(used.uses, 2);
        assert!(used.last_used_at.is_some());
        // Removed: its token stops working and leaves the keychain; the
        // other program keeps working; its name stays for the record.
        remove(&base, &keychain, &added.program.id).unwrap();
        assert_eq!(by_token(&base, &renewed.token).unwrap(), None);
        assert!(keychain.keys.borrow().get(&format!("local-program:{}", added.program.id)).is_none());
        assert!(token(&base, &keychain, &added.program.id).is_err());
        assert_eq!(by_token(&base, &other.token).unwrap().map(|p| p.id), Some(other.program.id.clone()));
        assert_eq!(list(&base).unwrap().len(), 1);
        assert_eq!(name_of(&base, &added.program.id).unwrap().as_deref(), Some("Asistente de prueba"));
    }

    #[test]
    fn without_a_keychain_the_token_is_shown_only_once_and_still_works() {
        let base = DocBase::in_memory().unwrap();
        let broken = FakeKeychain { broken: true, ..Default::default() };
        let added = add(&base, &broken, "Asistente de prueba", Permission::Read).unwrap();
        assert!(!added.kept);
        assert!(by_token(&base, &added.token).unwrap().is_some());
        assert!(token(&base, &broken, &added.program.id).is_err());
    }

    #[test]
    fn the_programs_are_on_unless_a_person_turned_them_off() {
        let base = DocBase::in_memory().unwrap();
        assert!(enabled(&base).unwrap());
        set_enabled(&base, false).unwrap();
        assert!(!enabled(&base).unwrap());
        set_enabled(&base, true).unwrap();
        assert!(enabled(&base).unwrap());
    }
}
