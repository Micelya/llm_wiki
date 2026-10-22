//! The client of the API of Micelya Core (its contract: Core's
//! `packages/contracts/src/http/openapi.yaml`, 0.5.0-draft).
//!
//! Every call to the API carries an access permit (`Authorization:
//! Bearer`) and the organization (`X-Tenant-Id`). The permit is asked of
//! the issuer (Keycloak) with the technical user of the organization
//! (`client_credentials`, scope `organization:<tenant>`) and lasts an hour.
//! It is kept in memory, shared by every client of the same connection,
//! until a minute before it ends, or until the API refuses it; then it is
//! asked again. It is never written anywhere, nor shown.
//!
//! The client blocks: the base asks from its worker threads, and the
//! commands of Ajustes run it where blocking is allowed (`spawn_blocking`).
//! Every wait has a limit (`Timing`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use reqwest::blocking::{Client, RequestBuilder};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::CoreConnection;

/// How long the client waits.
#[derive(Debug, Clone)]
pub struct Timing {
    /// Each call to the issuer or the API.
    pub call: Duration,
    /// The upload of one page.
    pub upload: Duration,
    /// Between two looks at a reading in progress: Core does not tell when
    /// it ends.
    pub poll_every: Duration,
    /// The longest a reading of one page is waited for.
    pub reading_cap: Duration,
    /// Before each repetition of a call that failed for a moment (no
    /// answer, the limit of use, Core not available): as many repetitions
    /// as waits.
    pub retry_waits: Vec<Duration>,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            call: Duration::from_secs(30),
            upload: Duration::from_secs(120),
            poll_every: Duration::from_secs(5),
            reading_cap: Duration::from_secs(600),
            retry_waits: vec![Duration::from_secs(2), Duration::from_secs(5)],
        }
    }
}

/// Which part of Core a call goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The issuer of the access permits (Keycloak).
    Issuer,
    /// The API.
    Api,
    /// The storage a page is uploaded to.
    Upload,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Side::Issuer => "issuer",
            Side::Api => "API",
            Side::Upload => "upload",
        }
    }
}

/// Why a call to Core did not work. The messages never carry the secret,
/// the permit nor the address of an upload (it is signed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// No answer: no connection, a name that does not resolve, or nothing
    /// within `seconds`.
    NoAnswer { side: Side, timed_out: bool, seconds: u64, detail: String },
    /// It answered with an error.
    Refused { side: Side, status: u16, code: Option<String>, detail: String },
    /// It answered something its contract does not say.
    Unexpected { side: Side, detail: String },
}

