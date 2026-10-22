//! What the screens can ask the document base.
//!
//! In the private space everything is visible, so every command passes
//! `Visible::All`. The shared space will compute it per person here,
//! without the queries below changing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use super::bridge;
use super::knowledge::extract::{KnowledgePlan, KnowledgeReport};
use super::knowledge::articles::{KnowledgeArticle, KnowledgeNote, KnowledgeOverview, PageObject, SavedQuery};
use super::knowledge::decide::{DecisionView, IdentityDecision};
use super::knowledge::entities::{DoubtView, EntityDetail, EntitySummary};
use super::knowledge::pages::ProjectReport;
use super::knowledge::research::{Research, ResearchSource};
use super::knowledge::resolve::ResolveReport;
use super::knowledge::screens::{DocumentEntities, KnowledgeAttention, MatterDetail, MatterSummary};
use super::knowledge::extract::{RereadPlan, RereadTarget};
use super::knowledge::{DocumentKnowledge, KnowledgeStatus};
use super::labels::{Label, LabelGroup, LabelRule};
use super::origins::Origin;
use super::progress::{FolderCount, Progress, Scope, StageItems};
use super::recognize::RecognitionRequest;
use super::similar::{self, Decision, OpenPair, PairDecision, SimilarDocument, SimilarGroup, SimilarPending};
use super::similar::classify::{DecidedType, DocumentType, ProposedType, RunReport, TypeDecision, TypesPlan};
use super::similar::data::Datum;
use super::similar::types::RelationDef;
use crate::source_volume::asking::{asker_for, AskConfig};
use super::services::{AiServices, AiServicesView};
use super::query::{Activity, DocumentDetail, DocumentFilter, DocumentPage, Health, LocationRef, OriginSummary};
use super::questions::{AskedQuestion, ModelQuestion};
use super::tree::FolderListing;
use super::worker::{self, Wake};
use super::{DocBase, Visible, PRIVATE_DB_FILE};

/// Event the screens listen to in order to refresh.
pub const CHANGED_EVENT: &str = "docbase://changed";

/// The base of the private space, or why it could not be opened.
pub struct DocBaseState {
    base: Result<Arc<DocBase>, String>,
    wake: Arc<Wake>,
    /// Ends the wait of the questions to the model (a scan to recognize).
    ask_wake: Arc<Wake>,
}

impl DocBaseState {
    fn base(&self) -> Result<&Arc<DocBase>, String> {
        self.base
            .as_ref()
            .map_err(|error| format!("La base documental no está disponible: {error}"))
    }

    /// The base, for the parts of the app that bring documents into it
    /// (the portals).
    pub(crate) fn shared(&self) -> Result<Arc<DocBase>, String> {
        self.base().cloned()
    }

    /// Asks the worker for a sweep of every origin now.
    pub(crate) fn request_sweep(&self) {
        self.wake.request_sweep();
    }
}

/// Opens the private space's base in the app's local data folder and
/// starts its background worker.
pub fn start(app: &tauri::AppHandle) {
    let wake = Arc::new(Wake::default());
    let ask_wake = Arc::new(Wake::default());
    // The catalog of document types can be replaced by a file of the same
    // shape in the data folder (similar/types.rs).
    if let Ok(dir) = app.path().app_data_dir() {
        similar::types::install_from(&dir.join("docbase/document-types.json"));
        // And so can the vocabulary of the knowledge layer.
        super::knowledge::vocabulary::install_from(&dir.join("docbase/knowledge.json"));
    }
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no se encontró la carpeta de datos de la app: {e}"))
        .and_then(|dir| DocBase::open(&dir.join(PRIVATE_DB_FILE)))
        .map(Arc::new);
    match &base {
        Ok(base) => {
            // Once, before anything projects: what a person wrote in the
            // old wiki's files comes into the base (D-403).
            match base.import_wiki_files() {
                Ok(report) if !report.skipped => eprintln!("[docbase] imported from the wiki's files: {report:?}"),
                Ok(_) => {}
                Err(error) => eprintln!("[docbase] the wiki's files could not be imported: {error}"),
            }
            bridge::share(base.clone());
            let handle = app.clone();
            worker::spawn(base.clone(), wake.clone(), move || {
                let _ = handle.emit(CHANGED_EVENT, ());
            });
            let handle = app.clone();
            let pages = app.clone();
            // Pages written again in the base change what the screens show.
            worker::spawn_asking(
                base.clone(),
                ask_wake.clone(),
                move || {
                    let _ = handle.emit(CHANGED_EVENT, ());
                },
                move || {
                    let _ = pages.emit(CHANGED_EVENT, ());
                },
            );
        }
        Err(error) => eprintln!("[docbase] {error}"),
    }
    app.manage(DocBaseState { base, wake, ask_wake });
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPreview {
    /// Suggested origin name: the folder's own name.
    pub name: String,
    /// A few example paths, one folder level per element, to help the
    /// user say what each level means.
    pub examples: Vec<Vec<String>>,
}

#[tauri::command(async)]
pub fn docbase_list_origins(state: State<'_, DocBaseState>) -> Result<Vec<OriginSummary>, String> {
    state.base()?.origin_summaries(&Visible::All)
}

#[tauri::command(async)]
pub fn docbase_preview_folder(folder: String) -> Result<FolderPreview, String> {
    let root = PathBuf::from(&folder);
    if !root.is_dir() {
        return Err(format!("'{folder}' no es una carpeta que se pueda leer"));
    }
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| folder.clone());
    Ok(FolderPreview {
        name,
        examples: example_paths(&root, 3, 4),
    })
}

