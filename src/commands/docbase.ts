import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

/** A folder incorporated into the space. Never copied, never written to. */
export interface Origin {
  id: string
  name: string
  provider: "local-folder"
  location: string
  addedAt: number
  lastScanAt: number | null
  /** `null` until the first sweep. `false`: what was known is kept. */
  reachable: boolean | null
}

export interface OriginSummary extends Origin {
  /** Files in the inventory, cloud shortcuts included. */
  files: number
  /** Files still waiting to be read. */
  pending: number
  /** Files that could not be read. */
  errors: number
  /** Cloud shortcuts: detected, not processable. */
  links: number
}

export interface FolderPreview {
  name: string
  /** A few example paths, one folder level per element. */
  examples: string[][]
}

export function listOrigins(): Promise<OriginSummary[]> {
  return invoke<OriginSummary[]>("docbase_list_origins")
}

export function previewFolder(folder: string): Promise<FolderPreview> {
  return invoke<FolderPreview>("docbase_preview_folder", { folder })
}

/**
 * Incorporates a folder. `levels` says how the whole folder is ordered
 * ("cliente", "caso") and becomes a label rule on its root; leave it
 * empty unless the folder is ordered the same way throughout.
 */
export function addOrigin(name: string, folder: string, levels: string[]): Promise<Origin> {
  return invoke<Origin>("docbase_add_origin", { name, folder, convention: levels })
}

export function removeOrigin(originId: string): Promise<void> {
  return invoke<void>("docbase_remove_origin", { originId })
}

/** Asks for a sweep of every origin; results arrive through `onDocbaseChanged`. */
export function rescanOrigins(): Promise<void> {
  return invoke<void>("docbase_rescan")
}

/** Fires whenever something the document base screens show may have changed. */
export function onDocbaseChanged(handler: () => void): Promise<UnlistenFn> {
  return listen("docbase://changed", () => handler())
}

/**
 * Where a document stands, from the inventory to its text.
 * `link`: cloud shortcut, detected and not processable. `unread`: waiting
 * to be read. `unprocessed`: identified, text not extracted yet.
 * `needs-recognition`: its text is only pixels.
 */
export type DocumentState =
  | "excluded"
  | "link"
  | "unread"
  | "unreadable"
  | "too-large"
  | "unprocessed"
  | "processed"
  | "needs-recognition"
  | "unsupported"
  | "error"

export interface DocumentFilter {
  /** Words to look for in the name, the path and the text. All must be found. */
  query?: string
  originId?: string
  state?: DocumentState | ""
  /** Only documents with more than one location. */
  duplicatesOnly?: boolean
  /** Only files carrying this label. */
  labelId?: number | null
  /** Only documents with a similar document that waits for a decision. */
  similarPending?: boolean
  /** Only the members of the family whose principal is this content. */
  similarGroup?: number | null
  /** Only documents of this type; `"none"`: those without a type yet. */
  docType?: string | null
  /** One row per document (its most recent copy) instead of one per file; `total` counts documents. */
  onePerDocument?: boolean
  sort?: "recent" | "name" | "folder"
  limit?: number
  offset?: number
}

/** One location, with what is known of the document it holds. */
export interface DocumentRow {
  locationId: number
  originId: string
  originName: string
  originReachable: boolean | null
  relPath: string
  folder: string
  name: string
  ext: string
  size: number
  mtimeMs: number
  state: DocumentState
  contentId: number | null
  /** Locations holding this same document, this one included. */
  copies: number
  labels: Label[]
  /** What the document is: a type of the catalog, or one the model proposed. */
  docType: string | null
  docStage: string | null
  /** The type is a proposal of the model that no one confirmed yet. */
  docTypeProposed: boolean
  /** The name of a type outside the catalog: proposed by the model, or given by a person who confirmed it. */
  docTypeName: string | null
}

/** A label: an answer ("Zentora") to its group's question ("Cliente"), or standing alone. */
export interface Label {
  id: number
  groupId: number | null
  group: string | null
  name: string
  color: string
}

export interface LabelUse extends Label {
  /** Files carrying it. */
  files: number
}

export interface LabelGroup {
  /** `null` for the labels that stand alone. */
  id: number | null
  name: string | null
  labels: LabelUse[]
}

/** A rule on a folder that labels everything under it. */
export interface LabelRule {
  id: number
  originId: string
  /** Folder it sits on; "" for the root of the origin. */
  folder: string
  /** `fixed`: everything carries `label`. `levels`: each subfolder is a label of `groups[0]`, and so on down. */
  kind: "fixed" | "levels"
  label: Label | null
  groups: string[]
}

export function listLabelGroups(): Promise<LabelGroup[]> {
  return invoke<LabelGroup[]>("docbase_label_groups")
}

/** Finds the label, creating its group and the label when they do not exist. */
export function ensureLabel(group: string | null, name: string): Promise<Label> {
  return invoke<Label>("docbase_ensure_label", { group, name })
}

export function renameLabel(labelId: number, name: string): Promise<void> {
  return invoke<void>("docbase_rename_label", { labelId, name })
}

export function deleteLabel(labelId: number): Promise<void> {
  return invoke<void>("docbase_delete_label", { labelId })
}

export function renameGroup(groupId: number, name: string): Promise<void> {
  return invoke<void>("docbase_rename_group", { groupId, name })
}

export function deleteGroup(groupId: number): Promise<void> {
  return invoke<void>("docbase_delete_group", { groupId })
}

export function listLabelRules(originId: string): Promise<LabelRule[]> {
  return invoke<LabelRule[]>("docbase_label_rules", { originId })
}

export function addFixedRule(originId: string, folder: string, labelId: number): Promise<void> {
  return invoke<void>("docbase_add_fixed_rule", { originId, folder, labelId })
}

/** An empty list removes the rule from the folder. */
export function setLevelsRule(originId: string, folder: string, groups: string[]): Promise<void> {
  return invoke<void>("docbase_set_levels_rule", { originId, folder, groups })
}

export function removeRule(ruleId: number): Promise<void> {
  return invoke<void>("docbase_remove_rule", { ruleId })
}

/** Puts a label on one file or takes it off, whatever the folder rules say. */
export function setLocationLabel(locationId: number, labelId: number, on: boolean): Promise<void> {
  return invoke<void>("docbase_set_location_label", { locationId, labelId, on })
}

export interface DocumentPage {
  rows: DocumentRow[]
  /** Rows matching the filter, before `limit` and `offset`. */
  total: number
}

export interface LocationRef {
  locationId: number
  originName: string
  originReachable: boolean | null
  relPath: string
  path: string
  mtimeMs: number
  /** False when it is the same document in another file (another format). */
  sameFile: boolean
}

export interface DocumentDetail extends DocumentRow {
  path: string
  fileHash: string | null
  error: string | null
  wordCount: number | null
  processedAt: number | null
  firstSeenAt: number
  text: string | null
  textEngine: string | null
  /** When that text was obtained (for a scan: when it was recognized). */
  textAt: number | null
  /** Every location of this document, this one first. */
  locations: LocationRef[]
  /** When it was generated from a template: when, from which and for which matter (D-488). */
  generated: GeneratedLine | null
}

/** What the screen of a generated document says of it. */
export interface GeneratedLine {
  /** Its record. */
  id: number
  generatedAt: number
  /** The name the template had then. */
  templateName: string
  /** The location of the template's principal now, to open it; `null` when the template is no more. */
  templateLocation: number | null
  matterId: string | null
  matterName: string | null
}

/** A document generated with a template, in the template's screen. */
export interface GeneratedEntry {
  id: number
  generatedAt: number
  fileName: string
  matterName: string | null
  /** Where the base has it; `null` until it finds it, or when it is gone. */
  locationId: number | null
}

