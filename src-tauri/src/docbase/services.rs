//! The registry of the AI services (P-141, D-443): which engine each
//! service uses, with which model and reasoning, and an alternative engine.
//!
//! Three services: asking the model (the reading of the base and the chat),
//! recognizing the text of scans, and researching on the internet. The
//! registry is one setting of the base (`docbase_setting`, `ai-services`):
//! the base reads it at every request, so a change made in Ajustes is used
//! by the next question without the screens having to push anything. The
//! data to talk to a provider (its address, its key) is not here: it is a
//! shared connection, kept by the app (its key in the system keychain), and
//! a choice only names it.
//!
//! The engines a service can choose, what each one says it can do, where it
//! processes what it is sent and what its provider keeps are a catalog
//! (`resources/catalog/ai-engines.json`). The base runs by itself only the
//! engines marked `inBase` (today Codex, and Micelya Core for the scans); a
//! service set to another engine is, for the base, a service without an
//! engine, as before the registry. An engine whose model the person does
//! not choose (Core says which one reads) is saved with the one of the
//! catalog (`defaultModel`).
//!
//! Before the registry existed the base kept its two engines apart
//! (`ask-engine`, `recognition-engine`); until the registry is saved they
//! are read from there, so opening the app with an old base changes
//! nothing. The contracts (`DocumentAsker`, `TextRecognizer`) and the name
//! each answer is saved with (`engine_id`) do not change.

use std::sync::OnceLock;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{sql_err, DocBase};
use crate::source_volume::asking::{reasoning_level, AskConfig};
use crate::source_volume::recognition::RecognitionConfig;
use crate::source_volume::recognition_local;

/// Where the registry is kept (`docbase_setting`).
const AI_SERVICES_SETTING: &str = "ai-services";
/// Where the base kept the engine of its questions before the registry.
const LEGACY_ASK_SETTING: &str = "ask-engine";
/// Where the base kept the engine that recognizes scans before the registry.
const LEGACY_RECOGNITION_SETTING: &str = "recognition-engine";

const CATALOG: &str = include_str!("../../resources/catalog/ai-engines.json");

/// The three services.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Ask,
    Recognize,
    Research,
}

impl Service {
    pub fn id(self) -> &'static str {
        match self {
            Service::Ask => "ask",
            Service::Recognize => "recognize",
            Service::Research => "research",
        }
    }
}

/// One engine chosen for a service.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineChoice {
    /// The engine, as the catalog names it (`codex-cli`, `tavily`…).
    pub engine: String,
    /// The shared connection it uses (the provider's data, the same for
    /// every service that chooses it); empty when it is the engine's own.
    #[serde(default)]
    pub connection: String,
    #[serde(default)]
    pub model: String,
    /// `low`, `medium` or `high` for an engine that offers it; empty leaves
    /// the model's own default (D-274).
    #[serde(default)]
    pub reasoning: String,
}

/// What a service uses: its engine and, optionally, an alternative for
/// what the first one does not do well.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceChoice {
    #[serde(default)]
    pub engine: Option<EngineChoice>,
    #[serde(default)]
    pub alternative: Option<EngineChoice>,
}

/// The registry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiServices {
    #[serde(default)]
    pub ask: ServiceChoice,
    #[serde(default)]
    pub recognize: ServiceChoice,
    #[serde(default)]
    pub research: ServiceChoice,
}

impl AiServices {
    pub fn service(&self, service: Service) -> &ServiceChoice {
        match service {
            Service::Ask => &self.ask,
            Service::Recognize => &self.recognize,
            Service::Research => &self.research,
        }
    }
}

/// What the screens get: the registry, whether it was ever saved (before
/// that it comes from the base's old settings, and the app completes it
/// from its own), and the catalog of engines.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiServicesView {
    pub services: AiServices,
    pub saved: bool,
    pub catalog: serde_json::Value,
}

