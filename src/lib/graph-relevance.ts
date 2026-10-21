/**
 * How related two nodes of the graph of the base are (D-410): computed
 * from the relations the base keeps, in memory, without reading files.
 *
 * Four signals add up:
 * - the relations that join them directly (a role in a document, what the
 *   documents say between two entities, a matter and its document...);
 * - the documents they share: two people named in the same documents, or
 *   a document and who it names;
 * - the neighbours they share (Adamic-Adar: a shared neighbour with few
 *   relations counts more than a very connected one);
 * - how near their kinds are (a matter and a document, a group and an
 *   organization).
 */

export interface RetrievalNode {
  readonly id: string
  readonly type: string
  /** The documents it is in: itself for a document, the ones it is related to otherwise. */
  readonly documents: ReadonlySet<string>
  readonly neighbors: ReadonlySet<string>
}

export interface RetrievalGraph {
  readonly nodes: ReadonlyMap<string, RetrievalNode>
  /** How many relations of the base join each pair, by `pairKey`. */
  readonly relations: ReadonlyMap<string, number>
}

const WEIGHTS = {
  directRelation: 3.0,
  sharedDocument: 4.0,
  commonNeighbor: 1.5,
  typeAffinity: 1.0,
} as const

/** More relations between the same two add little after this many. */
const MAX_DIRECT_RELATIONS = 3

const TYPE_AFFINITY: Record<string, Record<string, number>> = {
  persona: { persona: 0.8, organizacion: 1.0, grupo: 0.8, documento: 1.0, asunto: 1.0 },
  organizacion: { persona: 1.0, organizacion: 0.8, grupo: 1.2, documento: 1.0, asunto: 1.0 },
  grupo: { persona: 0.8, organizacion: 1.2, grupo: 0.5, documento: 0.8, asunto: 0.8 },
  documento: { persona: 1.0, organizacion: 1.0, grupo: 0.8, documento: 0.5, asunto: 1.2 },
  asunto: { persona: 1.0, organizacion: 1.0, grupo: 0.8, documento: 1.2, asunto: 0.5 },
}

const DOCUMENT = "documento"

export function pairKey(a: string, b: string): string {
  return a < b ? `${a}:::${b}` : `${b}:::${a}`
}

/** The graph the relevance is computed on: each node with its neighbours and documents. */
export function buildRetrievalGraph(
  nodes: readonly { id: string; type: string }[],
  edges: readonly { source: string; target: string; relations?: readonly unknown[] }[],
): RetrievalGraph {
  const types = new Map(nodes.map((node) => [node.id, node.type]))
  const neighbors = new Map<string, Set<string>>(nodes.map((node) => [node.id, new Set<string>()]))
  const relations = new Map<string, number>()
  for (const edge of edges) {
    if (edge.source === edge.target || !types.has(edge.source) || !types.has(edge.target)) continue
    neighbors.get(edge.source)?.add(edge.target)
    neighbors.get(edge.target)?.add(edge.source)
    const key = pairKey(edge.source, edge.target)
    relations.set(key, (relations.get(key) ?? 0) + Math.max(1, edge.relations?.length ?? 1))
  }
  const out = new Map<string, RetrievalNode>()
  for (const node of nodes) {
    const around = neighbors.get(node.id) ?? new Set<string>()
    const documents = node.type === DOCUMENT
      ? new Set([node.id])
      : new Set([...around].filter((id) => types.get(id) === DOCUMENT))
    out.set(node.id, { id: node.id, type: node.type, documents, neighbors: around })
  }
  return { nodes: out, relations }
}

export function calculateRelevance(
  nodeA: RetrievalNode,
  nodeB: RetrievalNode,
  graph: RetrievalGraph,
): number {
  if (nodeA.id === nodeB.id) return 0

  const direct = Math.min(graph.relations.get(pairKey(nodeA.id, nodeB.id)) ?? 0, MAX_DIRECT_RELATIONS)
  const directScore = direct * WEIGHTS.directRelation

  let sharedDocuments = 0
  for (const document of nodeB.documents) {
    if (nodeA.documents.has(document)) sharedDocuments += 1
  }
  const sharedDocumentScore = sharedDocuments * WEIGHTS.sharedDocument

  let adamicAdar = 0
  for (const neighborId of nodeA.neighbors) {
    if (!nodeB.neighbors.has(neighborId)) continue
    const neighbor = graph.nodes.get(neighborId)
    if (neighbor) adamicAdar += 1 / Math.log(Math.max(neighbor.neighbors.size, 2))
  }
  const commonNeighborScore = adamicAdar * WEIGHTS.commonNeighbor

  const typeAffinityScore = (TYPE_AFFINITY[nodeA.type]?.[nodeB.type] ?? 0.5) * WEIGHTS.typeAffinity

  return directScore + sharedDocumentScore + commonNeighborScore + typeAffinityScore
}