/** How the archive is doing, in numbers. */
export interface Health {
  origins: number
  unreachableOrigins: number
  /** Files in the inventory, cloud shortcuts included. */
  locations: number
  /** Distinct documents among the files already read. */
  documents: number
  /** Files by state. */
  byState: Partial<Record<DocumentState, number>>
  /** Files whose document is also somewhere else. */
  copies: number
  words: number
  lastScanAt: number | null
}

/** Something that happened to the files; one sweep's changes come together. */
export interface Activity {
  at: number
  kind: "added" | "modified" | "removed" | "moved"
  originName: string
  count: number
  /** One of the files; the only one when `count` is 1. */
  relPath: string
  /** For a move, where the file was before. */
  fromPath: string | null
}

export interface Home {
  health: Health
  origins: OriginSummary[]
  activity: Activity[]
}

export function getHome(): Promise<Home> {
  return invoke<Home>("docbase_home")
}

export interface FolderNode {
  name: string
  /** Path inside the origin, with forward slashes. */
  relPath: string
  /** Files anywhere under it. */
  files: number
  /** Left out by the user, itself or through a folder above it. */
  excluded: boolean
}

export interface FolderListing {
  originId: string
  folder: string
  excluded: boolean
  folders: FolderNode[]
  files: DocumentRow[]
}

/** The folders and files directly inside one folder of an origin ("" is its root). */
export function listFolder(originId: string, folder: string): Promise<FolderListing> {
  return invoke<FolderListing>("docbase_list_folder", { originId, folder })
}

/** Leaves a file or folder out of the base, or brings it back. Nothing on disk is touched. */
export function setExcluded(originId: string, relPath: string, isFolder: boolean, excluded: boolean): Promise<void> {
  return invoke<void>("docbase_set_excluded", { originId, relPath, isFolder, excluded })
}

export function openFolder(originId: string, folder: string): Promise<void> {
  return invoke<void>("docbase_open_folder", { originId, folder })
}

export function listDocuments(filter: DocumentFilter): Promise<DocumentPage> {
  return invoke<DocumentPage>("docbase_list_documents", { filter })
}

/** Every location of the document this location holds, this one first. */
export function documentCopies(locationId: number): Promise<LocationRef[]> {
  return invoke<LocationRef[]>("docbase_document_copies", { locationId })
}

export function getDocument(locationId: number): Promise<DocumentDetail | null> {
  return invoke<DocumentDetail | null>("docbase_get_document", { locationId })
}

/** Opens the original with the system's program, or shows it in its folder. */
export function openLocation(locationId: number, reveal = false): Promise<void> {
  return invoke<void>("docbase_open_location", { locationId, reveal })
}

// ── Similar documents ───────────────────────────────────────────────────

/**
 * Id of a relation between two documents, from the catalog
 * (`src-tauri/resources/catalog/document-types.json`). The list can grow
 * without code changes; the name the user reads comes from the vocabulary.
 */
export type RelationId =
  | "same-document"
  | "near-identical"
  | "signed-copy"
  | "preview"
  | "version"
  | "contained-in"
  | "same-template"
  | "similar"
  | (string & {})

export interface RelationDef {
  id: RelationId
  /** Joining makes both one document: only one stays as the wiki's source. */
  sameDocument: boolean
  /** One is the relation of the other (the signed copy, the preview…). */
  directed: boolean
  /** Found by comparing texts; the others are inferred. */
  measured: boolean
  /** Applied without a person (only the exact same text). */
  automatic: boolean
  /**
   * A person decides it: the two may be one same document (a copy, a
   * version, one inside the other, a delivery receipt, a signing
   * certificate). The same template and the only similar are different
   * documents and wait for no one (D-497).
   */
  asksPerson: boolean
}

/** A document of the base, by one of its locations. */
export interface DocRef {
  contentId: number
  locationId: number
  originName: string
  relPath: string
  name: string
  copies: number
}

export type SimilarState = "proposed" | "joined" | "discarded"

/** Kind of thing that changes between two similar documents. */
export type ChangeKind = "identifier" | "amount" | "date" | "number" | "name" | "field" | "wording"

/** What changes between two similar documents, as stored in the evidence. */
export interface SimilarChanges {
  matchedWords: number
  hunks: number
  removedWords: number
  addedWords: number
  dataTokens: number
  wordingTokens: number
  items: Partial<Record<ChangeKind, number>>
  longestHunk: number
  largestBlock: number
  repeated: { from: string; to: string; times: number }[]
  swaps: Partial<Record<ChangeKind, number>>
  examples: { kind: ChangeKind; a: string; b: string }[]
}

export interface SimilarEvidence {
  engine: string
  rule: string
  measures: { jaccard: number; containment: number; cover: number; similarity: number }
  words: { a: number; b: number }
  ownWords: { a: number; b: number }
  recognized: { a: boolean; b: boolean }
  marks: { a: string[]; b: string[] }
  changes: SimilarChanges
}

export interface SimilarDocument {
  /** The document asked about. */
  contentId: number
  other: DocRef
  /** Proposed relation. */
  relation: RelationId
  /** For a directed relation, which of the two is the relation of the other. */
  subjectContentId: number | null
  certainty: "high" | "low"
  /** One sentence in Spanish: "Mismo texto de base; cambian 3 importes y una fecha." */
  description: string
  /** Shingles in common over all the shingles of the two (without routine text). */
  jaccard: number
  /** Shingles in common over those of the smaller one. */
  containment: number
  /** Part of the shorter one found, word by word, in the other. */
  cover: number
  similarity: number
  /** For relations that make them one document, the source the app suggests. */
  suggestedSource: number | null
  evidence: SimilarEvidence
  /** Fingerprint of the evidence: send it back when deciding (what the person saw). */
  evidenceHash: string
  computedAt: number
  state: SimilarState
  chosenRelation: RelationId | null
  /** The document left as the wiki's source when joined as one document. */
  wikiSource: number | null
  decidedAt: number | null
  /**
   * Two of one template where no datum changes: possibly the same
   * document, so the pair waits for a person although made with one
   * template (D-440, D-497).
   */
  possiblySame: boolean
}

export type SimilarDecision =
  | { kind: "join"; relation: RelationId; wikiSource: number | null }
  | { kind: "discard" }
  | { kind: "undo" }

export interface SimilarPending {
  /**
   * Pairs with no decision whose proposed relation asks a person
   * (`RelationDef.asksPerson`): a copy, a version, one inside the other, a
   * delivery receipt or a signing certificate (D-497).
   */
  pairs: number
  /** Families with at least one such pair. */
  groups: number
  /**
   * Of `pairs`, those whose proposed relation makes both one document
   * (practically the same, signed copy, version): deciding them changes
   * which one feeds the wiki.
   */
  sameDocumentPairs: number
}

export interface SimilarGroupMember {
  doc: DocRef
  relation: RelationId | null
  cover: number | null
  state: SimilarState | null
}

export interface SimilarGroup {
  principal: DocRef
  /** The principal first. */
  members: SimilarGroupMember[]
  /** "large" (grew too much) or "dispersed" (a chain the principal does not hold). */
  flags: string[]
  /** Members whose pair with the principal waits for a person (its relation asks one). */
  pending: number
  /** Of `pending`, those whose proposed relation makes both one document. */
  pendingSameDocument: number
  /** How many fields its template has (each once), when it shares one. */
  templateFields: number | null
  /** Pairs of its template's documents where nothing changes but a few words, undecided: possibly the same document. */
  possiblySame: number
}

/** A piece of a template's text: what all its documents say, or a field. */
export type TemplateSegment = { text: string } | { field: number }

