import { type Vocabulary, vocabulary } from "@/vocabulary"
import { nodesInWords } from "./graph-words"
import type { GraphNode, GraphEdge, CommunityInfo } from "./wiki-graph"

/**
 * What the shape of the graph of the base points out (D-410): relations
 * that join distant parts of the archive, and the gaps and key points of
 * its communities. The texts use the profession's vocabulary; they never
 * ask to write links: every relation comes from the documents.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SurprisingConnection {
  source: GraphNode
  target: GraphNode
  score: number
  reasons: string[]
  key: string // stable ID for dismiss tracking
}

export interface KnowledgeGap {
  type: "isolated-node" | "sparse-community" | "bridge-node"
  title: string
  description: string
  nodeIds: string[]
  suggestion: string
}

/**
 * The node a research of a gap is about: the isolated node, or the most
 * related one of the gap (the same one its topic names). Its object is
 * what the research is kept of (P-236).
 */
export function researchNodeForKnowledgeGap(
  gap: KnowledgeGap,
  nodes: readonly GraphNode[],
): GraphNode | null {
  const candidates = gap.nodeIds
    .map((id) => nodes.find((node) => node.id === id))
    .filter((node): node is GraphNode => Boolean(node))
  if (gap.type !== "isolated-node") candidates.sort((a, b) => b.linkCount - a.linkCount)
  return candidates[0] ?? null
}

export function researchSeedForKnowledgeGap(
  gap: KnowledgeGap,
  nodes: readonly GraphNode[],
): { term: string; context: string } {
  const candidates = gap.nodeIds
    .map((id) => nodes.find((node) => node.id === id))
    .filter((node): node is GraphNode => Boolean(node))

  if (gap.type === "isolated-node") {
    return { term: candidates[0]?.label || gap.title, context: "" }
  }

  candidates.sort((a, b) => b.linkCount - a.linkCount)
  const representative = candidates[0]?.label || gap.title
  const related = candidates.slice(1, 3).map((node) => node.label)
  return {
    term: representative,
    context: related.length > 0 ? related.join(" ") : "",
  }
}

function formatDecimal(value: number): string {
  return value.toLocaleString("es-AR", { minimumFractionDigits: 1, maximumFractionDigits: 1 })
}

// ---------------------------------------------------------------------------
// Surprising Connections
// ---------------------------------------------------------------------------

/**
 * Relations that stand out: between two communities, between a node with
 * few relations and one of the most related, or among the weakest.
 */
export function findSurprisingConnections(
  nodes: GraphNode[],
  edges: GraphEdge[],
  _communities: CommunityInfo[],
  limit: number = 5,
): SurprisingConnection[] {
  const nodeMap = new Map(nodes.map((n) => [n.id, n]))
  const maxDegree = Math.max(...nodes.map((n) => n.linkCount), 1)
  const weights = edges.map((edge) => edge.weight).sort((a, b) => a - b)
  // The weakest quarter of the relations.
  const weak = weights.length >= 4 ? weights[Math.floor(weights.length / 4)] : -Infinity

  const scored: SurprisingConnection[] = []

  for (const edge of edges) {
    const source = nodeMap.get(edge.source)
    const target = nodeMap.get(edge.target)
    if (!source || !target) continue

    let score = 0
    const reasons: string[] = []

    if (source.community !== target.community) {
      score += 3
      reasons.push("une dos comunidades distintas")
    }

    const minDeg = Math.min(source.linkCount, target.linkCount)
    const maxDeg = Math.max(source.linkCount, target.linkCount)
    if (minDeg <= 2 && maxDeg >= maxDegree * 0.5) {
      score += 2
      reasons.push("une a uno con pocas relaciones con uno de los más relacionados")
    }

    if (edge.weight > 0 && edge.weight <= weak) {
      score += 1
      reasons.push("es de las relaciones más débiles")
    }

    if (score >= 3 && reasons.length > 0) {
      const key = [source.id, target.id].sort().join(":::")
      scored.push({ source, target, score, reasons, key })
    }
  }

  scored.sort((a, b) => b.score - a.score)
  return scored.slice(0, limit)
}

