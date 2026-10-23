import { invoke } from "@tauri-apps/api/core"

/**
 * The contract of operations (D-498, `src-tauri/src/operations`): what the
 * chat (and, next, the local API and the MCP) can ask of the base. A change
 * is only proposed: the app shows its card, a person confirms it, and it is
 * kept in the record of operations, undoable.
 */

/** One thing of the base a card shows for a person to choose («¿Cuál?»). */
export interface OperationCandidate {
  key: string
  name: string
  place?: string
  locationId?: number
  date?: string
}

/** What a card acts on: a case, or a new one. */
export interface OperationTarget {
  kind: "caso" | "nuevo_caso"
  named: string
  key: string | null
  name: string | null
  status: "ok" | "not_found" | "choose"
  candidates?: OperationCandidate[]
  confirmation: string | null
  statusWord: string | null
}

export type OperationItemStatus = "ok" | "unchanged" | "not_found" | "choose" | "blocked"

/** What a record shows when deciding about it (the screen of doubts shows the same). */
export interface OperationSide {
  key: string
  name: string
  kind: string
  aliases: string[]
  identifiers: string[]
  documents: string[]
  documentCount: number
}

export interface OperationItem {
  named: string
  key: string | null
  name: string | null
  place: string | null
  locationId: number | null
  date: string | null
  status: OperationItemStatus
  checked: boolean
  required: boolean
  reason: string | null
  candidates?: OperationCandidate[]
  detail?: OperationSide
}

export type OperationCardState = "proposed" | "applied" | "cancelled" | "undone" | "failed" | "expired"

/** A card, as the backend builds it from the base (never from what the model wrote). */
export interface OperationCard {
  id: number
  operation: string
  title: string
  request: string | null
  target: OperationTarget | null
  items: OperationItem[]
  warnings: string[]
  note: string | null
  /** What it costs, said before (reading again: questions to the model). */
  cost?: { documents: number; questions: number } | null
  secondConfirmation: boolean
  proposedBy: { caller: string; engine: string | null }
  state: OperationCardState
  fingerprint: string
  blocked: string | null
  result: string | null
  undoable: boolean
  undoNote: string | null
  open: { kind: "caso" | "ficha"; id: string } | null
  input: Record<string, unknown>
}

export interface OperationConfirmed {
  /** `applied`; `changed`: the base changed since the card was shown; `needs_second`: more than ten things. */
  outcome: "applied" | "changed" | "needs_second"
  card: OperationCard
}

export interface OperationUndone {
  /** `undone`, or `changed`: something changed since it was done, and undoing waits for the person. */
  outcome: "undone" | "changed"
  card: OperationCard
}

/** The card as it is now (one that waits is recomputed with the base). */
export function getOperationCard(id: number): Promise<OperationCard> {
  return invoke<OperationCard>("operations_card", { id })
}

/** A person chose a candidate: `item` is what the model named, or `target` for the case. */
export function chooseOperationCandidate(id: number, item: string, candidate: string): Promise<OperationCard> {
  return invoke<OperationCard>("operations_choose", { id, item, candidate })
}

/** A person confirmed a card with what they left ticked and the fingerprint they saw. */
export function confirmOperation(id: number, chosen: string[], fingerprint: string, secondConfirmation = false): Promise<OperationConfirmed> {
  return invoke<OperationConfirmed>("operations_confirm", { id, chosen, fingerprint, secondConfirmation })
}

export function cancelOperation(id: number): Promise<OperationCard> {
  return invoke<OperationCard>("operations_cancel", { id })
}

/** Undoes what a card did; `force` after being told something changed since. */
export function undoOperation(id: number, force = false): Promise<OperationUndone> {
  return invoke<OperationUndone>("operations_undo", { id, force })
}

/** A button of a screen does an operation of the contract at once (the person decides), kept in the record. */
export function applyOperation(operation: string, data: Record<string, unknown>): Promise<OperationCard> {
  return invoke<OperationCard>("operations_apply", { operation, data })
}

/** The turns of the chat's agent: how many it used, of its cap, and where «Seguir buscando» goes on from. */
export interface AgentBudget {
  used: number
  max: number
  reached: boolean
  continuation?: string
}

/** The line the history of the next message carries about a card (11.6). */
export function operationHistoryLine(card: OperationCard): string | null {
  const state: Partial<Record<OperationCardState, string>> = {
    applied: "hecha (la confirmó la persona)",
    cancelled: "cancelada por la persona: no se hizo",
    undone: "hecha y después deshecha por la persona",
    proposed: "propuesta, sin confirmar todavía",
    failed: "no se pudo hacer",
  }
  const word = state[card.state]
  return word ? `[Acción] ${card.title}: ${word}` : null
}
