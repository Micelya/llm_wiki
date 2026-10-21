import { describe, expect, it } from "vitest"
import type { GraphEdge, GraphNode } from "@/lib/wiki-graph"
import { applyGraphSearch } from "./graph-search"

const nodes: GraphNode[] = [
  { id: "mat-1", label: "Cobro de honorarios", type: "asunto", linkCount: 2, community: 0, confirmation: "proposed", status: "open" },
  { id: "ent-1", label: "Ana Prueba", type: "persona", linkCount: 1, community: 0 },
  { id: "doc-3", label: "Contrato de locación", type: "documento", linkCount: 1, community: 1 },
]

const edges: GraphEdge[] = [
  { source: "mat-1", target: "ent-1", weight: 1 },
  { source: "mat-1", target: "doc-3", weight: 1 },
]

describe("applyGraphSearch", () => {
  it("returns the original graph for an empty query", () => {
    const out = applyGraphSearch(nodes, edges, " ")
    expect(out.nodes).toHaveLength(3)
    expect(out.edges).toHaveLength(2)
    expect(out.matchedNodeIds.size).toBe(0)
  })

  it("matches by label, id, and what the node is", () => {
    expect(applyGraphSearch(nodes, edges, "honorarios").nodes.map((n) => n.id)).toEqual(["mat-1"])
    expect(applyGraphSearch(nodes, edges, "persona").nodes.map((n) => n.id)).toEqual(["ent-1"])
    expect(applyGraphSearch(nodes, edges, "documento").nodes.map((n) => n.id)).toEqual(["doc-3"])
    expect(applyGraphSearch(nodes, edges, "doc-3").nodes.map((n) => n.id)).toEqual(["doc-3"])
  })

  it("keeps only edges between matched nodes", () => {
    const out = applyGraphSearch(nodes, edges, "o")
    expect(out.nodes).toHaveLength(3)
    expect(out.edges).toHaveLength(2)

    const documentOnly = applyGraphSearch(nodes, edges, "locación")
    expect(documentOnly.nodes.map((n) => n.id)).toEqual(["doc-3"])
    expect(documentOnly.edges).toHaveLength(0)
  })
})