/** A document of a template, and the state of its pair with the principal (`null` for the principal). */
export interface TemplateDocument {
  doc: DocRef
  state: SimilarState | null
}

/** A field of a template: what changes from one document to another. */
export interface TemplateField {
  /** The kind of thing it is; its name comes from the vocabulary ("Fecha 2"). */
  kind: ChangeKind
  /** Its order among the fields of its kind, from 1. */
  ordinal: number
  /** For a field of a form, its name in the form. */
  formName: string | null
  /** The same field said again: the index of its first place, whose name it takes. */
  sameAs: number | null
  /** The value of each document, in the order of `Template.documents`; empty: nothing there. */
  values: string[]
  /**
   * The name it is shown with: what a person wrote, the name its text gives
   * it, or the model's; `null`: by its kind and order, from the vocabulary.
   */
  name: string | null
  nameSource: "person" | "label" | "model" | null
  /** What it is: a type of entity ("persona"…) or a kind of data ("fecha"…); `null` when nothing says. */
  what: string | null
  whatSource: "person" | "model" | "kind" | null
  /** A person marked it fixed: it is written always as `fixedValue`, and not asked when generating. */
  fixed: boolean
  fixedValue: string | null
}

/** What a person decides about a field of a template; all empty: back to the automatic ones. */
export interface FieldChoice {
  name: string | null
  what: string | null
  fixed: boolean
  fixedValue: string | null
}

/** Names a template, or gives it back its automatic name (`null`). */
export function nameTemplate(principal: number, name: string | null): Promise<void> {
  return invoke<void>("docbase_name_template", { principal, name })
}

/** What a person decides about the field at `position` of a template. */
export function chooseTemplateField(principal: number, position: number, choice: FieldChoice): Promise<void> {
  return invoke<void>("docbase_choose_template_field", { principal, position, choice })
}

/**
 * The template of a family of the same template: the text its documents
 * share, with the fields that change, and the value of each document.
 * Built without a model, also for families nobody confirmed ("proposed").
 */
export interface Template extends TemplateKind {
  principal: number
  /** The document asked about (its content id). */
  asked: number
  /** The principal first. */
  documents: TemplateDocument[]
  state: "proposed" | "confirmed"
  segments: TemplateSegment[]
  fields: TemplateField[]
  /** Pairs of documents (content ids) where nothing changes but a few words: possibly the same document. */
  possiblySame: [number, number][]
  computedAt: number
  format: TemplateFormat
  /** The name a person gave it; `null`: its automatic name. */
  name: string | null
  /** What a field can be, for a person to choose. */
  whats: string[]
  /** The documents generated with it, the latest first. */
  generated: GeneratedEntry[]
}

/**
 * The type of document of a template's principal, which names the template
 * until a person names it, and its order among the templates whose
 * principal has the same type, when there is more than one.
 */
export interface TemplateKind {
  docType: string | null
  /** The name the model proposed for a type outside the catalog, or the one a person confirmed. */
  proposedTypeName: string | null
  typeRank: number | null
}

/** Where the format of a template comes from, as its principal is. */
export type TemplateFormat = "word" | "pdf-text" | "pdf-form" | "scan" | "text"

/** A template in the list of the section «Plantillas». */
export interface TemplateSummary extends TemplateKind {
  principal: DocRef
  /** The name a person gave it; `null`: its automatic name. */
  name: string | null
  state: "proposed" | "confirmed"
  /** The documents of it, the principal included. */
  documents: number
  /** Its fields, each once; of them, the wording that changes. */
  fields: number
  wordingFields: number
  format: TemplateFormat
  /** Pairs of its documents possibly the same document, undecided. */
  possiblySame: number
  /** Fields whose name (or what they are) waits for the model. */
  waitingFields: number
  /** Documents generated with it. */
  generated: number
}

/** Every template of the base, for the section «Plantillas». */
export function listTemplates(): Promise<TemplateSummary[]> {
  return invoke<TemplateSummary[]>("docbase_templates")
}

/**
 * Where the value of a field of a generated document came from: written or
 * chosen by a person (`hand`), a person or organization of the base
 * (`entity`), what the base knows of it, such as its DNI or its address
 * (`card`), a matter (`matter`), one of the template's documents
 * (`document`), the calendar (`date`) or the field's fixed value (`fixed`).
 */
export type GeneratedValueSource = "hand" | "entity" | "card" | "matter" | "document" | "date" | "fixed"

/** The value of a field of a document to generate, by the position of its first place. */
export interface GivenValue {
  position: number
  /** Its name as the screen shows it. */
  name: string
  value: string
  source: GeneratedValueSource
  /** What it came from: `entity:<id>`, `matter:<id>`, `content:<id>`. */
  reference: string | null
}

/** What a person asks to generate from a template. */
export interface GenerateRequest {
  /** The template, by its principal (content id). */
  principal: number
  /** Its name as the screen shows it. */
  templateName: string
  values: GivenValue[]
  /** The matter it is generated for. */
  matterId: string | null
  /** The name of the file; `null`: the template's and the date. */
  fileName: string | null
  /** The document whose data it started with (content id). */
  fromDocument: number | null
}

/** A document generated from a template: its record, its file and where the base has it. */
export interface GeneratedDocument {
  id: number
  path: string
  fileName: string
  locationId: number | null
  contentId: number | null
  /** It is in the matter it was generated for. */
  inMatter: boolean
  /** The fields left empty that the Word shows by name, to be filled in Word. */
  missing: string[]
}

/** Generates a simple Word from a template in «Generados» of the work folder, with its record (D-488). */
export function generateDocument(request: GenerateRequest): Promise<GeneratedDocument> {
  return invoke<GeneratedDocument>("docbase_generate_document", { request })
}

/** Opens a generated document in Word, or shows it in its folder (`reveal`). */
export function openGenerated(id: number, reveal = false): Promise<void> {
  return invoke<void>("docbase_open_generated", { id, reveal })
}

/** The documents similar to the one at this location. */
export function listSimilarDocuments(locationId: number): Promise<SimilarDocument[]> {
  return invoke<SimilarDocument[]>("docbase_similar_documents", { locationId })
}

/** Joins, discards or undoes the decision about two similar documents. */
export function decideSimilar(
  contentId: number,
  otherContentId: number,
  decision: SimilarDecision,
  evidenceHash?: string,
): Promise<void> {
  return invoke<void>("docbase_decide_similar", { contentId, otherContentId, decision, evidenceHash: evidenceHash ?? null })
}

/** One decision of `decideSimilarMany`. */
export interface PairDecision {
  contentId: number
  otherContentId: number
  decision: SimilarDecision
  /** Fingerprint of the evidence the person saw. */
  evidenceHash?: string | null
}

/**
 * Several decisions at once, recorded in order with one regrouping (a
 * family confirmed in one click). Returns how many were recorded; when
 * one fails, the error says how many were saved before it.
 */
export function decideSimilarMany(decisions: PairDecision[]): Promise<number> {
  return invoke<number>("docbase_decide_similar_many", { decisions })
}

/** A pair waiting for a decision outside the families' principals, seen from `doc`. */
export interface OpenPair {
  doc: DocRef
  similar: SimilarDocument
}

/** The pairs that wait outside the families' principals: outside families, or between two members. */
export function listOpenSimilarPairs(): Promise<OpenPair[]> {
  return invoke<OpenPair[]>("docbase_similar_open_pairs")
}

export function getSimilarPending(): Promise<SimilarPending> {
  return invoke<SimilarPending>("docbase_similar_pending")
}

export function listSimilarGroups(): Promise<SimilarGroup[]> {
  return invoke<SimilarGroup[]>("docbase_similar_groups")
}

/** The template the document at this location shares with its family, or `null`. */
export function getSimilarTemplate(locationId: number): Promise<Template | null> {
  return invoke<Template | null>("docbase_similar_template", { locationId })
}