impl CoreError {
    /// A failure of a moment: the same call may work a little later.
    pub fn is_transient(&self) -> bool {
        match self {
            CoreError::NoAnswer { .. } => true,
            CoreError::Refused { status, .. } => *status == 429 || (500..=599).contains(status),
            CoreError::Unexpected { .. } => false,
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            CoreError::Refused { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The error in words. The base tells a failure of Core from one of
    /// the document by them (`similar::classify::stops_the_run`): no
    /// connection, the credential rejected, the limit of use and no answer
    /// in time stop the run and are tried again later.
    pub fn message(&self) -> String {
        match self {
            CoreError::NoAnswer { side, timed_out: true, seconds, .. } => {
                format!("Micelya Core did not answer within {seconds} seconds ({})", side.name())
            }
            CoreError::NoAnswer { side, detail, .. } => {
                format!("Micelya Core cannot be reached ({}): no connection ({detail})", side.name())
            }
            CoreError::Refused { side, status, code, detail } => {
                let what = match code {
                    Some(code) => format!("{status} {code}"),
                    None => status.to_string(),
                };
                let detail = if detail.is_empty() { String::new() } else { format!(": {detail}") };
                match (side, *status) {
                    (_, 429) => format!("Micelya Core is at its usage limit ({what}){detail}"),
                    (_, 500..=599) => format!("Micelya Core is not available now: no connection with its service ({what}){detail}"),
                    (Side::Issuer, 400 | 401 | 403) | (Side::Api, 401 | 403) => {
                        format!("Micelya Core rejected the credential ({}, {what}){detail}", side.name())
                    }
                    _ => format!("Micelya Core refused the request ({}, {what}){detail}", side.name()),
                }
            }
            CoreError::Unexpected { side, detail } => format!("Micelya Core answered something unexpected ({}): {detail}", side.name()),
        }
    }
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

/// The longest part of an answer kept in an error.
const DETAIL_CHARS: usize = 300;

fn shortened(text: &str) -> String {
    let text = text.trim();
    match text.char_indices().nth(DETAIL_CHARS) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

/// The text between `<tag>` and `</tag>`.
fn xml_field<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let start = text.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = text[start..].find(&format!("</{tag}>"))? + start;
    Some(&text[start..end])
}

/// What an error answer says, and its code: a problem of the API (RFC
/// 9457), an error of the issuer (OAuth) or of the storage (XML). Only
/// those fields: an answer could echo what it was sent.
fn summary(body: &str) -> (Option<String>, String) {
    if let Ok(serde_json::Value::Object(answer)) = serde_json::from_str::<serde_json::Value>(body) {
        let field = |key: &str| answer.get(key).and_then(|v| v.as_str()).map(str::to_string);
        let code = field("code").or_else(|| field("error"));
        let detail = [field("title"), field("detail"), field("error_description")].into_iter().flatten().collect::<Vec<_>>().join(": ");
        return (code, shortened(&detail));
    }
    if let Some(code) = xml_field(body, "Code") {
        return (Some(code.to_string()), shortened(xml_field(body, "Message").unwrap_or("")));
    }
    (None, String::new())
}

/// A failed sending in words, with its causes, without the address.
fn transport_detail(error: reqwest::Error) -> String {
    let error = error.without_url();
    let mut parts = vec![error.to_string()];
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !parts.contains(&text) {
            parts.push(text);
        }
        source = cause.source();
    }
    shortened(&parts.join(": "))
}

/// The answer of a call: its body when it worked (2xx), why not otherwise.
pub(crate) fn answer(side: Side, seconds: u64, sent: reqwest::Result<reqwest::blocking::Response>) -> Result<String, CoreError> {
    let response = sent.map_err(|error| CoreError::NoAnswer {
        side,
        timed_out: error.is_timeout(),
        seconds,
        detail: transport_detail(error),
    })?;
    let status = response.status();
    let body = response.text().map_err(|error| CoreError::NoAnswer {
        side,
        timed_out: error.is_timeout(),
        seconds,
        detail: transport_detail(error),
    })?;
    if status.is_success() {
        return Ok(body);
    }
    let (code, detail) = summary(&body);
    Err(CoreError::Refused { side, status: status.as_u16(), code, detail })
}

pub(crate) fn parsed<T: DeserializeOwned>(side: Side, body: &str) -> Result<T, CoreError> {
    serde_json::from_str(body).map_err(|e| CoreError::Unexpected { side, detail: shortened(&e.to_string()) })
}

/// An access permit, kept while it lasts.
struct Permit {
    token: String,
    until: Instant,
}

/// The permits of every connection in use, by `permit_key`.
fn permits() -> &'static Mutex<HashMap<String, Permit>> {
    static PERMITS: OnceLock<Mutex<HashMap<String, Permit>>> = OnceLock::new();
    PERMITS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The same issuer, organization, client and secret share a permit; the
/// secret goes in only as a hash.
fn permit_key(connection: &CoreConnection) -> String {
    let mut hasher = Sha256::new();
    for part in [&connection.issuer, &connection.tenant, &connection.client_id, &connection.client_secret] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    crate::source_volume::file_hash::hex(&hasher.finalize())
}

#[derive(Deserialize)]
struct PermitAnswer {
    access_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// One page of the list of readings (`listDocumentReadings`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingList {
    #[serde(default)]
    pub items: Vec<ReadingRequest>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// A reading of a document, as Core tells how it stands
/// (`DocumentReadingRequest`). It never carries the text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingRequest {
    pub id: String,
    /// `in_progress`, `succeeded`, `failed`, `expired` or `cancelled`.
    pub status: String,
    /// While in progress: `awaiting_input`, `queued`, `running` or
    /// `awaiting_provider`.
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub requested_at: Option<String>,
    /// Why it did not end well, in words for a person.
    #[serde(default)]
    pub failure: Option<String>,
    #[serde(default)]
    pub filename: String,
}

pub struct CoreClient {
    connection: CoreConnection,
    http: Client,
    timing: Timing,
}

impl CoreClient {
    pub fn new(connection: CoreConnection, timing: Timing) -> Result<Self, CoreError> {
        let http = crate::proxy::configure_blocking_http_client(Client::builder().timeout(timing.call))
            .build()
            .map_err(|e| CoreError::Unexpected { side: Side::Api, detail: transport_detail(e) })?;
        Ok(Self { connection, http, timing })
    }

    pub fn timing(&self) -> &Timing {
        &self.timing
    }

    fn seconds(&self) -> u64 {
        self.timing.call.as_secs()
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.connection.api.trim_end_matches('/'), path.trim_start_matches('/'))
    }

    /// Asks the issuer for a new permit and keeps it.
    pub fn new_permit(&self) -> Result<String, CoreError> {
        let url = format!("{}/protocol/openid-connect/token", self.connection.issuer.trim_end_matches('/'));
        let scope = format!("organization:{}", self.connection.tenant);
        let form = [
            ("grant_type", "client_credentials"),
            ("client_id", self.connection.client_id.as_str()),
            ("client_secret", self.connection.client_secret.as_str()),
            ("scope", scope.as_str()),
        ];
        let body = answer(Side::Issuer, self.seconds(), self.http.post(url).form(&form).send())?;
        let permit: PermitAnswer = parsed(Side::Issuer, &body)?;
        if permit.access_token.is_empty() {
            return Err(CoreError::Unexpected { side: Side::Issuer, detail: "no access permit in the answer".to_string() });
        }
        let lasts = permit.expires_in.unwrap_or(300);
        // Asked again a minute before it ends (half its life, if shorter).
        let until = Instant::now() + Duration::from_secs(lasts - (lasts / 2).min(60));
        permits()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(permit_key(&self.connection), Permit { token: permit.access_token.clone(), until });
        Ok(permit.access_token)
    }

    /// The permit kept while it lasts, or a new one.
    fn permit(&self) -> Result<String, CoreError> {
        let kept = permits()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&permit_key(&self.connection))
            .filter(|permit| permit.until > Instant::now())
            .map(|permit| permit.token.clone());
        match kept {
            Some(token) => Ok(token),
            None => self.new_permit(),
        }
    }

    fn forget_permit(&self) {
        permits().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(&permit_key(&self.connection));
    }

    /// Repeats `call` after each wait of `retry_waits` while it fails for a
    /// moment; `before_again` runs before each repetition.
    pub(crate) fn repeating<T>(
        &self,
        mut call: impl FnMut() -> Result<T, CoreError>,
        mut before_again: impl FnMut(&CoreError),
    ) -> Result<T, CoreError> {
        let mut waits = self.timing.retry_waits.iter();
        loop {
            match call() {
                Err(error) if error.is_transient() => match waits.next() {
                    Some(wait) => {
                        before_again(&error);
                        std::thread::sleep(*wait);
                    }
                    None => return Err(error),
                },
                other => return other,
            }
        }
    }

    /// One call to the API with the permit and the organization. A permit
    /// the API refuses is asked again, once.
    pub(crate) fn call(&self, method: Method, path: &str, build: impl Fn(RequestBuilder) -> RequestBuilder) -> Result<String, CoreError> {
        let mut renewed = false;
        loop {
            let permit = self.permit()?;
            let request = self
                .http
                .request(method.clone(), self.api_url(path))
                .bearer_auth(&permit)
                .header("X-Tenant-Id", &self.connection.tenant);
            match answer(Side::Api, self.seconds(), build(request).send()) {
                Err(CoreError::Refused { status: 401, .. }) if !renewed => {
                    self.forget_permit();
                    renewed = true;
                }
                other => return other,
            }
        }
    }

    /// The newest readings this credential sees, `limit` at most (100 is
    /// the most the API gives), after `cursor` when given.
    pub fn list_readings(&self, limit: u32, cursor: Option<&str>) -> Result<ReadingList, CoreError> {
        let mut path = format!("tenant/document-readings?limit={limit}");
        if let Some(cursor) = cursor {
            path.push_str(&format!("&cursor={}", urlencoding(cursor)));
        }
        let body = self.repeating(|| self.call(Method::GET, &path, |r| r), |_| {})?;
        parsed(Side::Api, &body)
    }
}

/// `text` for a query of an address.
fn urlencoding(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::fake::FakeCore;
    use super::*;

    #[test]
    fn the_permit_is_kept_until_it_ends_and_asked_again_when_it_ends_or_is_refused() {
        let core = FakeCore::start();
        let client = CoreClient::new(core.connection(), FakeCore::timing()).unwrap();
        client.list_readings(10, None).unwrap();
        client.list_readings(10, None).unwrap();
        // Another client of the same connection uses the same permit.
        CoreClient::new(core.connection(), FakeCore::timing()).unwrap().list_readings(10, None).unwrap();
        assert_eq!(core.with(|state| state.permits_given), 1);

        // It ends: a new one.
        core.with(|state| state.permit_lasts = 1);
        client.new_permit().unwrap();
        assert_eq!(core.with(|state| state.permits_given), 2);
        std::thread::sleep(Duration::from_millis(1100));
        client.list_readings(10, None).unwrap();
        assert_eq!(core.with(|state| state.permits_given), 3);

        // The API refuses it: a new one, once.
        core.with(|state| {
            state.permit_lasts = 3600;
            state.refuse_permits_given_before = state.permits_given + 1;
        });
        client.list_readings(10, None).unwrap();
        assert_eq!(core.with(|state| state.permits_given), 4);
        let lists = core.seen().into_iter().filter(|s| s.path.starts_with("/v1/")).count();
        assert_eq!(lists, 6);
    }

    #[test]
    fn a_call_that_fails_for_a_moment_is_repeated_and_the_errors_say_what_happened() {
        let core = FakeCore::start();
        let client = CoreClient::new(core.connection(), FakeCore::timing()).unwrap();
        core.with(|state| state.list_failures = vec![503, 429]);
        assert!(client.list_readings(5, None).is_ok());
        core.with(|state| state.list_failures = vec![503, 503, 503]);
        let error = client.list_readings(5, None).unwrap_err();
        assert_eq!(error.status(), Some(503));
        assert!(error.message().contains("no connection"), "{error}");
        assert!(crate::docbase::similar::classify::stops_the_run(&error.message()));

        let unreachable = CoreError::NoAnswer { side: Side::Api, timed_out: false, seconds: 30, detail: "refused".into() };
        assert!(crate::docbase::similar::classify::stops_the_run(&unreachable.message()));
        let late = CoreError::NoAnswer { side: Side::Api, timed_out: true, seconds: 30, detail: String::new() };
        assert_eq!(late.message(), "Micelya Core did not answer within 30 seconds (API)");
        let rejected = CoreError::Refused { side: Side::Issuer, status: 401, code: Some("invalid_client".into()), detail: "x".into() };
        assert!(crate::docbase::similar::classify::stops_the_run(&rejected.message()));
        let refused = CoreError::Refused { side: Side::Api, status: 422, code: None, detail: "no".into() };
        assert!(!crate::docbase::similar::classify::stops_the_run(&refused.message()));
    }

    #[test]
    fn an_answer_in_error_keeps_only_what_it_says_and_its_code() {
        let problem = r#"{"type":"about:blank","title":"No se admite","status":422,"code":"document_reading.filename_not_allowed","detail":"El nombre no se admite","traceId":"t"}"#;
        assert_eq!(
            summary(problem),
            (Some("document_reading.filename_not_allowed".to_string()), "No se admite: El nombre no se admite".to_string())
        );
        let oauth = r#"{"error":"invalid_client","error_description":"Invalid client credentials"}"#;
        assert_eq!(summary(oauth), (Some("invalid_client".to_string()), "Invalid client credentials".to_string()));
        let storage = "<Error><Code>SignatureDoesNotMatch</Code><Message>No coincide</Message><StringToSign>firmado</StringToSign></Error>";
        assert_eq!(summary(storage), (Some("SignatureDoesNotMatch".to_string()), "No coincide".to_string()));
        assert_eq!(summary("cualquier otra cosa"), (None, String::new()));
        assert_eq!(urlencoding("a b/c"), "a%20b%2Fc");
    }
}
