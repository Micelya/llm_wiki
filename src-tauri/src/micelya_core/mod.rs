//! Micelya Core, the platform of AI services of Micelya (D-463, D-468):
//! the connection with it, kept in the system keychain, and the client that
//! talks to its API (`client.rs`).
//!
//! The connection is one entry of the keychain, `connection:micelya-core`,
//! whose text is a JSON object: `environment`, `api` (the address of the
//! API), `issuer` (the Keycloak realm that gives the access permits),
//! `tenant` (the organization), `client_id` and `client_secret` (the
//! technical user of the organization, `client_credentials`). Ajustes →
//! Conexiones → Micelya Core edits it. The screens never get the secret
//! back, only whether there is one; a change that brings no secret keeps
//! the one kept, and any other data of the entry stays as it was.

pub mod client;
#[cfg(test)]
pub(crate) mod fake;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::secrets::{Keychain, SecretStore};
use client::{CoreClient, CoreError, Side, Timing};

/// Its name in the keychain.
pub const CONNECTION_KEY: &str = "connection:micelya-core";
/// How the choices of the registry of AI services and the catalog of
/// engines name it (the screens use it; the tests check they agree).
#[allow(dead_code)]
pub const CONNECTION_ID: &str = "micelya-core";
/// How many readings «Probar la conexión» asks for (the most the API gives at once).
const TEST_LIST_LIMIT: u32 = 100;

/// The connection with Micelya Core, as the keychain keeps it.
#[derive(Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct CoreConnection {
    #[serde(default)]
    pub environment: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub tenant: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

/// Never prints the secret.
impl std::fmt::Debug for CoreConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoreConnection")
            .field("environment", &self.environment)
            .field("api", &self.api)
            .field("issuer", &self.issuer)
            .field("tenant", &self.tenant)
            .field("client_id", &self.client_id)
            .field("client_secret", &if self.client_secret.is_empty() { "" } else { "(kept)" })
            .finish()
    }
}

/// What the screens see and edit of the connection: everything but the
/// secret.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionFields {
    pub api: String,
    pub issuer: String,
    pub tenant: String,
    pub client_id: String,
}

/// The connection for the screens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    #[serde(flatten)]
    pub fields: ConnectionFields,
    /// The environment of Core it was given for (development, production…); not edited here.
    pub environment: String,
    /// Whether a secret is kept (it is never shown).
    pub has_secret: bool,
}

impl CoreConnection {
    fn view(&self) -> ConnectionView {
        ConnectionView {
            fields: ConnectionFields {
                api: self.api.clone(),
                issuer: self.issuer.clone(),
                tenant: self.tenant.clone(),
                client_id: self.client_id.clone(),
            },
            environment: self.environment.clone(),
            has_secret: !self.client_secret.is_empty(),
        }
    }
}

/// The entry kept in `store`, as a JSON object; `None` when there is none.
fn read_entry(store: &dyn SecretStore) -> Result<Option<Map<String, Value>>, String> {
    let Some(text) = store.get(CONNECTION_KEY)? else {
        return Ok(None);
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(entry)) => Ok(Some(entry)),
        _ => Err("The connection with Micelya Core kept in the keychain cannot be read".to_string()),
    }
}

/// The connection kept in `store`, `None` when there is none.
pub fn read_connection(store: &dyn SecretStore) -> Result<Option<CoreConnection>, String> {
    match read_entry(store)? {
        Some(entry) => serde_json::from_value(Value::Object(entry))
            .map(Some)
            .map_err(|_| "The connection with Micelya Core kept in the keychain cannot be read".to_string()),
        None => Ok(None),
    }
}

/// The connection of the system keychain, `None` when there is none.
pub fn saved_connection() -> Result<Option<CoreConnection>, String> {
    read_connection(&Keychain)
}