// ---------------------------------------------------------------------------
// Knowledge Gaps
// ---------------------------------------------------------------------------

/**
 * Gaps and key points of the graph:
 * - nodes without relations (a document nothing was read from, an entity
 *   no visible document names);
 * - communities whose nodes are barely related to each other (fewer than
 *   two relations inside per node, with three nodes or more);
 * - key points: nodes related to three communities or more.
 */
export function detectKnowledgeGaps(
  nodes: GraphNode[],
  edges: GraphEdge[],
  communities: CommunityInfo[],
  limit: number = 8,
  words: Vocabulary = vocabulary(),
): KnowledgeGap[] {
  const gaps: KnowledgeGap[] = []
  const nodeMap = new Map(nodes.map((n) => [n.id, n]))

  const isolatedNodes = nodes.filter((n) => n.linkCount === 0)
  if (isolatedNodes.length > 0) {
    const topIsolated = isolatedNodes.slice(0, 5)
    gaps.push({
      type: "isolated-node",
      title: `Sin relaciones: ${nodesInWords(isolatedNodes, words)}`,
      description: topIsolated.map((n) => n.label).join(", ") +
        (isolatedNodes.length > 5 ? ` y ${isolatedNodes.length - 5} más` : ""),
      nodeIds: isolatedNodes.map((n) => n.id),
      suggestion: "La base no los relaciona con nada: puede ser un documento del que no se leyó a nadie, o un nombre que ningún documento visible menciona. Abrí su ficha para revisarlo.",
    })
  }

  // Mean internal degree, not raw density: density shrinks as communities grow.
  for (const comm of communities) {
    if (comm.meanIntraDegree < 2 && comm.nodeCount >= 3) {
      const members = nodes.filter((n) => n.community === comm.id)
      gaps.push({
        type: "sparse-community",
        title: `Comunidad poco relacionada: ${comm.topNodes[0] ?? `comunidad ${comm.id + 1}`}`,
        description: `${nodesInWords(members, words)}, con ${formatDecimal(comm.meanIntraDegree)} relaciones internas en promedio.`,
        nodeIds: members.map((n) => n.id),
        suggestion: `Lo que la une es poco: pueden ser documentos de temas distintos, o documentos que todavía no se agruparon en ${words.knowledge.matters.toLowerCase()}.`,
      })
    }
  }

  const communityNeighbors = new Map<string, Set<number>>()
  for (const node of nodes) {
    communityNeighbors.set(node.id, new Set())
  }
  for (const edge of edges) {
    const sourceNode = nodeMap.get(edge.source)
    const targetNode = nodeMap.get(edge.target)
    if (sourceNode && targetNode) {
      communityNeighbors.get(edge.source)?.add(targetNode.community)
      communityNeighbors.get(edge.target)?.add(sourceNode.community)
    }
  }

  const bridgeNodes = nodes
    .filter((n) => (communityNeighbors.get(n.id)?.size ?? 0) >= 3)
    .sort((a, b) => (communityNeighbors.get(b.id)?.size ?? 0) - (communityNeighbors.get(a.id)?.size ?? 0))
    .slice(0, 3)

  for (const bridge of bridgeNodes) {
    const commCount = communityNeighbors.get(bridge.id)?.size ?? 0
    gaps.push({
      type: "bridge-node",
      title: `Punto clave: ${bridge.label}`,
      description: `Se relaciona con ${commCount} comunidades distintas.`,
      nodeIds: [bridge.id],
      suggestion: "Lo que se diga de este nodo toca varias comunidades a la vez: conviene que su ficha esté revisada.",
    })
  }

  return gaps.slice(0, limit)
}
