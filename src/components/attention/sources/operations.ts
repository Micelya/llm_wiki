import { listProgramRequests, type OperationCard } from "@/commands/operations"
import type { AttentionNotice } from "@/lib/attention/types"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"

/**
 * What the programs connected to the server inside the machine raise
 * (D-495, the contract's design 6.4): a change an assistant or a script
 * asked for waits for a person, with the same card as the chat's. Nothing
 * is done until she confirms it; one nobody decides expires in 7 days.
 */
export const OPERATIONS_SOURCE = "operations"

/** Requests listed when the line is opened; the rest are in its screen. */
const LISTED = 30

/** Who asked, by the name the program had. */
export function requesterOf(card: OperationCard): string {
  return card.proposedBy.engine || card.proposedBy.caller.replace(/^program:/, "")
}

export function programRequestNotices(cards: OperationCard[] | null): AttentionNotice[] {
  if (!cards || cards.length === 0) return []
  const go = { kind: "go" as const, label: "Revisar", target: { view: "program-requests" as const } }
  return [
    {
      source: OPERATIONS_SOURCE,
      id: `${OPERATIONS_SOURCE}:program-requests`,
      kind: "decision",
      topic: "program-requests",
      priority: "normal",
      count: cards.length,
      title: plural(cards.length, "pedido de otro programa espera tu confirmación", "pedidos de otros programas esperan tu confirmación"),
      explanation:
        "Un programa conectado a Micelya (por ejemplo, un asistente) pidió cambiar algo en la base. No se hizo nada: cada pedido espera a que una persona lo revise.",
      todo: "Mirá cada pedido con lo que va a tocar, destildá lo que no quieras y confirmalo o cancelalo.",
      resolvedWhen: "Desaparece cuando cada pedido se confirma o se cancela; uno sin decidir vence a los 7 días.",
      actions: [go],
      cases: cards.slice(0, LISTED).map((card) => ({
        id: `program-request:${card.id}`,
        title: card.title,
        detail: `Lo pidió «${requesterOf(card)}»`,
        actions: [go],
      })),
      moreCases: Math.max(0, cards.length - LISTED),
    },
  ]
}

export function useProgramRequestNotices(): AttentionNotice[] | null {
  const query = useDocbaseQuery("program-requests", listProgramRequests)
  if (query.error) throw new Error(query.error)
  if (query.data === null) return null
  return programRequestNotices(query.data)
}