#[tauri::command(async)]
pub fn docbase_add_origin(
    state: State<'_, DocBaseState>,
    name: String,
    folder: String,
    convention: Vec<String>,
) -> Result<Origin, String> {
    let base = state.base()?;
    let origin = base.add_origin(&name, Path::new(&folder), &convention)?;
    state.wake.request_sweep();
    Ok(origin)
}

#[tauri::command(async)]
pub fn docbase_remove_origin(state: State<'_, DocBaseState>, origin_id: String) -> Result<(), String> {
    state.base()?.remove_origin(&origin_id)
}

/// Asks for a sweep of every origin now; the result arrives as
/// `docbase://changed` events.
#[tauri::command(async)]
pub fn docbase_rescan(state: State<'_, DocBaseState>) -> Result<(), String> {
    state.base()?;
    state.wake.request_sweep();
    Ok(())
}

/// What the Inicio screen shows of the base.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub health: Health,
    pub origins: Vec<OriginSummary>,
    pub activity: Vec<Activity>,
}

#[tauri::command(async)]
pub fn docbase_home(state: State<'_, DocBaseState>) -> Result<Home, String> {
    let base = state.base()?;
    Ok(Home {
        health: base.health(&Visible::All)?,
        origins: base.origin_summaries(&Visible::All)?,
        activity: base.activity(&Visible::All, 12)?,
    })
}

#[tauri::command(async)]
pub fn docbase_list_documents(
    state: State<'_, DocBaseState>,
    filter: Option<DocumentFilter>,
) -> Result<DocumentPage, String> {
    state
        .base()?
        .documents(&Visible::All, &filter.unwrap_or_default())
}

#[tauri::command(async)]
pub fn docbase_document_copies(
    state: State<'_, DocBaseState>,
    location_id: i64,
) -> Result<Vec<LocationRef>, String> {
    state.base()?.copies(&Visible::All, location_id)
}

#[tauri::command(async)]
pub fn docbase_get_document(
    state: State<'_, DocBaseState>,
    location_id: i64,
) -> Result<Option<DocumentDetail>, String> {
    state.base()?.document(&Visible::All, location_id)
}

#[tauri::command(async)]
pub fn docbase_label_groups(state: State<'_, DocBaseState>) -> Result<Vec<LabelGroup>, String> {
    state.base()?.label_groups()
}

/// The label with that name in that group (none: it stands alone),
/// created when it does not exist yet.
#[tauri::command(async)]
pub fn docbase_ensure_label(
    state: State<'_, DocBaseState>,
    group: Option<String>,
    name: String,
) -> Result<Label, String> {
    state.base()?.ensure_label(group.as_deref(), &name)
}

#[tauri::command(async)]
pub fn docbase_rename_label(state: State<'_, DocBaseState>, label_id: i64, name: String) -> Result<(), String> {
    state.base()?.rename_label(label_id, &name)
}

#[tauri::command(async)]
pub fn docbase_delete_label(state: State<'_, DocBaseState>, label_id: i64) -> Result<(), String> {
    state.base()?.delete_label(label_id)
}

#[tauri::command(async)]
pub fn docbase_rename_group(state: State<'_, DocBaseState>, group_id: i64, name: String) -> Result<(), String> {
    state.base()?.rename_group(group_id, &name)
}

#[tauri::command(async)]
pub fn docbase_delete_group(state: State<'_, DocBaseState>, group_id: i64) -> Result<(), String> {
    state.base()?.delete_group(group_id)
}

#[tauri::command(async)]
pub fn docbase_label_rules(state: State<'_, DocBaseState>, origin_id: String) -> Result<Vec<LabelRule>, String> {
    state.base()?.label_rules(&origin_id)
}

/// "Everything under this folder carries this label."
#[tauri::command(async)]
pub fn docbase_add_fixed_rule(
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
    label_id: i64,
) -> Result<(), String> {
    state.base()?.add_fixed_rule(&origin_id, &folder, label_id)
}

/// "Each subfolder of this folder is a … (group), level by level."
/// An empty list removes that rule from the folder.
#[tauri::command(async)]
pub fn docbase_set_levels_rule(
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
    groups: Vec<String>,
) -> Result<(), String> {
    state.base()?.set_levels_rule(&origin_id, &folder, &groups)
}

#[tauri::command(async)]
pub fn docbase_remove_rule(state: State<'_, DocBaseState>, rule_id: i64) -> Result<(), String> {
    state.base()?.remove_rule(rule_id)
}

/// Puts a label on one file or takes it off, whatever the rules say.
#[tauri::command(async)]
pub fn docbase_set_location_label(
    state: State<'_, DocBaseState>,
    location_id: i64,
    label_id: i64,
    on: bool,
) -> Result<(), String> {
    state.base()?.set_location_label(location_id, label_id, on)
}