/// One entry of the catalog, as far as the base needs it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogEngine {
    service: String,
    id: String,
    #[serde(default)]
    in_base: bool,
    #[serde(default)]
    reasoning: bool,
    /// The model it is saved with when none is given.
    #[serde(default)]
    default_model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalog {
    engines: Vec<CatalogEngine>,
}

fn catalog_value() -> &'static serde_json::Value {
    static VALUE: OnceLock<serde_json::Value> = OnceLock::new();
    VALUE.get_or_init(|| serde_json::from_str(CATALOG).expect("the catalog of AI engines is valid JSON"))
}

fn catalog() -> &'static Catalog {
    static PARSED: OnceLock<Catalog> = OnceLock::new();
    PARSED.get_or_init(|| serde_json::from_value(catalog_value().clone()).expect("the catalog of AI engines has engines"))
}

fn catalog_engine(service: Service, engine: &str) -> Option<&'static CatalogEngine> {
    catalog().engines.iter().find(|entry| entry.service == service.id() && entry.id == engine)
}

/// Whether the base can run this engine by itself for this service.
pub fn runs_in_base(service: Service, engine: &str) -> bool {
    catalog_engine(service, engine).is_some_and(|entry| entry.in_base)
}

fn setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(sql_err)
}

/// The registry as saved, `None` before anyone saved it. One that cannot
/// be read counts as not saved, so the base goes on with its old settings.
fn saved(conn: &Connection) -> Result<Option<AiServices>, String> {
    Ok(setting(conn, AI_SERVICES_SETTING)?.and_then(|value| serde_json::from_str(&value).ok()))
}

/// The registry from the settings the base had before it: the engine of
/// the questions and the one that recognizes scans.
fn legacy(conn: &Connection) -> Result<AiServices, String> {
    let ask: Option<AskConfig> = setting(conn, LEGACY_ASK_SETTING)?.and_then(|v| serde_json::from_str(&v).ok());
    let recognize: Option<RecognitionConfig> =
        setting(conn, LEGACY_RECOGNITION_SETTING)?.and_then(|v| serde_json::from_str(&v).ok());
    Ok(AiServices {
        ask: ServiceChoice {
            engine: ask.map(|c| EngineChoice { connection: c.engine.clone(), engine: c.engine, model: c.model, reasoning: c.reasoning }),
            alternative: None,
        },
        recognize: ServiceChoice {
            engine: recognize.map(|c| EngineChoice { connection: c.engine.clone(), engine: c.engine, model: c.model, reasoning: String::new() }),
            alternative: None,
        },
        research: ServiceChoice::default(),
    })
}

/// The registry, and whether it was ever saved.
pub(crate) fn ai_services(conn: &Connection) -> Result<(AiServices, bool), String> {
    match saved(conn)? {
        Some(services) => Ok((services, true)),
        None => Ok((legacy(conn)?, false)),
    }
}

/// The engine the base asks with: the one of «Preguntar al modelo» when the
/// base can run it.
pub(crate) fn ask_engine(conn: &Connection) -> Result<Option<AskConfig>, String> {
    let (services, _) = ai_services(conn)?;
    Ok(services
        .ask
        .engine
        .filter(|choice| runs_in_base(Service::Ask, &choice.engine))
        .map(|choice| AskConfig { engine: choice.engine, model: choice.model, reasoning: choice.reasoning }))
}

/// The engine that recognizes scans: the one of «Reconocer escaneos» when
/// the base can run it.
pub(crate) fn recognition_engine(conn: &Connection) -> Result<Option<RecognitionConfig>, String> {
    let (services, _) = ai_services(conn)?;
    let chosen = services.recognize.engine.filter(|choice| runs_in_base(Service::Recognize, &choice.engine));
    Ok(match chosen {
        // The engine of this computer, only while its models are there.
        Some(choice) if choice.engine == recognition_local::ENGINE => local_recognition_engine(),
        Some(choice) => Some(RecognitionConfig { engine: choice.engine, model: choice.model }),
        // None chosen: the one of this computer, by default (D-558).
        None => local_recognition_engine(),
    })
}