export function listSimilarRelations(): Promise<RelationDef[]> {
  return invoke<RelationDef[]>("docbase_similar_relations")
}

/** Which engine answers the questions about documents, and with which model. */
export interface AskConfig {
  engine: "codex-cli"
  model: string
  /** How much the model reasons; part of the engine's name in the base (D-274). */
  reasoning?: "low" | "medium" | "high"
}

/** What a document is and in what stage (pass 4). */
export interface DocumentType {
  contentId: number
  /** A type of the catalog, or the id the model proposed (`proposed`). */
  docType: string
  stage: string | null
  /** A new type the model proposed: not in the catalog, waiting for a person. */
  proposed: boolean
  proposedName: string | null
  proposedDescription: string | null
  /** "marks": verified without the model; "model": asked about it; "family": the answer about its family's principal. */
  source: "marks" | "model" | "family"
  principal: number | null
  confidence: number | null
  reason: string | null
  computedAt: number
}

/** One value of one datum of a document (pass 5). */
export interface Datum {
  field: string
  value: string
  /** "rule", "mark" or "model". */
  source: string
}

export interface DocumentTypeView {
  docType: DocumentType | null
  data: Datum[]
}

export interface ProposedType {
  id: string
  name: string | null
  description: string | null
  documents: number
}

/** What a run would ask the model now. */
export interface TypesPlan {
  documents: number
  typed: number
  typeQuestions: number
  /** More may appear once the pending types are known. */
  dataQuestions: number
  proposedTypes: ProposedType[]
  engine: AskConfig | null
}

/** What a run did. */
export interface TypesRunReport {
  asked: number
  failed: number
  typed: number
  dataAsked: number
  dataFailed: number
  corrected: number
  left: number
  errors: string[]
}

/** A document type of the catalog, with its stages and data; names come from the vocabulary. */
export interface DocTypeDef {
  id: string
  stages: string[]
  data: string[]
  /** For a type a person confirmed from the model's proposals: its name. */
  name: string | null
}

export function getTypesPlan(): Promise<TypesPlan> {
  return invoke<TypesPlan>("docbase_types_plan")
}

/** What a person decides about a type the model proposed (R-22). */
export type TypeDecision =
  /** It joins the closed list with this name: the model's, or another one (rename). */
  | { kind: "confirm"; name: string; description?: string | null }
  /** Its documents lose it and are asked again, telling the model not to propose it. */
  | { kind: "reject" }
  | { kind: "undo" }

export function decideType(typeId: string, decision: TypeDecision): Promise<void> {
  return invoke<void>("docbase_decide_type", { typeId, decision })
}

/** A type a person confirmed or rejected. */
export interface DecidedType {
  id: string
  decision: "confirmed" | "rejected"
  name: string | null
  description: string | null
  decidedAt: number
  /** Documents that have it now. */
  documents: number
}

export function listDecidedTypes(): Promise<DecidedType[]> {
  return invoke<DecidedType[]>("docbase_decided_types")
}

/** The types the model proposed that wait for a person. Computes nothing. */
export function listProposedTypes(): Promise<ProposedType[]> {
  return invoke<ProposedType[]>("docbase_proposed_types")
}

/** Asks the model what is pending. Costs the provider's quota: only on an explicit action. */
export function runTypes(engine: AskConfig, maxQuestions?: number): Promise<TypesRunReport> {
  return invoke<TypesRunReport>("docbase_types_run", { engine, maxQuestions: maxQuestions ?? null })
}

/** Which engine recognizes the text of scans, and with which model. */
export interface RecognitionConfig {
  /** Codex, or Micelya Core (Textract, page by page). */
  engine: "codex-cli" | "core"
  model: string
}

/** The AI services of the registry (P-141, `docbase/services.rs`). */
export type AiServiceId = "ask" | "recognize" | "research"

/** One engine chosen for a service. */
export interface EngineChoice {
  /** The engine, as the catalog names it (`codex-cli`, `tavily`…). */
  engine: string
  /** The shared connection it uses (a preset of Conexiones, a search provider); empty: the engine's own. */
  connection: string
  model: string
  /** `low`, `medium` or `high` for an engine that offers it; empty: the model's default. */
  reasoning: string
}

/** What a service uses: its engine and an optional alternative. */
export interface ServiceChoice {
  engine: EngineChoice | null
  alternative: EngineChoice | null
}

/** The registry: which engine each AI service uses. */
export interface AiServices {
  ask: ServiceChoice
  recognize: ServiceChoice
  research: ServiceChoice
}

export type CapabilityLevel = "yes" | "partial" | "no"

/** One engine of the catalog, for one service (`resources/catalog/ai-engines.json`). */
export interface CatalogEngine {
  service: AiServiceId
  id: string
  name: string
  /** `local`: a program on this computer; `api`: a provider's API; `search`: a web search. */
  kind: "local" | "api" | "search"
  /** The base runs it by itself (today Codex, and Micelya Core for the scans). */
  inBase: boolean
  /** It takes a reasoning level. */
  reasoning: boolean
  /** The shared connection it needs (Ajustes → Conexiones), when it is one of those. */
  connection?: string
  /** The model it is saved with when none is chosen. */
  defaultModel?: string
  /** False: the person does not choose the model (the provider says which one reads). */
  modelChoice?: boolean
  /** How it reads, in words. */
  notes?: string
  /** What it says it can do, when it declares it. */
  capabilities?: Partial<Record<string, CapabilityLevel>>
  languages?: string[]
  /** Where what it is sent is processed. */
  processing: string
  /** What its provider keeps. */
  retention: string
}

export interface EnginesCatalog {
  services: { id: AiServiceId; name: string; purpose: string }[]
  capabilities: { id: string; name: string }[]
  engines: CatalogEngine[]
}

export interface AiServicesView {
  services: AiServices
  /** False until the registry is saved once: until then it is what the base had. */
  saved: boolean
  catalog: EnginesCatalog
}

/** The registry of the AI services, with the catalog of engines. */
export function getAiServices(): Promise<AiServicesView> {
  return invoke<AiServicesView>("docbase_ai_services")
}

/** Saves the registry; the next request of each service uses it. */
export function setAiServices(services: AiServices): Promise<AiServicesView> {
  return invoke<AiServicesView>("docbase_set_ai_services", { services })
}

// ── What is asked of the model (P-278, `docbase/questions.rs`) ──────────

/** A piece of a question: the same in every case, or a value that changes in each case (`part` says which). */
export interface QuestionSegment {
  text: string
  part: string | null
}

/** One text sent to the model, in order: the question, or the system text and the message of the chat. */
export interface QuestionSent {
  title: string
  segments: QuestionSegment[]
}

/** A catalog a question is written with, and which one is in use. */
export interface QuestionCatalog {
  file: string
  name: string
  /** The one of the data folder (true) or the one that comes with the app. */
  fromFolder: boolean
  folderPath: string | null
  /** A file in the data folder that is not in use (not valid, or put there after the app opened). */
  unusedFile: boolean
}

export interface QuestionVersionCount {
  version: string
  count: number
}

/** A question the base or the chat asks the model. */
export interface ModelQuestion {
  id: string
  name: string
  purpose: string
  when: string
  note: string | null
  /** Null: the question has no version. */
  version: string | null
  catalogs: QuestionCatalog[]
  /** The service of the registry that answers it, and its engine. */
  service: AiServiceId
  engine: EngineChoice | null
  /** Why it is not asked now, when it is not. */
  engineNote: string | null
  /** How it is built over an invented document, by the same functions as the reading. */
  example: QuestionSent[]
  noExample: string | null
  /** The shape of the answer it asks for, as JSON. */
  answerShape: string | null
  /** The base keeps each one asked. */
  kept: boolean
  /** How many the base keeps of each version, the current one first. */
  keptByVersion: QuestionVersionCount[]
}