/// The folders and files directly inside one folder of an origin.
#[tauri::command(async)]
pub fn docbase_list_folder(
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
) -> Result<FolderListing, String> {
    state.base()?.list_folder(&Visible::All, &origin_id, &folder)
}

/// Leaves a file or a folder out of the base, or brings it back. Nothing
/// on disk is touched.
#[tauri::command(async)]
pub fn docbase_set_excluded(
    state: State<'_, DocBaseState>,
    origin_id: String,
    rel_path: String,
    is_folder: bool,
    excluded: bool,
) -> Result<(), String> {
    state
        .base()?
        .set_excluded(&origin_id, &rel_path, is_folder, excluded)?;
    // Bringing something back gives the worker files to read.
    state.wake.request_sweep();
    Ok(())
}

/// Shows a folder of an origin in the system's file manager.
#[tauri::command(async)]
pub fn docbase_open_folder(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = state
        .base()?
        .folder_path(&origin_id, &folder)?
        .ok_or_else(|| "Ese origen ya no existe".to_string())?;
    if !Path::new(&path).is_dir() {
        return Err(format!("No se puede abrir '{path}': la carpeta no está disponible ahora"));
    }
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|e| format!("No se pudo abrir '{path}': {e}"))
}

/// Opens a file of the inventory with the program the system uses for
/// it, or shows it in its folder. Only locations the base knows can be
/// opened this way.
#[tauri::command(async)]
pub fn docbase_open_location(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    location_id: i64,
    reveal: bool,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = state
        .base()?
        .location_path(&Visible::All, location_id)?
        .ok_or_else(|| "Ese archivo ya no está en el inventario".to_string())?;
    if !Path::new(&path).exists() {
        return Err(format!("No se puede abrir '{path}': el archivo no está disponible ahora"));
    }
    let opened = if reveal {
        app.opener().reveal_item_in_dir(&path)
    } else {
        app.opener().open_path(path.clone(), None::<&str>)
    };
    opened.map_err(|e| format!("No se pudo abrir '{path}': {e}"))
}

/// The documents similar to the one at this location, with the relation
/// proposed for each, its description, measures and evidence.
#[tauri::command(async)]
pub fn docbase_similar_documents(state: State<'_, DocBaseState>, location_id: i64) -> Result<Vec<SimilarDocument>, String> {
    state.base()?.similar_of(&Visible::All, location_id)
}