/// An address of Core: `https`, or `http` on this computer only.
fn address(value: &str, what: &str) -> Result<String, String> {
    let value = value.trim().trim_end_matches('/').to_string();
    let parsed = reqwest::Url::parse(&value).map_err(|_| format!("The {what} is not a web address"))?;
    let local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match parsed.scheme() {
        "https" => Ok(value),
        "http" if local => Ok(value),
        _ => Err(format!("The {what} must start with https://")),
    }
}

/// A name without spaces (the organization, the client).
fn name(value: &str, what: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("The {what} is missing"));
    }
    if value.chars().any(char::is_whitespace) || value.len() > 200 {
        return Err(format!("The {what} cannot have spaces"));
    }
    Ok(value)
}

/// `fields` checked and trimmed, with `secret`, or the one of `kept` when
/// `secret` is empty.
fn connection_of(fields: &ConnectionFields, secret: Option<&str>, kept: Option<&CoreConnection>) -> Result<CoreConnection, String> {
    let secret = match secret.map(str::trim).filter(|s| !s.is_empty()) {
        Some(secret) => secret.to_string(),
        None => kept.map(|c| c.client_secret.clone()).filter(|s| !s.is_empty()).ok_or("The secret of the client is missing")?,
    };
    Ok(CoreConnection {
        environment: kept.map(|c| c.environment.clone()).unwrap_or_default(),
        api: address(&fields.api, "address of the API")?,
        issuer: address(&fields.issuer, "issuer")?,
        tenant: name(&fields.tenant, "organization")?,
        client_id: name(&fields.client_id, "identifier of the client")?,
        client_secret: secret,
    })
}

/// Keeps `fields` and, when given, a new secret (without one, the secret
/// kept stays). Any other data of the entry stays as it was.
pub fn write_connection(store: &dyn SecretStore, fields: &ConnectionFields, secret: Option<&str>) -> Result<ConnectionView, String> {
    let entry = read_entry(store)?;
    let kept = match &entry {
        Some(entry) => serde_json::from_value::<CoreConnection>(Value::Object(entry.clone())).ok(),
        None => None,
    };
    let connection = connection_of(fields, secret, kept.as_ref())?;
    let mut entry = entry.unwrap_or_default();
    let Value::Object(known) = serde_json::to_value(&connection).map_err(|e| e.to_string())? else {
        unreachable!("a connection is an object");
    };
    entry.extend(known);
    let text = Value::Object(entry).to_string();
    store.set(CONNECTION_KEY, &text)?;
    // Kept only if the keychain gives it back the same.
    match store.get(CONNECTION_KEY)? {
        Some(read) if read == text => Ok(connection.view()),
        _ => Err("The keychain did not keep the connection with Micelya Core".to_string()),
    }
}

/// What «Probar la conexión» found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub ok: bool,
    /// The step that failed: `fields` (something is missing or wrong),
    /// `permit` (the issuer) or `readings` (the API).
    pub step: Option<String>,
    /// What failed: `fields`, `unreachable`, `timeout`, `rejected`,
    /// `organization`, `forbidden`, `not-found`, `unavailable` or `other`.
    pub problem: Option<String>,
    /// The HTTP status of the answer, when there was one.
    pub status: Option<u16>,
    /// What the other side or the check said (never the secret nor the
    /// permit).
    pub detail: Option<String>,
    /// How many readings Core keeps for this credential, when it answered.
    pub readings: Option<usize>,
    /// There are more than `readings`.
    pub more_readings: bool,
}

impl ConnectionTest {
    fn failed(step: &str, problem: &str, status: Option<u16>, detail: String) -> Self {
        Self {
            ok: false,
            step: Some(step.to_string()),
            problem: Some(problem.to_string()),
            status,
            detail: Some(detail),
            ..Default::default()
        }
    }

