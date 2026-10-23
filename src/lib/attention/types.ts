import type { DocumentFilter } from "@/commands/docbase"

/**
 * The shape every part of the app gives what needs a person's attention.
 *
 * Each part keeps detecting its own things (the document base, the
 * wiki's queue, its review, its link check; later the entities,
 * deadlines, tasks…) and hands them over as notices of this shape. The
 * attention view only gathers and orders them (design, attention view,
 * section 7). A notice exists only while what it says is true: a source
 * publishes its whole list again whenever something changes, and what it
 * leaves out is gone.
 */

/** The three classes of the view, which never mix. */
export type NoticeKind =
  /** Something does not work and the app cannot fix it alone. */
  | "problem"
  /** Something waits for a person's judgement. */
  | "decision"
  /** What the app is doing. Shown to know what happens and to pause it; it needs no attention. */
  | "ongoing"

/**
 * Inside its class. `low` lines are shown, marked as such, and left out
 * of the counters: they explain something that stays out, or that can
 * wait, without asking for anything urgent.
 */
export type NoticePriority = "high" | "normal" | "low"

/** A place of the app where something is done, never a list to search in. */
export type AttentionTarget =
  | { view: "origins" }
  | { view: "documents"; filter: DocumentFilter }
  | { view: "document"; locationId: number }
  /** The similar documents, on one family when `principal` (its principal's content id) is given. */
  | { view: "similar"; principal?: number | null }
  | { view: "document-types" }
  | { view: "settings" }
  /** The attention view, on one of its tabs when `tab` is given. */
  | { view: "attention"; tab?: "problem" | "decision" | "ongoing" }
  /** A page of the wiki, by its path inside the project. */
  | { view: "wiki-page"; path: string }
  /** The people and organizations, on one entity when `entityId` is given. */
  | { view: "entities"; entityId?: string | null }
  /** The doubtful identities, on one group of doubts when `group` is given. */
  | { view: "identity-doubts"; group?: string | null }
  /** The matters, on one matter when `matterId` is given. */
  | { view: "matters"; matterId?: string | null }
  /** The judicial portals («Portales»). */
  | { view: "portals" }
  /** The changes other programs asked for, each with its card (D-495). */
  | { view: "program-requests" }

/** A button of a line or of one of its cases. */
export type AttentionAction =
  | {
      kind: "go"
      label: string
      target: AttentionTarget
    }
  | {
      kind: "run"
      label: string
      run: () => unknown | Promise<unknown>
      /** When set, a confirmation with this message comes first: say what is lost. */
      confirm?: string
      /** A destructive action is drawn as such. */
      danger?: boolean
    }

/** One of the things a line groups, shown when the line is opened. */
export interface AttentionCase {
  id: string
  title: string
  /** Its own reason, when it has one: the error of a document, its folder. */
  detail?: string
  actions?: AttentionAction[]
}

export interface AttentionNotice {
  /** Unique across sources and stable while it is true: `<source>:<topic>`. */
  id: string
  /** The source that raised it (`docbase`, `wiki-queue`…). */
  source: string
  kind: NoticeKind
  /** What it is about, in one word (`origins`, `similar`…): a line groups one topic. */
  topic: string
  priority: NoticePriority
  /**
   * How many things it groups. For a decision, how many decisions a
   * person makes (a family is one decision, not one per document); it is
   * what the counters add up.
   */
  count: number
  /** What happens: "3 documentos no se pudieron incorporar al wiki". */
  title: string
  /** Why it matters. */
  explanation: string
  /** What to do. */
  todo: string
  /** When it goes away by itself: "Desaparece cuando la carpeta vuelve a leerse". */
  resolvedWhen: string
  /** The actions in the row, the main one first. */
  actions: AttentionAction[]
  /** The cases one by one, when they are at hand. */
  cases?: AttentionCase[]
  /** Or how to get them, when they cost a query; asked only when the line is opened. */
  loadCases?: () => Promise<AttentionCase[]>
  /** Cases that exist beyond the ones listed. */
  moreCases?: number
  /** For what is in progress. */
  progress?: { done: number; total: number }
  /** Whose it is when it is not the archive's: "Proyecto «Pericias»". */
  scope?: string
}

/** A part of the app that raises notices. See `src/components/attention/sources/`. */
export interface AttentionSourceInfo {
  id: string
  /** Who raises it, in words, for when it fails: "la base documental". */
  label: string
}
