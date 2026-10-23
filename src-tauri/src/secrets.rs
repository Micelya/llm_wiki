//! The API keys of the providers live in the system keychain (Credential
//! Manager on Windows), not in plain text in `app-state.json` (P-141,
//! D-444).
//!
//! A key belongs to a place of the settings: the main model
//! (`llmConfig.apiKey`, name `llm`), each connection with a provider
//! (`providerConfigs.<id>.apiKey`, `provider:<id>`), the web search
//! (`searchApiConfig.apiKey`, `search`) and each search provider
//! (`searchApiConfig.providerConfigs.<id>.apiKey`, `search:<id>`). The
//! screens keep the keys in memory: they read them from here when the app
//! opens and hand each change back; the file keeps every other setting,
//! with the key empty. A connection with a provider (`connection:<id>`,
//! today Micelya Core) is kept whole in one entry, a JSON text with its
//! secret; only its own commands read it, and never hand the secret back
//! (`micelya_core`).
//!
//! Nothing is ever lost because the keychain failed: a key is emptied in
//! the file only after the keychain kept it and gave it back the same, and
//! a key the keychain did not keep stays in the file, as before. When the
//! app opens, the keys still in the file move to the keychain that way,
//! once (a key in the file is always the latest one: it is there only
//! because the keychain could not keep it).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// The name of the app in the keychain.
const SERVICE: &str = "com.micelya.desktop";
const STORE_FILE: &str = "app-state.json";
/// The settings of the file that may carry a key.
const SETTINGS_WITH_KEYS: [&str; 3] = ["llmConfig", "providerConfigs", "searchApiConfig"];

/// Where the keys are kept.
pub trait SecretStore {
    /// The key kept with this name, `None` when there is none.
    fn get(&self, name: &str) -> Result<Option<String>, String>;
    fn set(&self, name: &str, value: &str) -> Result<(), String>;
    /// Removes it; removing one that is not there is no error.
    fn delete(&self, name: &str) -> Result<(), String>;
}

/// The keychain of the operating system.
pub struct Keychain;

fn entry(name: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, name).map_err(|e| format!("The keychain cannot be used: {e}"))
}

impl SecretStore for Keychain {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        match entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("The keychain could not be read: {e}")),
        }
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        entry(name)?.set_password(value).map_err(|e| format!("The keychain could not keep the key: {e}"))
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("The keychain could not remove the key: {e}")),
        }
    }
}

/// Whether `name` is the name of one of the app's keys: the ones of the
/// settings, the connections with a provider (`connection:<id>`, for now
/// only Micelya Core: `micelya_core`) and the tokens of the programs that
/// use the base through the server inside the machine
/// (`local-program:<id>`, `local_server::programs`).
pub fn is_key_name(name: &str) -> bool {
    if name == "llm" || name == "search" {
        return true;
    }
    name.strip_prefix("provider:")
        .or_else(|| name.strip_prefix("search:"))
        .or_else(|| name.strip_prefix("connection:"))
        .or_else(|| name.strip_prefix(PROGRAM_TOKEN_PREFIX))
        .is_some_and(|id| {
            !id.is_empty() && id.len() <= 100 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

/// Whether `name` is a connection with a provider. Its secret never goes
/// to the screens: a connection is read and written only by its own
/// commands, which give back whether there is a secret, not the secret.
pub fn is_connection_name(name: &str) -> bool {
    is_key_name(name) && name.starts_with("connection:")
}

/// The keychain's name of the token of a program (`local-program:<id>`).
pub const PROGRAM_TOKEN_PREFIX: &str = "local-program:";

/// Whether `name` is the token of a program. Like a connection, only its
/// own commands read it.
pub fn is_program_token_name(name: &str) -> bool {
    is_key_name(name) && name.starts_with(PROGRAM_TOKEN_PREFIX)
}

/// The keys the screens can read and write themselves.
fn is_screen_key(name: &str) -> bool {
    is_key_name(name) && !is_connection_name(name) && !is_program_token_name(name)
}

/// One place of the settings where a key may be: its name in the keychain
/// and the path of its field.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    name: String,
    path: Vec<String>,
}

fn place(name: String, path: &[&str]) -> Place {
    Place { name, path: path.iter().map(|p| p.to_string()).collect() }
}

/// Every place of `settings` (the settings of the file, by their key) where
/// a key may be.
fn places(settings: &Map<String, Value>) -> Vec<Place> {
    let mut found = Vec::new();
    if settings.get("llmConfig").is_some_and(Value::is_object) {
        found.push(place("llm".into(), &["llmConfig", "apiKey"]));
    }
    if let Some(Value::Object(configs)) = settings.get("providerConfigs") {
        for (id, _) in configs.iter().filter(|(_, c)| c.is_object()) {
            found.push(place(format!("provider:{id}"), &["providerConfigs", id, "apiKey"]));
        }
    }
    if let Some(Value::Object(search)) = settings.get("searchApiConfig") {
        found.push(place("search".into(), &["searchApiConfig", "apiKey"]));
        if let Some(Value::Object(configs)) = search.get("providerConfigs") {
            for (id, _) in configs.iter().filter(|(_, c)| c.is_object()) {
                found.push(place(format!("search:{id}"), &["searchApiConfig", "providerConfigs", id, "apiKey"]));
            }
        }
    }
    found.retain(|p| is_key_name(&p.name));
    found
}

fn parent_mut<'a>(settings: &'a mut Map<String, Value>, path: &[String]) -> Option<&'a mut Map<String, Value>> {
    let (first, rest) = path.split_first()?;
    let mut current = settings.get_mut(first)?.as_object_mut()?;
    for step in &rest[..rest.len().saturating_sub(1)] {
        current = current.get_mut(step)?.as_object_mut()?;
    }
    Some(current)
}