    fn of_error(error: &CoreError) -> Self {
        let step = |side: &Side| if *side == Side::Issuer { "permit" } else { "readings" };
        match error {
            CoreError::NoAnswer { side, timed_out, detail, .. } => {
                Self::failed(step(side), if *timed_out { "timeout" } else { "unreachable" }, None, detail.clone())
            }
            CoreError::Refused { side, status, code, detail } => {
                let code = code.as_deref().unwrap_or("");
                let problem = match (side, *status) {
                    (Side::Issuer, _) if code == "invalid_scope" => "organization",
                    (Side::Issuer, 400 | 401 | 403) => "rejected",
                    (Side::Api, 401) => "rejected",
                    (Side::Api, 403) => "forbidden",
                    (Side::Api, 409) => "organization",
                    (_, 404) => "not-found",
                    (_, 500..=599) => "unavailable",
                    _ => "other",
                };
                let detail = if code.is_empty() { detail.clone() } else { format!("{code}: {detail}") };
                Self::failed(step(side), problem, Some(*status), detail)
            }
            CoreError::Unexpected { side, detail } => Self::failed(step(side), "other", None, detail.clone()),
        }
    }
}

/// How «Probar la conexión» waits: once, without repeating, so a wrong
/// address says so soon.
fn test_timing() -> Timing {
    Timing { call: Duration::from_secs(15), retry_waits: Vec::new(), ..Timing::default() }
}

/// Asks for a new access permit and the list of readings with `connection`.
pub fn test_connection(connection: &CoreConnection, timing: Timing) -> ConnectionTest {
    let client = match CoreClient::new(connection.clone(), timing) {
        Ok(client) => client,
        Err(error) => return ConnectionTest::of_error(&error),
    };
    if let Err(error) = client.new_permit() {
        return ConnectionTest::of_error(&error);
    }
    match client.list_readings(TEST_LIST_LIMIT, None) {
        Ok(list) => ConnectionTest {
            ok: true,
            readings: Some(list.items.len()),
            more_readings: list.next_cursor.is_some(),
            ..Default::default()
        },
        Err(error) => ConnectionTest::of_error(&error),
    }
}

/// Tests `fields` with `secret`, or with the secret kept in `store`.
pub fn test_fields(store: &dyn SecretStore, fields: &ConnectionFields, secret: Option<&str>, timing: Timing) -> ConnectionTest {
    let kept = match read_connection(store) {
        Ok(kept) => kept,
        Err(error) => return ConnectionTest::failed("fields", "fields", None, error),
    };
    match connection_of(fields, secret, kept.as_ref()) {
        Ok(connection) => test_connection(&connection, timing),
        Err(error) => ConnectionTest::failed("fields", "fields", None, error),
    }
}

/// The connection with Micelya Core, without its secret; `None` when there
/// is none.
#[tauri::command]
pub fn micelya_core_connection() -> Result<Option<ConnectionView>, String> {
    Ok(saved_connection()?.map(|connection| connection.view()))
}

/// Keeps the connection in the keychain: these fields and, when given, a
/// new secret (without one, the secret kept stays).
#[tauri::command]
pub fn micelya_core_save_connection(fields: ConnectionFields, secret: Option<String>) -> Result<ConnectionView, String> {
    write_connection(&Keychain, &fields, secret.as_deref())
}

/// Removes the connection from the keychain.
#[tauri::command]
pub fn micelya_core_remove_connection() -> Result<(), String> {
    Keychain.delete(CONNECTION_KEY)
}

/// «Probar la conexión»: asks for an access permit and the list of
/// readings with these fields and the secret given (or the one kept),
/// without keeping anything.
#[tauri::command]
pub async fn micelya_core_test_connection(fields: ConnectionFields, secret: Option<String>) -> Result<ConnectionTest, String> {
    // The client blocks: it runs where blocking is allowed.
    tauri::async_runtime::spawn_blocking(move || test_fields(&Keychain, &fields, secret.as_deref(), test_timing()))
        .await
        .map_err(|e| format!("The test of the connection stopped: {e}"))
}

#[cfg(test)]
mod tests {
    use super::fake::FakeCore;
    use super::*;
    use crate::secrets::fake::FakeKeychain;