/// What a person decided about two similar documents: join them with a
/// relation (and, when it makes them one document, which one stays as
/// the wiki's source), discard the pair, or undo the decision.
#[tauri::command(async)]
pub fn docbase_decide_similar(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    content_id: i64,
    other_content_id: i64,
    decision: Decision,
    evidence_hash: Option<String>,
) -> Result<(), String> {
    state
        .base()?
        .decide_similar(content_id, other_content_id, &decision, evidence_hash.as_deref())?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// Several decisions about similar documents at once (a family confirmed
/// in one click). Returns how many were recorded.
#[tauri::command(async)]
pub fn docbase_decide_similar_many(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    decisions: Vec<PairDecision>,
) -> Result<usize, String> {
    let result = state.base()?.decide_similar_many(&decisions);
    let _ = app.emit(CHANGED_EVENT, ());
    result
}

/// The pairs that wait for a decision outside the families' principals.
#[tauri::command(async)]
pub fn docbase_similar_open_pairs(state: State<'_, DocBaseState>) -> Result<Vec<OpenPair>, String> {
    state.base()?.similar_open_pairs(&Visible::All)
}

/// How many similar documents and families wait for a decision (Inicio).
#[tauri::command(async)]
pub fn docbase_similar_pending(state: State<'_, DocBaseState>) -> Result<SimilarPending, String> {
    state.base()?.similar_pending(&Visible::All)
}

/// The provisional families, each with its principal document.
#[tauri::command(async)]
pub fn docbase_similar_groups(state: State<'_, DocBaseState>) -> Result<Vec<SimilarGroup>, String> {
    state.base()?.similar_groups(&Visible::All)
}

/// The template the document at this location shares with its family:
/// the text they share with its fields, and the value of each document.
/// `None` when it shares none.
#[tauri::command(async)]
pub fn docbase_similar_template(state: State<'_, DocBaseState>, location_id: i64) -> Result<Option<similar::template::Template>, String> {
    state.base()?.similar_template(&Visible::All, location_id)
}

/// The relations a person can choose, as the catalog lists them. Their
/// names come from the vocabulary.
#[tauri::command(async)]
pub fn docbase_similar_relations() -> Vec<RelationDef> {
    similar::relations()
}

/// What a run of passes 4 to 6 would ask the model now: how many
/// questions about type and stage, how many about data, and the types the
/// model proposed that wait for a person.
#[tauri::command(async)]
pub fn docbase_types_plan(state: State<'_, DocBaseState>) -> Result<TypesPlan, String> {
    state.base()?.types_plan()
}

/// The types the model proposed that wait for a person, without
/// computing anything (the attention view asks it on every change).
#[tauri::command(async)]
pub fn docbase_proposed_types(state: State<'_, DocBaseState>) -> Result<Vec<ProposedType>, String> {
    state.base()?.proposed_types()
}

/// Confirms (with the name it keeps), renames, rejects or undoes the
/// decision about a type the model proposed (R-22).
#[tauri::command(async)]
pub fn docbase_decide_type(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    type_id: String,
    decision: TypeDecision,
) -> Result<(), String> {
    state.base()?.decide_type(&type_id, &decision)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// The types people confirmed or rejected.
#[tauri::command(async)]
pub fn docbase_decided_types(state: State<'_, DocBaseState>) -> Result<Vec<DecidedType>, String> {
    state.base()?.decided_types()
}

/// Asks the model the pending questions about type, stage and data, at
/// most `max_questions`, with this engine. Only on an explicit action of
/// the user: it costs the provider's quota. The engine is also kept as
/// the one that answers while the archive's wiki is active.
#[tauri::command(async)]
pub fn docbase_types_run(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    engine: AskConfig,
    max_questions: Option<usize>,
) -> Result<RunReport, String> {
    let base = state.base()?.clone();
    let asker = asker_for(&engine)?;
    let report = base.types_run(asker.as_ref(), max_questions.unwrap_or(RUN_LIMIT))?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(report)
}

/// The registry of the AI services (which engine each service uses), with
/// the catalog of engines (P-141).
#[tauri::command(async)]
pub fn docbase_ai_services(state: State<'_, DocBaseState>) -> Result<AiServicesView, String> {
    state.base()?.ai_services()
}

/// What the base and the chat ask the model (P-278): each question with its
/// version, how it is built over an invented document, the catalogs it is
/// written with and the engine that answers it. Asks nothing and writes
/// nothing; without a base, the questions all the same.
#[tauri::command(async)]
pub fn docbase_model_questions(app: tauri::AppHandle, state: State<'_, DocBaseState>) -> Result<Vec<ModelQuestion>, String> {
    // Where a catalog can be replaced (`start`).
    let folder = app.path().app_data_dir().ok().map(|dir| dir.join("docbase"));
    super::questions::model_questions(state.base().ok().map(|base| base.as_ref()), folder.as_deref())
}

/// The last question of its current version the base keeps, with its whole
/// text: it carries the text of the documents it was about.
#[tauri::command(async)]
pub fn docbase_model_question_last(state: State<'_, DocBaseState>, id: String) -> Result<Option<AskedQuestion>, String> {
    state.base()?.last_model_question(&id)
}

/// Saves the registry of the AI services: the next request uses it, and
/// the questions to the model stop waiting to see the change. The screens
/// that show an engine read it from what this returns.
#[tauri::command(async)]
pub fn docbase_set_ai_services(state: State<'_, DocBaseState>, services: AiServices) -> Result<AiServicesView, String> {
    let base = state.base()?;
    base.set_ai_services(services)?;
    state.ask_wake.request();
    base.ai_services()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTypeView {
    pub doc_type: Option<DocumentType>,
    pub data: Vec<Datum>,
}

/// What the document at this location is, in what stage, and its data.
#[tauri::command(async)]
pub fn docbase_document_type(state: State<'_, DocBaseState>, location_id: i64) -> Result<DocumentTypeView, String> {
    let (doc_type, data) = state.base()?.document_type_of(location_id)?;
    Ok(DocumentTypeView { doc_type, data })
}

/// The closed list of document types, for the screens: the catalog's,
/// whose names come from the vocabulary, and then the ones a person
/// confirmed from the model's proposals, with the name they were given.
#[tauri::command(async)]
pub fn docbase_document_types(state: State<'_, DocBaseState>) -> Result<Vec<DocTypeView>, String> {
    let mut out: Vec<DocTypeView> = similar::types::catalog()
        .document_types()
        .map(|t| DocTypeView {
            id: t.id.clone(),
            stages: t.stages.clone(),
            data: t.data.iter().map(|d| d.id.clone()).collect(),
            name: None,
        })
        .collect();
    for decided in state.base()?.decided_types()? {
        if decided.decision == "confirmed" {
            out.push(DocTypeView { id: decided.id, stages: Vec::new(), data: Vec::new(), name: decided.name });
        }
    }
    Ok(out)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocTypeView {
    pub id: String,
    pub stages: Vec<String>,
    pub data: Vec<String>,
    /// For a type a person confirmed: its name. The catalog's are named by the vocabulary.
    pub name: Option<String>,
}

/// Questions a run started by the user asks at most (R-16).
const RUN_LIMIT: usize = 60;

/// How far the knowledge layer read the archive: documents read, still
/// to read and failed, mentions, identifiers and claims kept, and what
/// the check of the quotes discarded.
#[tauri::command(async)]
pub fn docbase_knowledge_status(state: State<'_, DocBaseState>) -> Result<KnowledgeStatus, String> {
    state.base()?.knowledge_status(&Visible::All)
}

/// The graph of the archive from the base (D-351, D-410): its people,
/// organizations, groups, documents and matters, and their relations.
#[tauri::command(async)]
pub fn docbase_graph(state: State<'_, DocBaseState>) -> Result<super::graph::BaseGraph, String> {
    state.base()?.graph(&Visible::All)
}

/// What the knowledge layer read in the document at this location: its
/// mentions with their role, identifiers and quotes, its claims, and what
/// was discarded.
#[tauri::command(async)]
pub fn docbase_knowledge_document(state: State<'_, DocBaseState>, location_id: i64) -> Result<DocumentKnowledge, String> {
    state.base()?.knowledge_of(&Visible::All, location_id)
}

/// What a reading for the knowledge layer would ask now: documents still
/// to read, the questions that takes, and those waiting for their type.
#[tauri::command(async)]
pub fn docbase_knowledge_plan(state: State<'_, DocBaseState>) -> Result<KnowledgePlan, String> {
    state.base()?.knowledge_plan()
}

/// Reads the pending documents for the knowledge layer, asking at most
/// `max_questions` questions, with this engine. Only on an explicit action
/// of the user: it costs the provider's quota. The engine is also kept as
/// the one that answers while the archive's wiki is active.
#[tauri::command(async)]
pub fn docbase_knowledge_run(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    engine: AskConfig,
    max_questions: Option<usize>,
) -> Result<KnowledgeReport, String> {
    let base = state.base()?.clone();
    let asker = asker_for(&engine)?;
    let report = base.knowledge_run(asker.as_ref(), max_questions.unwrap_or(RUN_LIMIT))?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(report)
}

/// Resolves the identities of what the knowledge layer read: without a
/// model, or asking at most `max_questions` questions to this engine (only
/// on an explicit action of the user: it costs the provider's quota).
#[tauri::command(async)]
pub fn docbase_knowledge_resolve(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    engine: Option<AskConfig>,
    max_questions: Option<usize>,
) -> Result<ResolveReport, String> {
    let base = state.base()?.clone();
    let report = match engine {
        Some(engine) => {
            let asker = asker_for(&engine)?;
            base.knowledge_resolve(Some(asker.as_ref()), max_questions.unwrap_or(RUN_LIMIT))?
        }
        None => base.knowledge_resolve(None, 0)?,
    };
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(report)
}

/// Projects the record into the archive's wiki now: the pages of the
/// entities, the blocks of the pages of documents and the overview. Without
/// an engine, what the documents say is left as it is (or pending); with
/// one, at most `max_questions` questions (only on an explicit action of
/// the user: it costs the provider's quota).
#[tauri::command(async)]
pub fn docbase_knowledge_project(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    engine: Option<AskConfig>,
    max_questions: Option<usize>,
) -> Result<ProjectReport, String> {
    let base = state.base()?.clone();
    let report = match engine {
        Some(engine) => {
            let asker = asker_for(&engine)?;
            base.knowledge_project(Some(asker.as_ref()), max_questions.unwrap_or(RUN_LIMIT))?
        }
        None => base.knowledge_project(None, 0)?,
    };
    if report.wrote() {
        let _ = app.emit(CHANGED_EVENT, ());
    }
    Ok(report)
}

/// The language the archive's wiki is written in ("auto" or none: the
/// language of the documents), for the pages the knowledge layer writes.
#[tauri::command(async)]
pub fn docbase_set_wiki_language(state: State<'_, DocBaseState>, language: Option<String>) -> Result<(), String> {
    state.base()?.set_wiki_language(language.as_deref())
}

/// The entities of the record, of one kind or all, whose names contain
/// the words of `query`.
#[tauri::command(async)]
pub fn docbase_knowledge_entities(
    state: State<'_, DocBaseState>,
    kind: Option<String>,
    query: Option<String>,
) -> Result<Vec<EntitySummary>, String> {
    state.base()?.knowledge_entities(&Visible::All, kind.as_deref(), query.as_deref())
}

/// One entity with its aliases, identifiers, mentions, claims and doubts.
#[tauri::command(async)]
pub fn docbase_knowledge_entity(state: State<'_, DocBaseState>, id: String) -> Result<Option<EntityDetail>, String> {
    state.base()?.knowledge_entity(&Visible::All, &id)
}

// The pages, notes, overview and saved queries kept in the base (D-402,
// D-404): what the screens, the search and the chat read instead of the
// files of the wiki.

/// The article of an object (`ent-…`, `mat-…`, `doc-…`) as the projection
/// last wrote it, with the notes of a person; none while it has no page.
#[tauri::command(async)]
pub fn docbase_knowledge_article(state: State<'_, DocBaseState>, id: String) -> Result<Option<KnowledgeArticle>, String> {
    state.base()?.knowledge_article(&id)
}

/// The article of a document's page (the page of its family of versions).
#[tauri::command(async)]
pub fn docbase_knowledge_document_article(state: State<'_, DocBaseState>, content_id: i64) -> Result<Option<KnowledgeArticle>, String> {
    state.base()?.knowledge_document_article(content_id)
}

/// The object a page names: its logical path (`wiki/entities/…`), the old
/// wiki's absolute path, or a link (`name` of `[[name|text]]`).
#[tauri::command(async)]
pub fn docbase_knowledge_object_of_page(state: State<'_, DocBaseState>, reference: String) -> Result<Option<PageObject>, String> {
    state.base()?.knowledge_object_of_page(&reference)
}

/// The notes of a person on an object.
#[tauri::command(async)]
pub fn docbase_knowledge_notes(state: State<'_, DocBaseState>, object_id: String) -> Result<Option<KnowledgeNote>, String> {
    state.base()?.knowledge_notes(&object_id)
}

/// Keeps the notes of a person on an object; empty text removes them.
#[tauri::command(async)]
pub fn docbase_knowledge_set_notes(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    object_id: String,
    text: String,
) -> Result<Option<KnowledgeNote>, String> {
    let note = state.base()?.knowledge_set_notes(&object_id, &text)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(note)
}

/// The notes of the old wiki whose page matched no object, kept apart.
#[tauri::command(async)]
pub fn docbase_knowledge_orphan_notes(state: State<'_, DocBaseState>) -> Result<Vec<KnowledgeNote>, String> {
    state.base()?.knowledge_orphan_notes()
}

/// Removes a note by its number.
#[tauri::command(async)]
pub fn docbase_knowledge_delete_note(app: tauri::AppHandle, state: State<'_, DocBaseState>, id: i64) -> Result<(), String> {
    state.base()?.knowledge_delete_note(id)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// The search of the base (its pages), for the search screen (D-408).
#[tauri::command(async)]
pub fn docbase_search(
    state: State<'_, DocBaseState>,
    query: String,
    top_k: Option<usize>,
) -> Result<crate::commands::search::ProjectSearchResponse, String> {
    crate::commands::search::search_base(state.base()?, &query, top_k.unwrap_or(20))
}

/// The words the pages are written with (their headings), so that a screen
/// shows one part of an article in the words of the vocabulary (D-408).
#[tauri::command]
pub fn docbase_knowledge_page_words() -> std::collections::BTreeMap<String, String> {
    super::knowledge::vocabulary::vocabulary().pages.words.clone()
}

/// The overview of the archive.
#[tauri::command(async)]
pub fn docbase_knowledge_overview(state: State<'_, DocBaseState>) -> Result<Option<KnowledgeOverview>, String> {
    state.base()?.knowledge_overview()
}

/// The saved queries of the chat, the latest first.
#[tauri::command(async)]
pub fn docbase_saved_queries(state: State<'_, DocBaseState>) -> Result<Vec<SavedQuery>, String> {
    state.base()?.saved_queries()
}

/// Saves a question of the chat with its answer and what it cites.
#[tauri::command(async)]
pub fn docbase_save_query(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    title: String,
    question: String,
    answer: String,
    refs: Vec<String>,
) -> Result<SavedQuery, String> {
    let saved = state.base()?.save_query(&title, &question, &answer, &refs)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(saved)
}

#[tauri::command(async)]
pub fn docbase_delete_saved_query(app: tauri::AppHandle, state: State<'_, DocBaseState>, id: i64) -> Result<(), String> {
    state.base()?.delete_saved_query(id)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// A page of the base whole, with its title, by its logical path or a
/// link: what the chat hands a provider of the command line (D-413).
#[tauri::command(async)]
pub fn docbase_knowledge_page_text(state: State<'_, DocBaseState>, path: String) -> Result<Option<PageText>, String> {
    let base = state.base()?;
    let Some(object) = base.knowledge_object_of_page(&path)? else { return Ok(None) };
    Ok(base.knowledge_page_text(&object.path)?.map(|(title, text)| PageText { path: object.path, title, text }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageText {
    pub path: String,
    pub title: String,
    pub text: String,
}

/// Keeps a research on the internet, of a person, organization or matter,
/// or loose (D-414).
#[tauri::command(async)]
pub fn docbase_research_save(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    object_id: Option<String>,
    topic: String,
    body: String,
    sources: Vec<ResearchSource>,
) -> Result<Research, String> {
    let saved = state.base()?.save_research(object_id.as_deref(), &topic, &body, &sources)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(saved)
}

/// The researches of an object, the latest first.
#[tauri::command(async)]
pub fn docbase_research_of(state: State<'_, DocBaseState>, object_id: String) -> Result<Vec<Research>, String> {
    state.base()?.research_of(&object_id)
}

/// The loose researches, the latest first.
#[tauri::command(async)]
pub fn docbase_research_loose(state: State<'_, DocBaseState>) -> Result<Vec<Research>, String> {
    state.base()?.loose_research()
}

#[tauri::command(async)]
pub fn docbase_research_delete(app: tauri::AppHandle, state: State<'_, DocBaseState>, id: i64) -> Result<(), String> {
    state.base()?.delete_research(id)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// The folder of the chat's history and preferences in the app's data
/// folder (D-415); the first time, the chat files of the general project's
/// folder are copied into it.
#[tauri::command(async)]
pub fn docbase_chat_folder(app: tauri::AppHandle, state: State<'_, DocBaseState>) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no se encontró la carpeta de datos de la app: {e}"))?
        .join(super::chat_files::CHAT_FOLDER);
    std::fs::create_dir_all(dir.join("chats")).map_err(|e| format!("no se pudo crear '{}': {e}", dir.display()))?;
    if let Ok(base) = state.base() {
        match base.copy_chat_files_once(&dir) {
            Ok(report) if !report.skipped => eprintln!("[docbase] chat files copied to the data folder: {report:?}"),
            Ok(_) => {}
            Err(error) => eprintln!("[docbase] the chat files could not be copied: {error}"),
        }
    }
    Ok(dir.to_string_lossy().replace('\\', "/"))
}

/// Whether the base reads its documents on its own (`background-reading`).
#[tauri::command(async)]
pub fn docbase_background_reading(state: State<'_, DocBaseState>) -> Result<bool, String> {
    state.base()?.background_reading()
}

#[tauri::command(async)]
pub fn docbase_set_background_reading(state: State<'_, DocBaseState>, on: bool) -> Result<(), String> {
    state.base()?.set_background_reading(on)
}


/// A person asks to recognize the text of scans (P-242), with the
/// automatic reading on or off: these documents (`content_ids`), every one
/// waiting (none given), or the ones that failed (`only_failed`). They are
/// read in the background, one after the other, and «En curso» shows how
/// it goes. Returns how many were asked for.
#[tauri::command(async)]
pub fn docbase_recognize_text(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    content_ids: Option<Vec<i64>>,
    only_failed: Option<bool>,
) -> Result<usize, String> {
    let which = match (content_ids, only_failed.unwrap_or(false)) {
        (Some(ids), _) => RecognitionRequest::These(ids),
        (None, true) => RecognitionRequest::Failed,
        (None, false) => RecognitionRequest::AllWaiting,
    };
    let base = state.base()?;
    if base.recognition_engine()?.is_none() {
        return Err("Falta elegir el motor que reconoce escaneos: en Ajustes → Servicios de IA, elegí uno para «Reconocer escaneos».".to_string());
    }
    let asked = base.request_recognition(&which)?;
    state.ask_wake.request();
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(asked)
}

/// Every stage of the processing over `scope` (the whole archive, an
/// origin or a folder of it), with how it stands and the last error of the
/// model: what «En curso» shows (D-424). Reads only.
#[tauri::command(async)]
pub fn docbase_progress(state: State<'_, DocBaseState>, scope: Option<Scope>) -> Result<Progress, String> {
    state.base()?.progress(&scope.unwrap_or_default())
}

/// The documents (or origins, or pages) of one stage, failed first, then
/// pending, then done. Reads only.
#[tauri::command(async)]
pub fn docbase_progress_items(
    state: State<'_, DocBaseState>,
    stage: String,
    scope: Option<Scope>,
    limit: Option<usize>,
) -> Result<StageItems, String> {
    state.base()?.progress_items(&stage, &scope.unwrap_or_default(), limit.unwrap_or(50))
}

/// The folders of an origin with the files under each, to narrow «En curso».
#[tauri::command(async)]
pub fn docbase_progress_folders(state: State<'_, DocBaseState>, origin_id: String) -> Result<Vec<FolderCount>, String> {
    state.base()?.progress_folders(&origin_id)
}

/// The pairs of mentions the resolution left for a person.
#[tauri::command(async)]
pub fn docbase_knowledge_doubts(state: State<'_, DocBaseState>) -> Result<Vec<DoubtView>, String> {
    state.base()?.knowledge_doubts(&Visible::All)
}

/// Every decision about identities, the latest first.
#[tauri::command(async)]
pub fn docbase_knowledge_decisions(state: State<'_, DocBaseState>) -> Result<Vec<DecisionView>, String> {
    state.base()?.knowledge_decisions()
}

/// Keeps and applies a decision of the user about identities (join,
/// separate, wrong datum), with the evidence the screen showed.
#[tauri::command(async)]
pub fn docbase_knowledge_decide(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    decision: IdentityDecision,
    note: Option<String>,
    shown_evidence: Option<String>,
) -> Result<i64, String> {
    let id = state.base()?.knowledge_decide(&decision, note.as_deref(), shown_evidence.as_deref())?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(id)
}

/// Undoes a decision about identities.
#[tauri::command(async)]
pub fn docbase_knowledge_undo(app: tauri::AppHandle, state: State<'_, DocBaseState>, id: i64) -> Result<(), String> {
    state.base()?.knowledge_undo(id)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// Reads again the documents whose reading failed (an answer of another
/// shape, or an empty reading of a long text). Returns how many.
#[tauri::command(async)]
pub fn docbase_knowledge_read_again_failed(app: tauri::AppHandle, state: State<'_, DocBaseState>) -> Result<usize, String> {
    let count = state.base()?.knowledge_read_again_failed(&Visible::All)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(count)
}

/// What reading again a document, the documents of an origin or folder, or
/// the whole archive would take, for the notice of its cost (D-432).
#[tauri::command(async)]
pub fn docbase_knowledge_reread_plan(state: State<'_, DocBaseState>, target: Option<RereadTarget>) -> Result<RereadPlan, String> {
    state.base()?.knowledge_reread_plan(&target.unwrap_or_default())
}

/// Asks to read them again with new answers of the model (D-432): kept in
/// the base until it is done. Returns how many documents.
#[tauri::command(async)]
pub fn docbase_knowledge_reread(app: tauri::AppHandle, state: State<'_, DocBaseState>, target: Option<RereadTarget>) -> Result<usize, String> {
    let base = state.base()?;
    if super::similar::classify::ask_engine(&base.conn())?.is_none() {
        return Err("Falta elegir el motor de «Preguntar al modelo» en Ajustes → Servicios de IA: sin él no se puede volver a leer.".to_string());
    }
    let count = base.knowledge_request_reread(&target.unwrap_or_default())?;
    state.ask_wake.request();
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(count)
}

/// Keeps and applies several decisions of the user at once, with one
/// resolution (taking a document out of a matter, confirming a family).
#[tauri::command(async)]
pub fn docbase_knowledge_decide_many(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    decisions: Vec<IdentityDecision>,
    note: Option<String>,
) -> Result<Vec<i64>, String> {
    let ids = state.base()?.knowledge_decide_many(&decisions, note.as_deref())?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(ids)
}

/// What the knowledge layer brings to the attention view: failed and noisy
/// readings, conflicts, recent joins by identifier, matters the model
/// proposed, pages kept and how far the work went.
#[tauri::command(async)]
pub fn docbase_knowledge_attention(state: State<'_, DocBaseState>) -> Result<KnowledgeAttention, String> {
    state.base()?.knowledge_attention(&Visible::All)
}

/// The matters of the archive, the largest first.
#[tauri::command(async)]
pub fn docbase_knowledge_matters(state: State<'_, DocBaseState>) -> Result<Vec<MatterSummary>, String> {
    state.base()?.knowledge_matters(&Visible::All)
}

/// One matter with its documents, why each is in it, its parties and the
/// decisions about it.
#[tauri::command(async)]
pub fn docbase_knowledge_matter(state: State<'_, DocBaseState>, id: String) -> Result<Option<MatterDetail>, String> {
    state.base()?.knowledge_matter(&Visible::All, &id)
}

/// The entities the document at this location names, with their roles,
/// and its matter.
#[tauri::command(async)]
pub fn docbase_knowledge_document_entities(state: State<'_, DocBaseState>, location_id: i64) -> Result<DocumentEntities, String> {
    state.base()?.knowledge_document_entities(&Visible::All, location_id)
}

/// Takes a document (`content:<id>` or `location:<id>`) out of a matter,
/// or turns down the model's proposal of it.
#[tauri::command(async)]
pub fn docbase_knowledge_leave_matter(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    document: String,
    matter_id: String,
    note: Option<String>,
) -> Result<Vec<i64>, String> {
    let ids = state.base()?.knowledge_leave_matter(&document, &matter_id, note.as_deref())?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(ids)
}

/// Confirms, discards or reopens a matter the grouping proposed
/// (`confirm`, `discard`, `reopen`).
#[tauri::command(async)]
pub fn docbase_knowledge_decide_matter(app: tauri::AppHandle, state: State<'_, DocBaseState>, matter_id: String, decision: String) -> Result<(), String> {
    state.base()?.knowledge_decide_matter(&matter_id, &decision)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// Creates a matter by hand (empty or with documents); returns its id.
#[tauri::command(async)]
pub fn docbase_knowledge_create_matter(app: tauri::AppHandle, state: State<'_, DocBaseState>, name: String, documents: Vec<String>) -> Result<String, String> {
    let id = state.base()?.knowledge_create_matter(&name, &documents)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(id)
}

/// Names a matter by hand (an empty name gives back the app's name).
#[tauri::command(async)]
pub fn docbase_knowledge_rename_matter(app: tauri::AppHandle, state: State<'_, DocBaseState>, matter_id: String, name: String) -> Result<(), String> {
    state.base()?.knowledge_rename_matter(&matter_id, &name)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// Puts a document in a matter (`in`) or takes it out (`out`), by hand.
#[tauri::command(async)]
pub fn docbase_knowledge_choose_matter_document(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    matter_id: String,
    document: String,
    choice: String,
) -> Result<(), String> {
    state.base()?.knowledge_choose_matter_document(&matter_id, &document, &choice)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

/// Puts a document in a matter, or confirms the model's proposal of it.
#[tauri::command(async)]
pub fn docbase_knowledge_join_matter(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    document: String,
    matter_id: String,
    note: Option<String>,
) -> Result<i64, String> {
    let id = state.base()?.knowledge_join_matter(&document, &matter_id, note.as_deref())?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(id)
}

/// Up to `count` paths of nested folders under `root`, `depth` levels
/// deep at most, following the first folders in name order.
fn example_paths(root: &Path, count: usize, depth: usize) -> Vec<Vec<String>> {
    fn subfolders(dir: &Path) -> Vec<(String, PathBuf)> {
        let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
            .map(|entry| (entry.file_name().to_string_lossy().to_string(), entry.path()))
            .filter(|(name, _)| !name.starts_with('.') && !name.starts_with('$'))
            .collect();
        found.sort_by_key(|(name, _)| name.to_lowercase());
        found
    }

    let mut examples = Vec::new();
    for (name, path) in subfolders(root).into_iter().take(count) {
        let mut levels = vec![name];
        let mut current = path;
        while levels.len() < depth {
            let Some((name, path)) = subfolders(&current).into_iter().next() else {
                break;
            };
            levels.push(name);
            current = path;
        }
        examples.push(levels);
    }
    examples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    #[test]
    fn folder_examples_follow_the_first_folders_down() {
        let dir = TempDir::new();
        dir.folder("BOREAL/Reclamo facturas/Anexos");
        dir.folder("Andina/Contrato");
        dir.folder(".oculta/x");
        dir.write("suelto.txt", b"x");

        let examples = example_paths(dir.path(), 3, 4);
        assert_eq!(
            examples,
            vec![
                vec!["Andina".to_string(), "Contrato".to_string()],
                vec!["BOREAL".to_string(), "Reclamo facturas".to_string(), "Anexos".to_string()],
            ]
        );
    }
}