/// The key written at `path`; `None` when there is none (or it is empty).
fn key_at(settings: &Map<String, Value>, path: &[String]) -> Option<String> {
    let (first, rest) = path.split_first()?;
    let mut current = settings.get(first)?;
    for step in rest {
        current = current.get(step)?;
    }
    current.as_str().filter(|v| !v.is_empty()).map(str::to_string)
}

fn write_at(settings: &mut Map<String, Value>, path: &[String], value: &str) {
    if let (Some(parent), Some(field)) = (parent_mut(settings, path), path.last()) {
        parent.insert(field.clone(), Value::String(value.to_string()));
    }
}

/// Keeps `value` with `name` and reads it back: true only when the store
/// gives back the same.
fn keep(store: &dyn SecretStore, name: &str, value: &str) -> bool {
    store.set(name, value).is_ok() && matches!(store.get(name), Ok(Some(read)) if read == value)
}

/// What moving the keys did.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Moved {
    /// The names now in the keychain, emptied in the file.
    pub moved: Vec<String>,
    /// The names the keychain did not keep: they stay in the file.
    pub kept_in_file: Vec<String>,
}

/// Moves every key written in `settings` to `store`: each one is kept and
/// read back the same before its field is emptied. One the store could not
/// keep stays where it was.
pub fn move_keys(settings: &mut Map<String, Value>, store: &dyn SecretStore) -> Moved {
    let mut report = Moved::default();
    for place in places(settings) {
        let Some(value) = key_at(settings, &place.path) else { continue };
        if keep(store, &place.name, &value) {
            write_at(settings, &place.path, "");
            report.moved.push(place.name);
        } else {
            report.kept_in_file.push(place.name);
        }
    }
    report
}

/// Writes in `settings` the keys of `store` where the file has none. A key
/// in the file wins: it is there because the keychain could not keep it.
pub fn fill_keys(settings: &mut Map<String, Value>, store: &dyn SecretStore) {
    for place in places(settings) {
        if key_at(settings, &place.path).is_some() {
            continue;
        }
        if let Ok(Some(value)) = store.get(&place.name) {
            write_at(settings, &place.path, &value);
        }
    }
}

/// The keys kept with these names (a name with none is left out); an error
/// when the store cannot be read, so the screens keep the keys in the file.
pub fn read_keys(store: &dyn SecretStore, names: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut found = BTreeMap::new();
    for name in names {
        if !is_screen_key(name) {
            return Err(format!("'{name}' is not the name of a key of the app"));
        }
        if let Some(value) = store.get(name)? {
            found.insert(name.clone(), value);
        }
    }
    Ok(found)
}

/// Keeps these keys (an empty one is removed) and returns the names the
/// store kept, each read back the same. A name left out was not kept: the
/// screens leave that key in the file.
pub fn write_keys(store: &dyn SecretStore, entries: &BTreeMap<String, String>) -> Vec<String> {
    entries
        .iter()
        .filter(|(name, _)| is_screen_key(name))
        .filter(|(name, value)| if value.is_empty() { store.delete(name).is_ok() } else { keep(store, name, value) })
        .map(|(name, _)| name.clone())
        .collect()
}

/// When the app opens, before the screens read the settings: the keys
/// still in the file move to the keychain. It edits the same settings the
/// screens use (the store plugin keeps one copy of the file), and saves
/// only when a key moved.
pub fn move_keys_of_app_state(app: &AppHandle) {
    let store = match app.store(STORE_FILE) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("[secrets] the settings could not be opened: {e}");
            return;
        }
    };
    let mut settings = Map::new();
    for key in SETTINGS_WITH_KEYS {
        if let Some(value) = store.get(key) {
            settings.insert(key.to_string(), value);
        }
    }
    let report = move_keys(&mut settings, &Keychain);
    if !report.kept_in_file.is_empty() {
        eprintln!("[secrets] {} key(s) stay in the settings file: the keychain did not keep them", report.kept_in_file.len());
    }
    if report.moved.is_empty() {
        return;
    }
    for (key, value) in settings {
        store.set(key, value);
    }
    match store.save() {
        Ok(()) => eprintln!("[secrets] {} key(s) moved from the settings file to the system keychain", report.moved.len()),
        Err(e) => eprintln!("[secrets] the keys are in the keychain but the settings file could not be saved: {e}"),
    }
}