/** The last question of its current version the base keeps; its text carries the text of the firm's documents. */
export interface AskedQuestion {
  version: string
  engine: string
  askedAt: number
  prompt: string
}

/** Every question the base and the chat ask the model. Asks nothing. */
export function listModelQuestions(): Promise<ModelQuestion[]> {
  return invoke<ModelQuestion[]>("docbase_model_questions")
}

/** The last question `id` of its current version the base keeps, or null. */
export function lastModelQuestion(id: string): Promise<AskedQuestion | null> {
  return invoke<AskedQuestion | null>("docbase_model_question_last", { id })
}

/**
 * Asks the base to recognize the text of scans (P-242), with the automatic
 * reading on or off: these documents, every one waiting (`null`), or the
 * ones that failed (`onlyFailed`). They are read in the background, one
 * after the other. Returns how many were asked for.
 */
export function recognizeText(contentIds: number[] | null, onlyFailed = false): Promise<number> {
  return invoke<number>("docbase_recognize_text", { contentIds, onlyFailed })
}

export function getDocumentType(locationId: number): Promise<DocumentTypeView> {
  return invoke<DocumentTypeView>("docbase_document_type", { locationId })
}

export function listDocumentTypes(): Promise<DocTypeDef[]> {
  return invoke<DocTypeDef[]>("docbase_document_types")
}

/** What a projection of the knowledge layer into the archive's wiki did. */
export interface KnowledgeProjectReport {
  skipped: boolean
  reason: string | null
  pages: number
  written: number
  unchanged: number
  removed: number
  kept: number
  /** Pages moved to the file of their entity's new name, and pages whose links followed. */
  moved: number
  redirected: number
  asked: number
  failed: number
  left: number
  sentences: number
  discarded: number
  sourcePages: number
  blocksWritten: number
  overviewWritten: boolean
  errors: string[]
}

/**
 * Projects the record of the knowledge layer into the archive's wiki now.
 * Without an engine nothing is asked; with one it costs the provider's
 * quota: only on an explicit action.
 */
export function projectKnowledge(engine: AskConfig | null, maxQuestions?: number): Promise<KnowledgeProjectReport> {
  return invoke<KnowledgeProjectReport>("docbase_knowledge_project", { engine, maxQuestions: maxQuestions ?? null })
}

/** The language the base writes its articles in. */
export function setWikiLanguage(language: string | null): Promise<void> {
  return invoke<void>("docbase_set_wiki_language", { language })
}

// ── Knowledge layer: people, organizations and matters ──────────────────

/** How far the knowledge layer read, resolved and projected the archive. */
export interface KnowledgeStatus {
  documents: number
  extracted: number
  stale: number
  /** Read with an earlier version of the instructions, kept as they are (D-432). */
  older: number
  /** Asked by a person to read again, waiting for it. */
  reread: number
  pending: number
  failed: number
  mentions: number
  identifiers: number
  evidence: number
  discarded: number
  discardedByReason: Record<string, number>
  answers: number
  entities: number
  entitiesByKind: Record<string, number>
  doubts: number
  claims: number
  conflicts: number
  toResolve: number
  pages: number
  toProject: number
  engine: AskConfig | null
}

export interface KnowledgeIdentifierView {
  kind: string
  value: string
  quote: string
  page: number | null
  byShape: boolean
}

export interface KnowledgeMentionView {
  id: number
  kind: string
  written: string
  role: string
  quote: string
  page: number | null
  sameAs: number | null
  entityId: string | null
  identifiers: KnowledgeIdentifierView[]
}

export interface KnowledgeEvidenceView {
  id: number
  subject: number
  predicate: string
  objectMention: number | null
  objectValue: string | null
  quote: string
  page: number | null
  factTime: string | null
}

export interface KnowledgeDiscarded {
  what: string
  reason: string
  said: string
  quote: string
}

/** What the layer read in one document. */
export interface DocumentKnowledge {
  contentId: number | null
  state: "done" | "failed" | "stale" | "pending"
  /** Read with an earlier version of the instructions (D-432). */
  older: boolean
  /** A person asked to read it again, and it waits for it. */
  reread: boolean
  engine: string | null
  extractedAt: number | null
  mentions: KnowledgeMentionView[]
  evidence: KnowledgeEvidenceView[]
  discarded: KnowledgeDiscarded[]
  error: string | null
}

export interface KnowledgePlan {
  documents: number
  questions: number
  waitingForType: number
  engine: AskConfig | null
}

export interface KnowledgeRunReport {
  asked: number
  failed: number
  extracted: number
  unusable: number
  left: number
  waitingForType: number
  mentions: number
  evidence: number
  discarded: number
  errors: string[]
}

export interface KnowledgeResolveReport {
  skipped: boolean
  asked: number
  failed: number
  left: number
  groups: number
  entities: number
  byPerson: number
  byIdentifier: number
  byName: number
  byModel: number
  doubts: number
  changed: number
  claims: number
  conflicts: number
  unfit: number
  matters: number
  matterDocuments: number
  commonKeys: number
  errors: string[]
}

export interface EntitySummary {
  id: string
  kind: string
  name: string
  mentions: number
  documents: number
  aliases: string[]
}

export interface EntityAlias {
  written: string
  mentions: number
}

export interface EntityIdentifier {
  kind: string
  value: string
  documents: number
}

export interface EntityMention {
  mentionId: number
  contentId: number
  locationId: number | null
  document: string
  role: string
  written: string
  quote: string
  page: number | null
  /** What joined it to the entity: "persona", "identificador", "nombre", "modelo" or "sola". */
  resolvedBy: string | null
}

export interface ClaimEvidence {
  evidenceId: number
  contentId: number
  document: string
  quote: string
  page: number | null
  factTime: string | null
}

export interface ClaimView {
  id: number
  predicate: string
  objectEntity: string | null
  objectName: string | null
  objectValue: string | null
  state: "supported" | "conflict" | "retired"
  validFrom: string | null
  evidence: ClaimEvidence[]
  /**
   * How sure the record is of it: "quoted" (its quote is in the document,
   * nobody confirmed it), "confirmed" or "corrected" by a person, or
   * "doubtful".
   */
  certainty: "quoted" | "confirmed" | "doubtful" | "corrected"
  /** Why: "conflict", "identity", "attribution", "states", "contradicted", "by-person". */
  reasons: string[]
}

/** Two groups of mentions the resolution left for a person. */
export interface DoubtView {
  a: string
  b: string
  kind: string
  entityA: string | null
  entityB: string | null
  nameA: string | null
  nameB: string | null
  /** Why it is in doubt ("identificador-parecido", "modelo-dudosa"…). */
  reason: string
  detail: string | null
  evidenceHash: string
}

export interface IdentityDecisionView {
  id: number
  /** "join", "separate", "wrong" or "confirm". */
  kind: string
  a: string
  b: string | null
  note: string | null
  shownEvidence: string
  state: "active" | "undone"
  decidedAt: number
  undoneAt: number | null
  /**
   * Why an active decision no longer finds what it named in the documents
   * read again ("no-encontrada", "varias"): a person reviews it.
   */
  review: string | null
  /** The keys it applies to now, when what it named is written another way (JSON). */
  appliedAs: string | null
}

export interface DocumentMatter {
  contentId: number
  matterId: string
  name: string
  proposed: boolean
  pagePath: string | null
}

export interface IncomingClaim {
  claimId: number
  subjectId: string
  subjectName: string
  subjectKind: string
  predicate: string
  state: string
  documents: number
}