    // Every value below is made up.
    const KEPT: &str = r#"{"environment":"ambiente-de-prueba","api":"https://api.ejemplo.invalid/v1","issuer":"https://emisor.ejemplo.invalid/realms/prueba","tenant":"organizacion-de-prueba","client_id":"cliente-de-prueba","client_secret":"secreto-de-prueba"}"#;

    fn keychain_with(text: &str) -> FakeKeychain {
        let keychain = FakeKeychain::default();
        keychain.keys.borrow_mut().insert(CONNECTION_KEY.to_string(), text.to_string());
        keychain
    }

    fn fields_of(core: &FakeCore) -> ConnectionFields {
        let connection = core.connection();
        ConnectionFields { api: connection.api, issuer: connection.issuer, tenant: connection.tenant, client_id: connection.client_id }
    }

    fn quick() -> Timing {
        FakeCore::timing()
    }

    #[test]
    fn the_entry_kept_in_the_keychain_is_read_as_it_is_and_the_screens_never_get_the_secret() {
        let keychain = keychain_with(KEPT);
        let connection = read_connection(&keychain).unwrap().unwrap();
        assert_eq!(connection.environment, "ambiente-de-prueba");
        assert_eq!(connection.api, "https://api.ejemplo.invalid/v1");
        assert_eq!(connection.client_secret, "secreto-de-prueba");
        let view = connection.view();
        assert!(view.has_secret);
        let shown = serde_json::to_string(&view).unwrap();
        assert!(!shown.contains("secreto"), "{shown}");
        assert_eq!(serde_json::from_str::<Value>(&shown).unwrap()["clientId"], "cliente-de-prueba");
        assert!(!format!("{connection:?}").contains("secreto"));
        assert_eq!(read_connection(&FakeKeychain::default()).unwrap(), None);
        assert!(read_connection(&keychain_with("no es json")).is_err());
    }

    #[test]
    fn saving_keeps_the_secret_unless_a_new_one_is_given_and_every_other_data_of_the_entry() {
        let keychain = keychain_with(&KEPT.replace("\"client_secret\"", "\"otro_dato\":\"queda\",\"client_secret\""));
        let fields = ConnectionFields {
            api: " https://api.ejemplo.invalid/v2/ ".into(),
            issuer: "https://emisor.ejemplo.invalid/realms/prueba".into(),
            tenant: "organizacion-de-prueba".into(),
            client_id: "cliente-de-prueba".into(),
        };
        let view = write_connection(&keychain, &fields, None).unwrap();
        assert_eq!(view.fields.api, "https://api.ejemplo.invalid/v2");
        assert!(view.has_secret);
        let kept: Value = serde_json::from_str(&keychain.keys.borrow()[CONNECTION_KEY]).unwrap();
        assert_eq!(kept["client_secret"], "secreto-de-prueba");
        assert_eq!(kept["environment"], "ambiente-de-prueba");
        assert_eq!(kept["otro_dato"], "queda");
        assert_eq!(kept["api"], "https://api.ejemplo.invalid/v2");

        write_connection(&keychain, &fields, Some(" secreto-nuevo ")).unwrap();
        assert_eq!(read_connection(&keychain).unwrap().unwrap().client_secret, "secreto-nuevo");

        // A new connection needs its secret, and every field right.
        let empty = FakeKeychain::default();
        assert!(write_connection(&empty, &fields, None).unwrap_err().contains("secret"));
        let saved = write_connection(&empty, &fields, Some("secreto-de-prueba")).unwrap();
        assert_eq!(saved.environment, "");
        let kept: Value = serde_json::from_str(&empty.keys.borrow()[CONNECTION_KEY]).unwrap();
        for key in ["environment", "api", "issuer", "tenant", "client_id", "client_secret"] {
            assert!(kept.get(key).is_some(), "{key} is missing in {kept}");
        }
        let wrong = |change: fn(&mut ConnectionFields)| {
            let mut fields = fields.clone();
            change(&mut fields);
            write_connection(&empty, &fields, Some("s")).unwrap_err()
        };
        assert!(wrong(|f| f.api = "http://api.ejemplo.invalid/v1".into()).contains("https"));
        assert!(wrong(|f| f.issuer = "emisor".into()).contains("web address"));
        assert!(wrong(|f| f.tenant = " ".into()).contains("organization"));
        assert!(wrong(|f| f.client_id = "dos palabras".into()).contains("spaces"));
        // A keychain that does not keep it says so.
        assert!(write_connection(&FakeKeychain { forgetful: true, ..Default::default() }, &fields, Some("s")).is_err());
    }