/// The keys of the app with these names, from the keychain.
#[tauri::command]
pub fn secrets_read(names: Vec<String>) -> Result<BTreeMap<String, String>, String> {
    read_keys(&Keychain, &names)
}

/// Keeps these keys in the keychain (an empty one is removed); returns the
/// names it kept.
#[tauri::command]
pub fn secrets_write(entries: BTreeMap<String, String>) -> Vec<String> {
    write_keys(&Keychain, &entries)
}

#[cfg(test)]
pub mod fake {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::SecretStore;

    /// A keychain in memory, for the tests; `broken` fails at everything,
    /// like a system without a keychain; `forgetful` says it kept a key
    /// but gives back another.
    #[derive(Default)]
    pub struct FakeKeychain {
        pub keys: RefCell<BTreeMap<String, String>>,
        pub broken: bool,
        pub forgetful: bool,
    }

    impl SecretStore for FakeKeychain {
        fn get(&self, name: &str) -> Result<Option<String>, String> {
            if self.broken {
                return Err("no keychain".into());
            }
            Ok(self.keys.borrow().get(name).cloned())
        }

        fn set(&self, name: &str, value: &str) -> Result<(), String> {
            if self.broken {
                return Err("no keychain".into());
            }
            let kept = if self.forgetful { format!("{value}-otra") } else { value.to_string() };
            self.keys.borrow_mut().insert(name.to_string(), kept);
            Ok(())
        }