/// The engine of this computer (D-558), when its models are there. It
/// reads the new scans in the background, the automatic reading on or off
/// (`recognize.rs`): no model in the cloud is asked.
pub(crate) fn local_recognition_engine() -> Option<RecognitionConfig> {
    recognition_local::unavailable()
        .is_none()
        .then(|| RecognitionConfig { engine: recognition_local::ENGINE.to_string(), model: recognition_local::catalog().model_set.clone() })
}

/// The catalog for the screens: the engine of this computer says why it is
/// not available when its models are missing (`unavailable`).
fn catalog_for_screens() -> serde_json::Value {
    let mut value = catalog_value().clone();
    if let Some(why) = recognition_local::unavailable() {
        if let Some(engines) = value.get_mut("engines").and_then(|engines| engines.as_array_mut()) {
            for entry in engines.iter_mut().filter(|entry| entry["id"] == recognition_local::ENGINE) {
                entry["unavailable"] = serde_json::Value::String(why.clone());
            }
        }
    }
    value
}

/// Why a choice cannot be saved, or `None`.
fn invalid_choice(service: Service, choice: &EngineChoice) -> Option<String> {
    let Some(entry) = catalog_engine(service, &choice.engine) else {
        return Some(format!("Unknown engine '{}' for the service '{}'", choice.engine, service.id()));
    };
    if !choice.reasoning.trim().is_empty() {
        if !entry.reasoning {
            return Some(format!("The engine '{}' takes no reasoning level", choice.engine));
        }
        if let Err(error) = reasoning_level(&choice.reasoning) {
            return Some(error);
        }
    }
    None
}

fn normalized(service: Service, choice: EngineChoice) -> EngineChoice {
    let engine = choice.engine.trim().to_string();
    let mut model = choice.model.trim().to_string();
    if model.is_empty() {
        if let Some(default) = catalog_engine(service, &engine).and_then(|entry| entry.default_model.clone()) {
            model = default;
        }
    }
    EngineChoice {
        engine,
        connection: choice.connection.trim().to_string(),
        model,
        reasoning: choice.reasoning.trim().to_lowercase(),
    }
}

fn store(conn: &Connection, services: &AiServices) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
        params![AI_SERVICES_SETTING, serde_json::to_string(services).map_err(|e| e.to_string())?],
    )
    .map_err(sql_err)?;
    Ok(())
}

impl DocBase {
    /// The registry for the screens, with the catalog.
    pub fn ai_services(&self) -> Result<AiServicesView, String> {
        let (services, saved) = ai_services(&self.conn())?;
        Ok(AiServicesView { services, saved, catalog: catalog_for_screens() })
    }

    /// Saves the whole registry; the next request uses it. Every engine
    /// must be one of the catalog for its service.
    pub fn set_ai_services(&self, services: AiServices) -> Result<(), String> {
        let mut checked = AiServices::default();
        for service in [Service::Ask, Service::Recognize, Service::Research] {
            let given = services.service(service).clone();
            let engine = given.engine.map(|c| normalized(service, c)).filter(|c| !c.engine.is_empty());
            let alternative = given.alternative.map(|c| normalized(service, c)).filter(|c| !c.engine.is_empty());
            for choice in engine.iter().chain(alternative.iter()) {
                if let Some(error) = invalid_choice(service, choice) {
                    return Err(error);
                }
            }
            let choice = ServiceChoice { engine, alternative };
            match service {
                Service::Ask => checked.ask = choice,
                Service::Recognize => checked.recognize = choice,
                Service::Research => checked.research = choice,
            }
        }
        store(&self.conn(), &checked)
    }