export interface EntityDetail {
  id: string
  kind: string
  name: string
  state: "active" | "merged" | "retired"
  mergedInto: string | null
  aliases: EntityAlias[]
  identifiers: EntityIdentifier[]
  mentions: EntityMention[]
  claims: ClaimView[]
  doubts: DoubtView[]
  pagePath: string | null
  matters: DocumentMatter[]
  decisions: IdentityDecisionView[]
  incoming: IncomingClaim[]
}

/**
 * A decision of a person about identities and matters. References: an
 * entity (`entity:<id>`), a mention (`mention:<id>`), a group key, a piece
 * of evidence (`evidence:<id>`), a claim (`claim:<id>`), or a document
 * (`content:<id>`, `location:<id>`) for matters.
 */
export type IdentityDecision =
  | { kind: "join"; a: string; b: string }
  | { kind: "separate"; a: string; b: string }
  | { kind: "wrong"; a: string }
  | { kind: "confirm"; a: string }
  | { kind: "sameMatter"; a: string; b: string }
  | { kind: "otherMatter"; a: string; b: string }

export interface KnowledgeDocumentRef {
  contentId: number
  locationId: number | null
  name: string
}

export interface KnowledgeFailedReading {
  document: KnowledgeDocumentRef
  error: string | null
}

export interface KnowledgeNoisyReading {
  document: KnowledgeDocumentRef
  kept: number
  discarded: number
}

export interface KnowledgeConflictGroup {
  entityId: string
  entityName: string
  kind: string
  predicate: string
  within: string | null
  values: string[]
  claims: number[]
}

export interface KnowledgeIdentifierJoin {
  entityId: string
  name: string
  kind: string
  names: string[]
  identifiers: string[]
  documents: number
  readAt: number
}

/** A matter the grouping proposed, for a person to confirm or discard (D-381). */
export interface KnowledgeMatterToConfirm {
  id: string
  name: string
  documents: number
  /** In how many of its documents each kind of reason is ("numero", "asunto"...). */
  reasons: Record<string, number>
}

export interface KnowledgeProposedMatter {
  document: KnowledgeDocumentRef
  matterId: string
  matterName: string
  reason: string | null
}

export interface KnowledgeKeptPage {
  entityId: string
  path: string
  name: string | null
  mergedInto: string | null
}

export interface KnowledgeLeftWithDocuments {
  entityId: string
  name: string
  kind: string
  documents: KnowledgeDocumentRef[]
}

/** What the knowledge layer brings to the attention view. */
export interface KnowledgeAttention {
  status: KnowledgeStatus
  failed: KnowledgeFailedReading[]
  noisy: KnowledgeNoisyReading[]
  conflicts: KnowledgeConflictGroup[]
  identifierJoins: KnowledgeIdentifierJoin[]
  proposedMatters: KnowledgeProposedMatter[]
  /** The matters the grouping proposed that no person confirmed yet. */
  mattersToConfirm: KnowledgeMatterToConfirm[]
  synthesisPending: number
  discardedSentences: number
  keptPages: KnowledgeKeptPage[]
  leftWithDocuments: KnowledgeLeftWithDocuments[]
  /** Decisions of a person that no longer find what they named. */
  decisionsToReview: KnowledgeDecisionReview[]
}

export interface KnowledgeDecisionReview {
  decision: IdentityDecisionView
  document: KnowledgeDocumentRef | null
  /** "no-encontrada" or "varias". */
  reason: string
}

/** "open" for now (its life cycle is P-213). */
export type MatterStatus = "open"
/** Who made it: the grouping ("proposed") or a person. */
export type MatterOrigin = "proposed" | "person"
/** What a person decided about it. */
export type MatterConfirmation = "proposed" | "confirmed" | "discarded"

export interface MatterSummary {
  id: string
  name: string
  status: MatterStatus
  origin: MatterOrigin
  confirmation: MatterConfirmation
  documents: number
  proposed: number
  pagePath: string | null
}

export interface MatterReason {
  /** "persona", "documento", "asunto", "numero" or "importe". */
  kind: string
  value: string
  valueName: string | null
  with: KnowledgeDocumentRef | null
}

export interface MatterDocument {
  document: KnowledgeDocumentRef
  state: "firm" | "proposed"
  reasons: MatterReason[]
  proposalReason: string | null
}

export interface MatterParty {
  entityId: string
  kind: string
  name: string
  roles: string[]
  documents: number
}

export interface MatterDetail {
  id: string
  name: string
  state: "active" | "retired"
  status: MatterStatus
  origin: MatterOrigin
  confirmation: MatterConfirmation
  pagePath: string | null
  documents: MatterDocument[]
  parties: MatterParty[]
  decisions: IdentityDecisionView[]
}

export interface DocumentEntity {
  entityId: string | null
  kind: string
  name: string
  written: string[]
  roles: string[]
  pagePath: string | null
}

/** A reference of a document (a record, a file, an act): data of the document, not an entity. */
export interface DocumentReference {
  written: string
  normalized: string
  alsoWritten: string[]
  role: string
  quotes: { quote: string; page: number | null }[]
  identifiers: KnowledgeIdentifierView[]
}

export interface DocumentEntities {
  contentId: number | null
  state: "done" | "failed" | "stale" | "pending"
  /** Read with an earlier version of the instructions (D-432). */
  older: boolean
  /** A person asked to read it again, and it waits for it. */
  reread: boolean
  error: string | null
  entities: DocumentEntity[]
  references: DocumentReference[]
  matters: DocumentMatter[]
  discarded: number
}

export function getKnowledgeStatus(): Promise<KnowledgeStatus> {
  return invoke<KnowledgeStatus>("docbase_knowledge_status")
}

export function getKnowledgePlan(): Promise<KnowledgePlan> {
  return invoke<KnowledgePlan>("docbase_knowledge_plan")
}

export function getDocumentKnowledge(locationId: number): Promise<DocumentKnowledge> {
  return invoke<DocumentKnowledge>("docbase_knowledge_document", { locationId })
}

/** Reads the pending documents for the layer. It costs the provider's quota: only on an explicit action. */
export function runKnowledge(engine: AskConfig, maxQuestions?: number): Promise<KnowledgeRunReport> {
  return invoke<KnowledgeRunReport>("docbase_knowledge_run", { engine, maxQuestions: maxQuestions ?? null })
}

/** Resolves identities; with an engine it costs the provider's quota: only on an explicit action. */
export function resolveKnowledge(engine: AskConfig | null, maxQuestions?: number): Promise<KnowledgeResolveReport> {
  return invoke<KnowledgeResolveReport>("docbase_knowledge_resolve", { engine, maxQuestions: maxQuestions ?? null })
}

export function listEntities(kind: string | null = null, query: string | null = null): Promise<EntitySummary[]> {
  return invoke<EntitySummary[]>("docbase_knowledge_entities", { kind, query })
}

export function getEntity(id: string): Promise<EntityDetail | null> {
  return invoke<EntityDetail | null>("docbase_knowledge_entity", { id })
}

// The pages, notes, overview and saved queries kept in the base (D-402,
// D-404): what the screens, the search and the chat read instead of the
// files of the wiki.

/**
 * The notes of a person on an object (`ent-…`, `mat-…`, `doc-…`). A note
 * of the old wiki whose page matched no object is kept apart: no
 * `objectId`, with the `title` of the page it came from (D-403).
 */
export interface KnowledgeNote {
  id: number
  objectId: string | null
  title: string | null
  originPath: string | null
  text: string
  updatedAt: number
}

