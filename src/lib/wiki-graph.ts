import { invoke } from "@tauri-apps/api/core"
import type { PageObject } from "@/commands/docbase"
import { buildRetrievalGraph, calculateRelevance, pairKey } from "./graph-relevance"
import { detectCommunities } from "./wiki-graph-analysis"

/**
 * The graph of the archive, from the base (D-351, D-410): the people,
 * organizations, groups of companies and matters, each document with its
 * versions, and the relations the base keeps between them. Nothing is read
 * from files.
 */
export interface GraphNode {
  /** The object: `ent-…`, `mat-…` or `doc-…` (a document with its versions). */
  id: string
  label: string
  /** `persona`, `organizacion`, `grupo`, `asunto`, `documento`, or another kind of entity. */
  type: string
  linkCount: number // relations with other nodes
  community: number // community id from Louvain detection
  /** For a document: one place of it, to open its screen. */
  locationId?: number | null
  /** For a matter: `proposed` by the app or `confirmed` by a person. */
  confirmation?: string | null
  /** For a matter: its status (`open`...). */
  status?: string | null
}

export interface GraphEdge {
  source: string
  target: string
  weight: number // relevance score between source and target
  /** The relations of the base between the two. */
  relations?: GraphRelation[]
}

/** A relation of the base: its type, how the vocabulary names it, its quote and state. */
export interface GraphRelation {
  family: string
  kind: string
  label: string
  quote?: string | null
  state: string
}

interface BaseGraphNode {
  id: string
  label: string
  kind: string
  path: string | null
  locationId: number | null
  confirmation: string | null
  status: string | null
}

interface BaseGraphEdge extends GraphRelation {
  source: string
  target: string
}

export interface CommunityInfo {
  id: number
  nodeCount: number
  cohesion: number // intra-community edge density
  meanIntraDegree: number // average internal relations per node; scale-independent
  topNodes: string[] // top nodes by linkCount (labels)
}

export interface WikiGraphResult {
  nodes: GraphNode[]
  edges: GraphEdge[]
  communities: CommunityInfo[]
}

const COMMUNITY_WORKER_THRESHOLD = 500
let graphCache: { dataVersion: number; result: WikiGraphResult } | null = null
const graphBuilds = new Map<string, Promise<WikiGraphResult>>()

/** Forgets the graph built last, so the next one is read from the base again. */
export function clearGraphCache(): void {
  graphCache = null
}

/** The screen a node opens: its person or organization, matter or document. */
export function objectOfNode(node: GraphNode): PageObject {
  const kind = node.type === "asunto" ? "matter" : node.type === "documento" ? "document" : "entity"
  const content = kind === "document" ? Number(node.id.replace(/^doc-/, "")) : NaN
  return {
    objectId: node.id,
    kind,
    path: "",
    contentId: Number.isFinite(content) ? content : null,
    locationId: node.locationId ?? null,
  }
}

async function analyzeCommunities(
  nodes: { id: string; label: string; linkCount: number }[],
  edges: GraphEdge[],
): Promise<{ assignments: Map<string, number>; communities: CommunityInfo[] }> {
  if (nodes.length < COMMUNITY_WORKER_THRESHOLD || typeof Worker === "undefined") {
    return detectCommunities(nodes, edges)
  }

  return new Promise((resolve, reject) => {
    const worker = new Worker(
      new URL("./wiki-graph-analysis.worker.ts", import.meta.url),
      { type: "module" },
    )
    worker.onmessage = (event: MessageEvent<{
      assignments: [string, number][]
      communities: CommunityInfo[]
    }>) => {
      worker.terminate()
      resolve({
        assignments: new Map(event.data.assignments),
        communities: event.data.communities,
      })
    }
    worker.onerror = (event) => {
      worker.terminate()
      reject(new Error(event.message || "Knowledge graph analysis worker failed"))
    }
    worker.postMessage({ nodes, edges })
  })
}

/** The graph of the base, built once per `dataVersion`. */
export async function buildBaseGraph(dataVersion?: number): Promise<WikiGraphResult> {
  if (dataVersion !== undefined && graphCache?.dataVersion === dataVersion) return graphCache.result

  const buildKey = `${dataVersion ?? "uncached"}`
  const pending = graphBuilds.get(buildKey)
  if (pending) return pending
  const build = buildBaseGraphUncached()
  graphBuilds.set(buildKey, build)
  let result: WikiGraphResult
  try {
    result = await build
  } finally {
    graphBuilds.delete(buildKey)
  }
  // An older build may finish after a newer dataVersion: it never replaces it.
  if (dataVersion !== undefined && (!graphCache || graphCache.dataVersion <= dataVersion)) {
    graphCache = { dataVersion, result }
  }
  return result
}

/** The graph as the base gives it, with relevance and communities computed here. */
export async function graphFromBase(graph: { nodes: BaseGraphNode[]; edges: BaseGraphEdge[] }): Promise<WikiGraphResult> {
  const linkCounts = new Map<string, number>(graph.nodes.map((n) => [n.id, 0]))
  // One edge per pair of nodes, carrying every relation between them.
  const pairs = new Map<string, GraphEdge>()
  for (const edge of graph.edges) {
    if (edge.source === edge.target || !linkCounts.has(edge.source) || !linkCounts.has(edge.target)) continue
    const key = pairKey(edge.source, edge.target)
    const relation: GraphRelation = {
      family: edge.family,
      kind: edge.kind,
      label: edge.label,
      quote: edge.quote,
      state: edge.state,
    }
    const existing = pairs.get(key)
    if (existing) {
      existing.relations?.push(relation)
      continue
    }
    pairs.set(key, { source: edge.source, target: edge.target, weight: 1, relations: [relation] })
    linkCounts.set(edge.source, (linkCounts.get(edge.source) ?? 0) + 1)
    linkCounts.set(edge.target, (linkCounts.get(edge.target) ?? 0) + 1)
  }
  const types = graph.nodes.map((n) => ({ id: n.id, type: n.kind }))
  const edges = Array.from(pairs.values())
  const retrieval = buildRetrievalGraph(types, edges)
  for (const edge of edges) {
    const a = retrieval.nodes.get(edge.source)
    const b = retrieval.nodes.get(edge.target)
    if (a && b) edge.weight = calculateRelevance(a, b, retrieval)
  }
  const prelimNodes = graph.nodes.map((n) => ({ id: n.id, label: n.label, linkCount: linkCounts.get(n.id) ?? 0 }))
  const { assignments, communities } = await analyzeCommunities(prelimNodes, edges).catch(() =>
    detectCommunities(prelimNodes, edges),
  )
  const nodes: GraphNode[] = graph.nodes.map((n) => ({
    id: n.id,
    label: n.label,
    type: n.kind,
    linkCount: linkCounts.get(n.id) ?? 0,
    community: assignments.get(n.id) ?? 0,
    locationId: n.locationId,
    confirmation: n.confirmation,
    status: n.status,
  }))
  return { nodes, edges, communities }
}

async function buildBaseGraphUncached(): Promise<WikiGraphResult> {
  const graph = await invoke<{ nodes: BaseGraphNode[]; edges: BaseGraphEdge[] }>("docbase_graph")
  return graphFromBase(graph)
}
