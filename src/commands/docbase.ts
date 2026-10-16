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
}

/** A label: an answer ("BGH") to its group's question ("Cliente"), or standing alone. */
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
  /** Every location of this document, this one first. */
  locations: LocationRef[]
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

/** Totals of the general project's wiki, from the base's index of it. */
export interface WikiIndex {
  pages: number
  links: number
  /** Links that point at no page. */
  broken: number
}

/** A wiki page, as the index knows it. */
export interface PageRef {
  /** Path inside the project: `wiki/entities/bgh.md`. */
  path: string
  /** Where the page is on disk. */
  file: string
  title: string
  /** `type` of its header: source, entity, concept… */
  kind: string
}

export interface BrokenLink {
  from: PageRef
  /** What the link says, as written. */
  target: string
}

export interface Home {
  health: Health
  origins: OriginSummary[]
  activity: Activity[]
  wiki: WikiIndex
}

/** The wiki pages that came from the document at this location, or from any copy of it. */
export function pagesCiting(locationId: number): Promise<PageRef[]> {
  return invoke<PageRef[]>("docbase_pages_citing", { locationId })
}

export function listBrokenLinks(): Promise<BrokenLink[]> {
  return invoke<BrokenLink[]>("docbase_broken_links")
}

/** The pages that link to the page at this path inside the project. */
export function listBacklinks(pagePath: string): Promise<PageRef[]> {
  return invoke<PageRef[]>("docbase_backlinks", { pagePath })
}

export function getHome(): Promise<Home> {
  return invoke<Home>("docbase_home")
}

/** An origin as the general project's wiki sees it. */
export interface BridgedOrigin {
  originId: string
  name: string
  /** The folder under `raw/sources` it appears as, when it is mounted. */
  mountedAs: string | null
  /** Why it could not be mounted, when that is the case. */
  problem: string | null
}

/** The wiki that covers every origin of the space. */
export interface GeneralProject {
  id: string
  name: string
  /** The wiki project folder, once one was assigned. */
  path: string | null
  members: string
  /** Whether the wiki is fed from the base on its own. */
  wikiActive: boolean
  origins: BridgedOrigin[]
}

/** A file of the base as a source of the general project's wiki. */
export interface WikiSource {
  originId: string
  /** Path inside the wiki project: `raw/sources/<origin>/<file>`. */
  sourcePath: string
  /** What the wiki calls the source: the path under `raw/sources`. */
  identity: string
  ext: string
  /** The document it is, once the base read it. */
  contentId: number | null
}

/** `null` before the first origin is incorporated. */
export function getGeneralProject(): Promise<GeneralProject | null> {
  return invoke<GeneralProject | null>("docbase_general_project")
}

/** Mounts every origin in that wiki project. Copies nothing, ingests nothing. */
export function linkGeneralProject(projectPath: string, name: string): Promise<GeneralProject> {
  return invoke<GeneralProject>("docbase_link_general_project", { projectPath, name })
}

/** Makes sure the general project has its wiki in the workspace; creates it the first time. */
export function ensureDefaultProject(workspace: string): Promise<GeneralProject> {
  return invoke<GeneralProject>("docbase_ensure_default_project", { workspace })
}

export function setWikiActive(active: boolean): Promise<void> {
  return invoke<void>("docbase_set_wiki_active", { active })
}

export function unlinkGeneralProject(): Promise<void> {
  return invoke<void>("docbase_unlink_general_project")
}

export function listWikiSources(): Promise<WikiSource[]> {
  return invoke<WikiSource[]>("docbase_wiki_sources")
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

/** Which of these `raw/sources/…` paths of a wiki project the user left out of the base. */
export function excludedSources(projectPath: string, sourcePaths: string[]): Promise<string[]> {
  return invoke<string[]>("docbase_excluded_sources", { projectPath, sourcePaths })
}

export function openFolder(originId: string, folder: string): Promise<void> {
  return invoke<void>("docbase_open_folder", { originId, folder })
}

export function listDocuments(filter: DocumentFilter): Promise<DocumentPage> {
  return invoke<DocumentPage>("docbase_list_documents", { filter })
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
}

export type SimilarDecision =
  | { kind: "join"; relation: RelationId; wikiSource: number | null }
  | { kind: "discard" }
  | { kind: "undo" }

export interface SimilarPending {
  /** Pairs with a proposed relation and no decision. */
  pairs: number
  /** Families with at least one such pair. */
  groups: number
  /**
   * Of `pairs`, those whose proposed relation makes both one document
   * (practically the same, signed copy, version): deciding them changes
   * which one feeds the wiki. The rest are information.
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
  pending: number
  /** Of `pending`, those whose proposed relation makes both one document. */
  pendingSameDocument: number
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

export function getSimilarPending(): Promise<SimilarPending> {
  return invoke<SimilarPending>("docbase_similar_pending")
}

export function listSimilarGroups(): Promise<SimilarGroup[]> {
  return invoke<SimilarGroup[]>("docbase_similar_groups")
}

export function listSimilarRelations(): Promise<RelationDef[]> {
  return invoke<RelationDef[]>("docbase_similar_relations")
}

/** Which engine answers the questions about documents, and with which model. */
export interface AskConfig {
  engine: "codex-cli"
  model: string
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
}

export function getTypesPlan(): Promise<TypesPlan> {
  return invoke<TypesPlan>("docbase_types_plan")
}

/** The types the model proposed that wait for a person. Computes nothing. */
export function listProposedTypes(): Promise<ProposedType[]> {
  return invoke<ProposedType[]>("docbase_proposed_types")
}

/** Asks the model what is pending. Costs the provider's quota: only on an explicit action. */
export function runTypes(engine: AskConfig, maxQuestions?: number): Promise<TypesRunReport> {
  return invoke<TypesRunReport>("docbase_types_run", { engine, maxQuestions: maxQuestions ?? null })
}

/** The engine that answers while the archive's wiki is active; null stops it. */
export function setAskEngine(engine: AskConfig | null): Promise<void> {
  return invoke<void>("docbase_set_ask_engine", { engine })
}

export function getDocumentType(locationId: number): Promise<DocumentTypeView> {
  return invoke<DocumentTypeView>("docbase_document_type", { locationId })
}

export function listDocumentTypes(): Promise<DocTypeDef[]> {
  return invoke<DocTypeDef[]>("docbase_document_types")
}