        fn delete(&self, name: &str) -> Result<(), String> {
            if self.broken {
                return Err("no keychain".into());
            }
            self.keys.borrow_mut().remove(name);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeKeychain;
    use super::*;
    use serde_json::json;

    // Every key below is made up.
    fn old_settings() -> Map<String, Value> {
        json!({
            "llmConfig": { "provider": "openai", "apiKey": "clave-principal", "model": "m" },
            "providerConfigs": {
                "openai": { "apiKey": "clave-principal", "model": "m" },
                "custom-1a2b": { "apiKey": "clave-propia", "baseUrl": "https://ejemplo.invalid/v1" },
                "codex-cli": { "model": "m", "codexReasoningEffort": "high" }
            },
            "searchApiConfig": {
                "provider": "tavily",
                "apiKey": "clave-busqueda",
                "providerConfigs": { "tavily": { "apiKey": "clave-busqueda" }, "searxng": { "searXngUrl": "http://localhost:8080" } }
            },
            "zoomLevel": 1
        })
        .as_object()
        .unwrap()
        .clone()
    }

    fn no_key_left(settings: &Map<String, Value>) {
        let text = serde_json::to_string(settings).unwrap();
        assert!(!text.contains("clave-"), "a key is still in the file: {text}");
    }

    #[test]
    fn every_key_moves_to_the_keychain_and_the_file_keeps_the_rest() {
        let keychain = FakeKeychain::default();
        let mut settings = old_settings();
        let mut report = move_keys(&mut settings, &keychain);
        report.moved.sort();
        assert_eq!(report.moved, ["llm", "provider:custom-1a2b", "provider:openai", "search", "search:tavily"]);
        assert!(report.kept_in_file.is_empty());
        no_key_left(&settings);
        assert_eq!(settings["llmConfig"]["apiKey"], "");
        assert_eq!(settings["llmConfig"]["model"], "m");
        assert_eq!(settings["providerConfigs"]["custom-1a2b"]["baseUrl"], "https://ejemplo.invalid/v1");
        assert_eq!(settings["providerConfigs"]["codex-cli"], json!({ "model": "m", "codexReasoningEffort": "high" }));
        assert_eq!(settings["searchApiConfig"]["providerConfigs"]["searxng"], json!({ "searXngUrl": "http://localhost:8080" }));
        assert_eq!(settings["zoomLevel"], 1);
        let keys = keychain.keys.borrow().clone();
        assert_eq!(keys["llm"], "clave-principal");
        assert_eq!(keys["provider:custom-1a2b"], "clave-propia");
        assert_eq!(keys["search:tavily"], "clave-busqueda");
        assert_eq!(keys.len(), 5);

        // Once moved, nothing moves again, and filling gives back the same.
        assert_eq!(move_keys(&mut settings, &keychain), Moved::default());
        fill_keys(&mut settings, &keychain);
        assert_eq!(settings, old_settings());
    }

    #[test]
    fn without_a_keychain_the_keys_stay_in_the_file() {
        let mut settings = old_settings();
        let report = move_keys(&mut settings, &FakeKeychain { broken: true, ..Default::default() });
        assert!(report.moved.is_empty());
        assert_eq!(report.kept_in_file.len(), 5);
        assert_eq!(settings, old_settings());
        // Nor when the keychain gives back something else.
        let forgetful = FakeKeychain { forgetful: true, ..Default::default() };
        let report = move_keys(&mut settings, &forgetful);
        assert!(report.moved.is_empty());
        assert_eq!(settings, old_settings());
        // Filling from a keychain that cannot be read changes nothing.
        let mut emptied = old_settings();
        move_keys(&mut emptied, &FakeKeychain::default());
        let before = emptied.clone();
        fill_keys(&mut emptied, &FakeKeychain { broken: true, ..Default::default() });
        assert_eq!(emptied, before);
    }

    #[test]
    fn a_key_in_the_file_is_the_latest_one() {
        let keychain = FakeKeychain::default();
        keychain.keys.borrow_mut().insert("search".into(), "clave-vieja".into());
        let mut settings = old_settings();
        fill_keys(&mut settings, &keychain);
        assert_eq!(settings["searchApiConfig"]["apiKey"], "clave-busqueda");
        move_keys(&mut settings, &keychain);
        assert_eq!(keychain.keys.borrow()["search"], "clave-busqueda");
    }

    #[test]
    fn the_screens_read_and_write_only_the_keys_of_the_app() {
        let keychain = FakeKeychain::default();
        let entries: BTreeMap<String, String> = [
            ("provider:openai", "clave-a"),
            ("search:tavily", "clave-b"),
            ("otra-cosa", "clave-c"),
            ("provider:", "clave-d"),
            ("provider:../x", "clave-e"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        assert_eq!(write_keys(&keychain, &entries), ["provider:openai", "search:tavily"]);
        assert_eq!(keychain.keys.borrow().len(), 2);
        let read = read_keys(&keychain, &["provider:openai".into(), "llm".into()]).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read["provider:openai"], "clave-a");
        assert!(read_keys(&keychain, &["otra-cosa".into()]).is_err());
        // An empty key is removed.
        let cleared: BTreeMap<String, String> = [("provider:openai".to_string(), String::new())].into_iter().collect();
        assert_eq!(write_keys(&keychain, &cleared), ["provider:openai"]);
        assert!(!keychain.keys.borrow().contains_key("provider:openai"));
        // A keychain that does not work keeps nothing and says so.
        let broken = FakeKeychain { broken: true, ..Default::default() };
        assert!(write_keys(&broken, &entries).is_empty());
        assert!(read_keys(&broken, &["llm".into()]).is_err());
        assert!(write_keys(&FakeKeychain { forgetful: true, ..Default::default() }, &entries).is_empty());
    }

    #[test]
    fn a_connection_is_a_key_of_the_app_but_its_secret_never_goes_to_the_screens() {
        assert!(is_key_name("connection:micelya-core"));
        assert!(is_connection_name(crate::micelya_core::CONNECTION_KEY));
        assert!(!is_key_name("connection:"));
        assert!(!is_key_name("connection:../x"));
        assert!(!is_connection_name("provider:openai"));
        let keychain = FakeKeychain::default();
        keychain.keys.borrow_mut().insert("connection:micelya-core".into(), r#"{"client_secret":"secreto-inventado"}"#.into());
        assert!(read_keys(&keychain, &["connection:micelya-core".into()]).is_err());
        let entries: BTreeMap<String, String> = [("connection:micelya-core".to_string(), "otra cosa".to_string())].into_iter().collect();
        assert!(write_keys(&keychain, &entries).is_empty());
        assert!(keychain.keys.borrow()["connection:micelya-core"].contains("secreto-inventado"));
    }

    #[test]
    fn the_token_of_a_program_is_a_key_of_the_app_but_never_goes_to_the_screens() {
        assert!(is_key_name("local-program:prg-1a2b3c4d"));
        assert!(is_program_token_name("local-program:prg-1a2b3c4d"));
        assert!(!is_key_name("local-program:"));
        assert!(!is_program_token_name("connection:micelya-core"));
        let keychain = FakeKeychain::default();
        keychain.keys.borrow_mut().insert("local-program:prg-1".into(), "mcy_token-inventado".into());
        assert!(read_keys(&keychain, &["local-program:prg-1".into()]).is_err());
        let entries: BTreeMap<String, String> = [("local-program:prg-1".to_string(), "otro".to_string())].into_iter().collect();
        assert!(write_keys(&keychain, &entries).is_empty());
        assert_eq!(keychain.keys.borrow()["local-program:prg-1"], "mcy_token-inventado");
    }
}