    /// Changes only the engine of one service, keeping the rest of the
    /// registry (and, before it was saved, what the base had). The app
    /// saves the whole registry (`set_ai_services`); these are for tests.
    #[cfg(test)]
    fn set_engine_of(&self, service: Service, engine: Option<EngineChoice>) -> Result<(), String> {
        let (mut services, _) = ai_services(&self.conn())?;
        let target = match service {
            Service::Ask => &mut services.ask,
            Service::Recognize => &mut services.recognize,
            Service::Research => &mut services.research,
        };
        target.engine = engine;
        self.set_ai_services(services)
    }

    /// The engine of «Preguntar al modelo»; none clears it. The connection
    /// and anything else of the registry stay.
    #[cfg(test)]
    pub fn set_ask_engine(&self, engine: Option<&AskConfig>) -> Result<(), String> {
        let (services, _) = ai_services(&self.conn())?;
        let choice = engine.map(|config| EngineChoice {
            engine: config.engine.clone(),
            connection: services
                .ask
                .engine
                .as_ref()
                .filter(|current| current.engine == config.engine)
                .map(|current| current.connection.clone())
                .unwrap_or_else(|| config.engine.clone()),
            model: config.model.clone(),
            reasoning: config.reasoning.clone(),
        });
        self.set_engine_of(Service::Ask, choice)
    }