/** The article of an object: its page as the projection last wrote it in the base. */
export interface KnowledgeArticle {
  objectId: string
  kind: "entity" | "matter" | "document"
  /** Its logical path (`wiki/entities/…`, `wiki/sources/…`): what links between pages say. */
  path: string
  title: string
  /** Markdown with its header (frontmatter); without the notes, which come apart. */
  body: string
  /** `kept`: its object left the record; the page stays for its notes. */
  state: "projected" | "kept"
  writtenAt: number
  notes: KnowledgeNote | null
}

/** The object a page or a link names; `overview` for the overview. */
export interface PageObject {
  objectId: string
  kind: "entity" | "matter" | "document" | "overview"
  path: string
  /** For a document: the document its page is named by, and one place of it. */
  contentId: number | null
  locationId: number | null
}

export interface KnowledgeOverview {
  path: string
  body: string
  writtenAt: number
}

/** A question of the chat a person saved, with its answer and what it cites. */
export interface SavedQuery {
  id: number
  title: string
  question: string
  answer: string
  /** Logical paths, link names or object ids. */
  refs: string[]
  createdAt: number
}

/** The article of a person, organization, group, matter (`mat-…`) or document page (`doc-…`). */
export function getKnowledgeArticle(id: string): Promise<KnowledgeArticle | null> {
  return invoke<KnowledgeArticle | null>("docbase_knowledge_article", { id })
}

/** The article of a document's page (its family of versions), by the document. */
export function getDocumentArticle(contentId: number): Promise<KnowledgeArticle | null> {
  return invoke<KnowledgeArticle | null>("docbase_knowledge_document_article", { contentId })
}

/**
 * The object a page names: its logical path, the old wiki's absolute path,
 * or a link (`name` of `[[name|text]]`, with or without `.md` or `#anchor`).
 */
export function objectOfPage(reference: string): Promise<PageObject | null> {
  return invoke<PageObject | null>("docbase_knowledge_object_of_page", { reference })
}

export function getKnowledgeNotes(objectId: string): Promise<KnowledgeNote | null> {
  return invoke<KnowledgeNote | null>("docbase_knowledge_notes", { objectId })
}

/** Keeps the notes of a person on an object; empty text removes them. */
export function setKnowledgeNotes(objectId: string, text: string): Promise<KnowledgeNote | null> {
  return invoke<KnowledgeNote | null>("docbase_knowledge_set_notes", { objectId, text })
}

/** The notes of the old wiki whose page matched no object. */
export function listOrphanNotes(): Promise<KnowledgeNote[]> {
  return invoke<KnowledgeNote[]>("docbase_knowledge_orphan_notes")
}

export function deleteKnowledgeNote(id: number): Promise<void> {
  return invoke<void>("docbase_knowledge_delete_note", { id })
}

/** The overview of the archive (for Inicio). */
export function getKnowledgeOverview(): Promise<KnowledgeOverview | null> {
  return invoke<KnowledgeOverview | null>("docbase_knowledge_overview")
}

/**
 * The words the pages are written with, by their id in the vocabulary
 * (`matterFacts`, `docLocations`…): the headings of their sections.
 */
export function getPageWords(): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("docbase_knowledge_page_words")
}

/** A page the search of the base found: a document's, or of whom the question names. */
export interface BaseSearchResult {
  /** Its logical path: `objectOfPage` gives the screen it opens. */
  path: string
  title: string
  snippet: string
  score: number
  /** The people and organizations of the result the question names. */
  graphRelatedTo?: string[]
}

export interface BaseSearchResponse {
  mode: string
  results: BaseSearchResult[]
}

/** Searches the base: the texts, numbers and summaries of its documents (D-350, D-408). */
export function searchBase(query: string, topK = 20): Promise<BaseSearchResponse> {
  return invoke<BaseSearchResponse>("docbase_search", { query, topK })
}

/** The saved queries, the latest first. */
export function listSavedQueries(): Promise<SavedQuery[]> {
  return invoke<SavedQuery[]>("docbase_saved_queries")
}

export function saveQuery(title: string, question: string, answer: string, refs: string[]): Promise<SavedQuery> {
  return invoke<SavedQuery>("docbase_save_query", { title, question, answer, refs })
}

export function deleteSavedQuery(id: number): Promise<void> {
  return invoke<void>("docbase_delete_saved_query", { id })
}

/** A page of the base whole (what the app wrote, then the notes of a person). */
export interface PageText {
  /** Its logical path. */
  path: string
  title: string
  text: string
}

/** A page of the base by its logical path or a link; null when the base has none. */
export function getPageText(path: string): Promise<PageText | null> {
  return invoke<PageText | null>("docbase_knowledge_page_text", { path })
}

/** A source a research on the internet cites, by its number in the text (`[n]`). */
export interface ResearchSource {
  index: number
  title: string
  url: string
  source: string
}

/**
 * A research on the internet kept in the base (D-414): of a person,
 * organization or matter, or loose; apart from what the documents say.
 */
export interface Research {
  id: number
  objectId: string | null
  topic: string
  body: string
  sources: ResearchSource[]
  createdAt: number
}

export function saveResearch(
  objectId: string | null,
  topic: string,
  body: string,
  sources: ResearchSource[],
): Promise<Research> {
  return invoke<Research>("docbase_research_save", { objectId, topic, body, sources })
}

/** The researches of a person, organization or matter, the latest first. */
export function listResearchOf(objectId: string): Promise<Research[]> {
  return invoke<Research[]>("docbase_research_of", { objectId })
}

/** The researches of no object, the latest first. */
export function listLooseResearch(): Promise<Research[]> {
  return invoke<Research[]>("docbase_research_loose")
}

export function deleteResearch(id: number): Promise<void> {
  return invoke<void>("docbase_research_delete", { id })
}

/** The folder of the chat's history and preferences, in the app's data folder (D-415). */
export function getChatFolder(): Promise<string> {
  return invoke<string>("docbase_chat_folder")
}

/** Whether the base reads its documents on its own («Lectura automática»). */
export function getBackgroundReading(): Promise<boolean> {
  return invoke<boolean>("docbase_background_reading")
}

export function setBackgroundReading(on: boolean): Promise<void> {
  return invoke<void>("docbase_set_background_reading", { on })
}

// ── How far the processing went, stage by stage (D-424) ──────────────────

/** The stages, in the order a document goes through them. */
export type StageId =
  | "origins"
  | "text"
  | "recognition"
  | "similar"
  | "types"
  | "templates"
  | "reading"
  | "resolve"
  | "matters"
  | "pages"

/**
 * How a stage stands: `waiting` is its turn; `paused`, the automatic
 * reading is off or no model (or, for the scans, no engine) was chosen.
 */
export type StageState = "done" | "running" | "waiting" | "paused" | "error"

/** What part of the archive the figures are about; empty, all of it. */
export interface ProgressScope {
  originId?: string | null
  /** A folder inside the origin, with forward slashes. */
  folder?: string | null
}

/** The last error of the model, in memory while the app runs. */
export interface ModelError {
  /** The stage that asked; null when the model could not even be started. */
  stage: StageId | null
  kind: "limit" | "key" | "connection" | "other"
  message: string
  at: number
  /** When the engine tries again. */
  retryAt: number | null
}

/** One thing a stage goes over, as its list shows it. */
export interface StageItem {
  status: "pending" | "failed" | "done" | "out"
  locationId: number | null
  contentId: number | null
  originId: string | null
  /** For a page: its path, to open it. */
  pagePath: string | null
  name: string
  /** "<origin> › <folder>", or the folder of an origin on disk. */
  place: string
  /** The reason of a failure, why it is out, or the matter of a document. */
  detail: string | null
  /** Of a scan being recognized: the page the engine reads now. */
  page?: number | null
  /** Of a scan: how many pages need recognition, once counted. */
  pages?: number | null
}

