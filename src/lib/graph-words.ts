/**
 * How the graph of the base says what its nodes and relations are, with
 * the words of the profession's vocabulary (D-410).
 */
import type { ClaimView } from "@/commands/docbase"
import { certaintyInWords } from "@/components/knowledge/knowledge"
import { entityTypeName, type Vocabulary, vocabulary } from "@/vocabulary"
import type { GraphNode, GraphRelation } from "./wiki-graph"

/** The kinds of node the base gives, in the order the legend lists them. */
export const GRAPH_NODE_TYPES = ["persona", "organizacion", "grupo", "asunto", "documento"] as const

/** What a node is, in singular or plural: "Persona", "Casos", "Documentos". */
export function nodeTypeName(type: string, many = false, words: Vocabulary = vocabulary()): string {
  switch (type) {
    case "documento":
      return many ? "Documentos" : "Documento"
    case "asunto":
      return many ? words.knowledge.matters : words.knowledge.matter
    case "grupo":
      return many ? words.knowledge.groups.many : words.knowledge.groups.one
    default:
      return entityTypeName(type, many, words)
  }
}

/** A count of nodes by what they are: "2 documentos, 1 persona". */
export function nodesInWords(nodes: readonly Pick<GraphNode, "type">[], words: Vocabulary = vocabulary()): string {
  const counts = new Map<string, number>()
  for (const node of nodes) counts.set(node.type, (counts.get(node.type) ?? 0) + 1)
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([type, count]) => `${count} ${nodeTypeName(type, count !== 1, words).toLowerCase()}`)
    .join(", ")
}

/** What a person decided about a matter, and its status: "Propuesto · Abierto". */
export function matterStateInWords(
  node: Pick<GraphNode, "confirmation" | "status">,
  words: Vocabulary = vocabulary(),
): string {
  const states = words.knowledge.matterStates
  const parts: string[] = []
  if (node.confirmation === "confirmed") parts.push(states.confirmed)
  else if (node.confirmation === "proposed") parts.push(states.proposed)
  if (node.status) parts.push(states.status[node.status] ?? node.status)
  return parts.join(" · ")
}

/** How sure the base is of a relation, as a person reads it. */
export function relationStateInWords(relation: Pick<GraphRelation, "family" | "state">): string {
  switch (relation.family) {
    case "mention":
      return "leído en el documento"
    case "claim":
      return certaintyInWords({ certainty: relation.state as ClaimView["certainty"], reasons: [] }).label
    case "similar":
      if (relation.state === "confirmed") return "confirmada por una persona"
      if (relation.state === "joined") return "unidos por una persona"
      return "propuesta por la app"
    case "matter":
      return relation.state === "firm" ? "firme" : "propuesto por la app"
    default:
      return relation.state === "confirmed" ? "confirmado por una persona" : "propuesto por la app"
  }
}