    /// The engine of «Reconocer escaneos»; none clears it.
    #[cfg(test)]
    pub fn set_recognition_engine(&self, engine: Option<&RecognitionConfig>) -> Result<(), String> {
        let choice = engine.map(|config| EngineChoice {
            engine: config.engine.clone(),
            connection: config.engine.clone(),
            model: config.model.clone(),
            reasoning: String::new(),
        });
        self.set_engine_of(Service::Recognize, choice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_volume::asking::engine_id;

    fn codex(model: &str, reasoning: &str) -> EngineChoice {
        EngineChoice { engine: "codex-cli".into(), connection: "codex-cli".into(), model: model.into(), reasoning: reasoning.into() }
    }

    fn put(base: &DocBase, key: &str, value: &str) {
        base.conn()
            .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![key, value])
            .unwrap();
    }

    #[test]
    fn the_catalog_names_every_service_and_codex_runs_in_the_base() {
        let services: Vec<String> = catalog_value()["services"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(services, ["ask", "recognize", "research"]);
        assert!(runs_in_base(Service::Ask, "codex-cli"));
        assert!(runs_in_base(Service::Recognize, "codex-cli"));
        assert!(runs_in_base(Service::Recognize, "core"));
        assert!(!runs_in_base(Service::Ask, "core"));
        assert!(!runs_in_base(Service::Ask, "openai"));
        assert!(!runs_in_base(Service::Recognize, "openai"));
        assert!(!runs_in_base(Service::Research, "tavily"));
        for engine in catalog_value()["engines"].as_array().unwrap() {
            for field in ["service", "id", "name", "kind", "processing", "retention"] {
                assert!(engine[field].as_str().is_some_and(|v| !v.is_empty()), "{engine} has no {field}");
            }
            let service = engine["service"].as_str().unwrap();
            assert!(services.iter().any(|s| s == service), "{engine}");
        }
        // Every engine the base runs is one its contracts know.
        for entry in catalog().engines.iter().filter(|e| e.in_base) {
            match entry.service.as_str() {
                "ask" => assert!(crate::source_volume::asking::asker_for(&AskConfig { engine: entry.id.clone(), ..Default::default() }).is_ok()),
                "recognize" => assert!(crate::source_volume::recognition::recognizer_for(&RecognitionConfig {
                    engine: entry.id.clone(),
                    model: String::new()
                })
                .is_ok()),
                other => panic!("the base runs nothing for {other}"),
            }
        }
    }

    #[test]
    fn an_old_base_keeps_its_engines_until_the_registry_is_saved() {
        let base = DocBase::in_memory().unwrap();
        assert_eq!(ask_engine(&base.conn()).unwrap(), None);
        assert_eq!(recognition_engine(&base.conn()).unwrap(), None);
        put(&base, LEGACY_ASK_SETTING, r#"{"engine":"codex-cli","model":"m","reasoning":"high"}"#);
        put(&base, LEGACY_RECOGNITION_SETTING, r#"{"engine":"codex-cli","model":"r"}"#);
        let ask = ask_engine(&base.conn()).unwrap().unwrap();
        assert_eq!(engine_id(&ask), "codex-cli:m@high");
        let recognize = recognition_engine(&base.conn()).unwrap().unwrap();
        assert_eq!(crate::source_volume::recognition::engine_id(&recognize).unwrap(), "codex-cli:r");
        let view = base.ai_services().unwrap();
        assert!(!view.saved);
        assert_eq!(view.services.ask.engine, Some(codex("m", "high")));
        assert_eq!(view.services.recognize.engine, Some(codex("r", "")));
        assert_eq!(view.services.research, ServiceChoice::default());

        // Once saved, the registry is what counts.
        let mut services = view.services.clone();
        services.ask.engine = Some(codex("otro", "low"));
        services.recognize.alternative = Some(codex("r2", ""));
        services.research.engine = Some(EngineChoice { engine: "tavily".into(), connection: "tavily".into(), ..Default::default() });
        base.set_ai_services(services.clone()).unwrap();
        let view = base.ai_services().unwrap();
        assert!(view.saved);
        assert_eq!(view.services, services);
        assert_eq!(engine_id(&ask_engine(&base.conn()).unwrap().unwrap()), "codex-cli:otro@low");
        // The old settings are left as they were.
        assert!(setting(&base.conn(), LEGACY_ASK_SETTING).unwrap().unwrap().contains("\"m\""));
    }

    #[test]
    fn changing_the_engine_of_a_service_is_used_by_the_next_request() {
        let base = DocBase::in_memory().unwrap();
        base.set_ai_services(AiServices {
            ask: ServiceChoice { engine: Some(codex("m", "medium")), alternative: None },
            recognize: ServiceChoice { engine: Some(codex("m", "")), alternative: None },
            ..Default::default()
        })
        .unwrap();
        base.set_background_reading(true).unwrap();
        assert_eq!(engine_id(&base.background_ask_engine().unwrap().unwrap()), "codex-cli:m@medium");
        let mut services = base.ai_services().unwrap().services;
        services.ask.engine = Some(codex("n", "high"));
        base.set_ai_services(services.clone()).unwrap();
        assert_eq!(engine_id(&base.background_ask_engine().unwrap().unwrap()), "codex-cli:n@high");
        // An engine the base cannot run leaves it without one, as before.
        services.ask.engine = Some(EngineChoice { engine: "openai".into(), connection: "openai".into(), model: "x".into(), ..Default::default() });
        base.set_ai_services(services.clone()).unwrap();
        assert_eq!(base.background_ask_engine().unwrap(), None);
        assert_eq!(ask_engine(&base.conn()).unwrap(), None);
        // Recognition went on with its own.
        assert_eq!(recognition_engine(&base.conn()).unwrap().unwrap().model, "m");
        services.recognize.engine = None;
        base.set_ai_services(services).unwrap();
        assert_eq!(recognition_engine(&base.conn()).unwrap(), None);
    }

    #[test]
    fn only_engines_of_the_catalog_and_offered_levels_are_saved() {
        let base = DocBase::in_memory().unwrap();
        let with = |service: Service, choice: EngineChoice| {
            let mut services = AiServices::default();
            let target = ServiceChoice { engine: Some(choice), alternative: None };
            match service {
                Service::Ask => services.ask = target,
                Service::Recognize => services.recognize = target,
                Service::Research => services.research = target,
            }
            base.set_ai_services(services)
        };
        assert!(with(Service::Ask, EngineChoice { engine: "nada".into(), ..Default::default() }).is_err());
        assert!(with(Service::Recognize, EngineChoice { engine: "openai".into(), ..Default::default() }).is_err());
        assert!(with(Service::Research, EngineChoice { engine: "codex-cli".into(), ..Default::default() }).is_err());
        assert!(with(Service::Ask, codex("m", "ultra")).is_err());
        assert!(with(Service::Ask, EngineChoice { engine: "openai".into(), reasoning: "high".into(), ..Default::default() }).is_err());
        assert!(with(Service::Ask, codex(" m ", " High ")).is_ok());
        assert_eq!(base.ai_services().unwrap().services.ask.engine, Some(codex("m", "high")));
        // An empty engine is no engine.
        assert!(with(Service::Research, EngineChoice::default()).is_ok());
        assert_eq!(base.ai_services().unwrap().services.research.engine, None);
    }

    #[test]
    fn micelya_core_recognizes_scans_with_its_own_model_and_its_shared_connection() {
        let base = DocBase::in_memory().unwrap();
        let core = EngineChoice { engine: "core".into(), connection: crate::micelya_core::CONNECTION_ID.into(), ..Default::default() };
        base.set_ai_services(AiServices { recognize: ServiceChoice { engine: Some(core), alternative: None }, ..Default::default() })
            .unwrap();
        let saved = base.ai_services().unwrap().services.recognize.engine.unwrap();
        assert_eq!((saved.model.as_str(), saved.connection.as_str()), ("textract-1.0", "micelya-core"));
        let config = recognition_engine(&base.conn()).unwrap().unwrap();
        assert_eq!(crate::source_volume::recognition::engine_id(&config).unwrap(), "core:textract-1.0");
        let entry = catalog_value()["engines"].as_array().unwrap().iter().find(|e| e["service"] == "recognize" && e["id"] == "core").unwrap();
        assert_eq!(entry["connection"], crate::micelya_core::CONNECTION_ID);
        for words in ["Oregón (Estados Unidos)", "pagina-N.jpg"] {
            assert!(entry["processing"].as_str().unwrap().contains(words), "{entry}");
        }
        for words in ["Textract guarda el resultado 7 días", "Desktop borra cada lectura en Core apenas la guarda"] {
            assert!(entry["retention"].as_str().unwrap().contains(words), "{entry}");
        }
        // Core reads scans, it does not answer questions (yet).
        let mut ask = AiServices::default();
        ask.ask.engine = Some(EngineChoice { engine: "core".into(), ..Default::default() });
        assert!(base.set_ai_services(ask).is_err());
    }

    #[test]
    fn setting_one_engine_keeps_the_rest_of_the_registry() {
        let base = DocBase::in_memory().unwrap();
        put(&base, LEGACY_RECOGNITION_SETTING, r#"{"engine":"codex-cli","model":"r"}"#);
        // Before the registry: the recognition the base had is kept.
        base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "m".into(), reasoning: "low".into() })).unwrap();
        let view = base.ai_services().unwrap();
        assert!(view.saved);
        assert_eq!(view.services.ask.engine, Some(codex("m", "low")));
        assert_eq!(view.services.recognize.engine, Some(codex("r", "")));
        let mut services = view.services;
        services.ask.alternative = Some(codex("alt", ""));
        services.ask.engine.as_mut().unwrap().connection = "codex-otra".into();
        base.set_ai_services(services).unwrap();
        base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "m2".into(), reasoning: String::new() })).unwrap();
        let services = base.ai_services().unwrap().services;
        let ask = services.ask.engine.unwrap();
        assert_eq!((ask.model.as_str(), ask.connection.as_str()), ("m2", "codex-otra"));
        assert_eq!(services.ask.alternative, Some(codex("alt", "")));
        base.set_recognition_engine(None).unwrap();
        assert_eq!(recognition_engine(&base.conn()).unwrap(), None);
        assert_eq!(engine_id(&ask_engine(&base.conn()).unwrap().unwrap()), "codex-cli:m2");
        base.set_ask_engine(None).unwrap();
        assert_eq!(ask_engine(&base.conn()).unwrap(), None);
    }
}