    #[test]
    fn the_test_asks_for_a_permit_and_the_list_of_readings() {
        let core = FakeCore::start();
        core.with(|state| state.listed = 3);
        let keychain = keychain_with(&serde_json::to_string(&core.connection()).unwrap());
        let report = test_fields(&keychain, &fields_of(&core), None, quick());
        assert_eq!(report, ConnectionTest { ok: true, readings: Some(3), ..Default::default() });
        let seen = core.seen();
        assert_eq!(seen[0].path, "/realms/prueba/protocol/openid-connect/token");
        let form = String::from_utf8(seen[0].body.clone()).unwrap();
        assert!(form.contains("grant_type=client_credentials") && form.contains("scope=organization%3Aorganizacion-de-prueba"), "{form}");
        assert_eq!(seen[1].path, "/v1/tenant/document-readings?limit=100");
        assert_eq!(seen[1].header("x-tenant-id").as_deref(), Some("organizacion-de-prueba"));
        assert!(seen[1].header("authorization").unwrap().starts_with("Bearer permiso-"));
        // Nothing was kept.
        assert_eq!(read_connection(&keychain).unwrap().unwrap(), core.connection());
    }

    #[test]
    fn the_test_says_why_it_did_not_work_without_showing_the_secret() {
        let core = FakeCore::start();
        let keychain = keychain_with(&serde_json::to_string(&core.connection()).unwrap());
        let fields = fields_of(&core);

        let rejected = test_fields(&keychain, &fields, Some("secreto-equivocado"), quick());
        assert_eq!((rejected.ok, rejected.step.as_deref(), rejected.problem.as_deref()), (false, Some("permit"), Some("rejected")));
        assert_eq!(rejected.status, Some(401));
        assert!(!format!("{rejected:?}").contains("secreto"), "{rejected:?}");

        let other_tenant = ConnectionFields { tenant: "otra-organizacion".into(), ..fields.clone() };
        assert_eq!(test_fields(&keychain, &other_tenant, None, quick()).problem.as_deref(), Some("organization"));

        core.with(|state| state.list_status = Some(403));
        let forbidden = test_fields(&keychain, &fields, None, quick());
        assert_eq!((forbidden.step.as_deref(), forbidden.problem.as_deref()), (Some("readings"), Some("forbidden")));
        core.with(|state| state.list_status = Some(503));
        assert_eq!(test_fields(&keychain, &fields, None, quick()).problem.as_deref(), Some("unavailable"));

        let wrong_api = ConnectionFields { api: format!("{}/otra", core.url), ..fields.clone() };
        core.with(|state| state.list_status = None);
        assert_eq!(test_fields(&keychain, &wrong_api, None, quick()).problem.as_deref(), Some("not-found"));

        let missing = ConnectionFields { client_id: String::new(), ..fields.clone() };
        assert_eq!(test_fields(&keychain, &missing, None, quick()).step.as_deref(), Some("fields"));
        assert_eq!(test_fields(&FakeKeychain::default(), &fields, None, quick()).problem.as_deref(), Some("fields"));

        // Nobody answers at that address.
        let closed = ConnectionFields { issuer: "http://127.0.0.1:9/realms/prueba".into(), ..fields };
        assert_eq!(test_fields(&keychain, &closed, None, quick()).problem.as_deref(), Some("unreachable"));
    }
}