export interface ProgressStage {
  id: StageId
  state: StageState
  unit: "origins" | "files" | "documents" | "templates" | "pages"
  /** Done, pending and failed. */
  total: number
  done: number
  pending: number
  failed: number
  /** Left out of the stage, not in `total`. */
  out: number
  /** Work no document stands for: a decision to apply, the pages to compute again. */
  otherWork: boolean
  /** What the engine is on now, when it is known. */
  current: StageItem | null
  error: ModelError | null
  /**
   * `questions` left at its last run; `grouped` and `loose` for the
   * matters; for the scans, the `pages` that wait, the documents whose
   * pages are not counted yet (`uncounted`) and those asked for (`requested`);
   * `firstLap` (1) while the matters or the pages wait for the engine's
   * first pass since the app opened (D-452); `held` (1) while its questions
   * wait for a person (D-484); for the reading, the documents read with
   * another version or model (`outdated`, D-465).
   */
  extra: Record<string, number>
}

export interface Progress {
  stages: ProgressStage[]
  /** Documents still being processed (a file not read yet counts as one). */
  inProcess: number
  /** Documents waiting because the automatic reading is paused or no model was chosen. */
  paused: number
  backgroundReading: boolean
  engineChosen: boolean
  /** The engine that recognizes scans, as it names its text; null when none was chosen. */
  recognitionEngine?: string | null
  lastError: ModelError | null
  /** The questions of «quién es quién» and of the summaries that go by themselves (D-484). */
  autoQuestions?: AutoQuestions | null
}

/**
 * The round of questions of «quién es quién» and of the summaries of the
 * pages that follow a change in what was read (D-484): they go by
 * themselves with the automatic reading on; past `cap`, a person says
 * whether to go on.
 */
export interface AutoQuestions {
  /** Questions left, as last counted. */
  pending: number
  /** Asked since the round began. */
  asked: number
  cap: number
  /** The stage whose questions wait for a person, when one does. */
  held: "resolve" | "pages" | null
  /** What a person said of the round. */
  decision: "allowed" | "later" | null
}

/** `go`: the questions of the round are asked now; otherwise they are left for later (D-484). */
export function decideAutoQuestions(go: boolean): Promise<void> {
  return invoke<void>("docbase_auto_questions_decide", { go })
}

export interface StageItems {
  stage: StageId
  items: StageItem[]
  total: number
}

export interface FolderCount {
  folder: string
  files: number
}

/** Every stage of the processing over `scope`. Reads only. */
export function getProgress(scope: ProgressScope | null = null): Promise<Progress> {
  return invoke<Progress>("docbase_progress", { scope })
}

/** The things one stage goes over, failed first, then pending, then done. */
export function listStageItems(stage: StageId, scope: ProgressScope | null = null, limit = 50): Promise<StageItems> {
  return invoke<StageItems>("docbase_progress_items", { stage, scope, limit })
}

/** The folders of an origin, each with the files under it. */
export function listProgressFolders(originId: string): Promise<FolderCount[]> {
  return invoke<FolderCount[]>("docbase_progress_folders", { originId })
}

export function listIdentityDoubts(): Promise<DoubtView[]> {
  return invoke<DoubtView[]>("docbase_knowledge_doubts")
}

export function listIdentityDecisions(): Promise<IdentityDecisionView[]> {
  return invoke<IdentityDecisionView[]>("docbase_knowledge_decisions")
}

/** Keeps and applies a decision; `shownEvidence` is what the screen showed (R-08). */
export function decideIdentity(
  decision: IdentityDecision,
  note: string | null = null,
  shownEvidence: string | null = null,
): Promise<number> {
  return invoke<number>("docbase_knowledge_decide", { decision, note, shownEvidence })
}

/** Several decisions at once, with one resolution. */
export function decideIdentityMany(decisions: IdentityDecision[], note: string | null = null): Promise<number[]> {
  return invoke<number[]>("docbase_knowledge_decide_many", { decisions, note })
}

export function undoIdentityDecision(id: number): Promise<void> {
  return invoke<void>("docbase_knowledge_undo", { id })
}

/** Reads again the documents whose reading failed; returns how many. */
export function knowledgeReadAgainFailed(): Promise<number> {
  return invoke<number>("docbase_knowledge_read_again_failed")
}

/**
 * What a person asks to read again with the model (D-432): the document at
 * a location, the documents of an origin or of a folder of it, or (nothing
 * said) the whole archive.
 */
export interface RereadTarget {
  locationId?: number | null
  originId?: string | null
  folder?: string | null
  /** Of those, only the ones read with another version of the question or with another model (D-465). */
  outdated?: boolean
}

/** The documents of the archive read with another version of the question or with another model (D-465). */
export const OUTDATED_READINGS: RereadTarget = { outdated: true }

/** What reading them again takes: documents already read and questions to the model. */
export interface RereadPlan {
  documents: number
  questions: number
  /** Of those, the ones already asked to read again. */
  requested: number
  /** Of those, the ones read with another version of the question, and with another model than the one in use; a document can be both. */
  olderVersion: number
  otherModel: number
  engine: AskConfig | null
}

export function getRereadPlan(target: RereadTarget = {}): Promise<RereadPlan> {
  return invoke<RereadPlan>("docbase_knowledge_reread_plan", { target })
}

/** Asks to read them again with new answers of the model; kept in the base until it is done. */
export function requestReread(target: RereadTarget = {}): Promise<number> {
  return invoke<number>("docbase_knowledge_reread", { target })
}

export function getKnowledgeAttention(): Promise<KnowledgeAttention> {
  return invoke<KnowledgeAttention>("docbase_knowledge_attention")
}

export function listMatters(): Promise<MatterSummary[]> {
  return invoke<MatterSummary[]>("docbase_knowledge_matters")
}

export function getMatter(id: string): Promise<MatterDetail | null> {
  return invoke<MatterDetail | null>("docbase_knowledge_matter", { id })
}

export function getDocumentEntities(locationId: number): Promise<DocumentEntities> {
  return invoke<DocumentEntities>("docbase_knowledge_document_entities", { locationId })
}

/** Takes a document (`content:<id>` or `location:<id>`) out of a matter, or turns down the model's proposal. */
export function leaveMatter(document: string, matterId: string, note: string | null = null): Promise<number[]> {
  return invoke<number[]>("docbase_knowledge_leave_matter", { document, matterId, note })
}

/**
 * Confirms a matter the grouping proposed, discards it (it stays away
 * while the same documents are grouped) or makes it a proposal again.
 */
export function decideMatter(matterId: string, decision: "confirm" | "discard" | "reopen"): Promise<void> {
  return invoke<void>("docbase_knowledge_decide_matter", { matterId, decision })
}

/** Puts a document in a matter, or confirms the model's proposal. */
export function joinMatter(document: string, matterId: string, note: string | null = null): Promise<number> {
  return invoke<number>("docbase_knowledge_join_matter", { document, matterId, note })
}

/** Creates a matter by hand, empty or with documents (`content:<id>` or `location:<id>`); returns its id. */
export function createMatter(name: string, documents: string[] = []): Promise<string> {
  return invoke<string>("docbase_knowledge_create_matter", { name, documents })
}

/** Names a matter by hand; an empty name gives a proposed matter back the app's name. */
export function renameMatter(matterId: string, name: string): Promise<void> {
  return invoke<void>("docbase_knowledge_rename_matter", { matterId, name })
}

/**
 * Puts a document in a matter (`in`) or takes it out (`out`), by hand: it
 * survives every grouping, and a proposed matter becomes confirmed.
 */
export function chooseMatterDocument(matterId: string, document: string, choice: "in" | "out"): Promise<void> {
  return invoke<void>("docbase_knowledge_choose_matter_document", { matterId, document, choice })
}
