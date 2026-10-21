import { describe, expect, it } from "vitest"
import type { GraphEdge, GraphNode } from "@/lib/wiki-graph"
import { applyGraphFilters, DEFAULT_GRAPH_FILTERS, hasActiveGraphFilters, type GraphFilterState } from "./graph-filters"

const nodes: GraphNode[] = [
  { id: "index", label: "Index", type: "asunto", linkCount: 4, community: 0, confirmation: "proposed", status: "open" },
  { id: "concept-a", label: "Concept A", type: "organizacion", linkCount: 2, community: 0 },
  { id: "entity-b", label: "Entity B", type: "persona", linkCount: 3, community: 0 },
  { id: "source-c", label: "Source C", type: "documento", linkCount: 1, community: 1 },
  { id: "isolated", label: "Isolated", type: "organizacion", linkCount: 0, community: 2 },
]

const edges: GraphEdge[] = [
  { source: "index", target: "concept-a", weight: 1 },
  { source: "index", target: "entity-b", weight: 1 },
  { source: "concept-a", target: "entity-b", weight: 2 },
  { source: "source-c", target: "entity-b", weight: 3 },
]

function makeFilters(overrides: Partial<GraphFilterState> = {}): GraphFilterState {
  return {
    ...DEFAULT_GRAPH_FILTERS,
    hiddenTypes: new Set<string>(),
    hiddenNodeIds: new Set<string>(),
    hiddenConfirmations: new Set<string>(),
    ...overrides,
  }
}

describe("graph filters", () => {
  it("hides the matters by what a person decided of them", () => {
    const out = applyGraphFilters(nodes, edges, makeFilters({ hiddenConfirmations: new Set(["proposed"]) }))

    expect(out.nodes.map((n) => n.id)).not.toContain("index")
    expect(out.edges).toEqual([
      { source: "concept-a", target: "entity-b", weight: 2 },
      { source: "source-c", target: "entity-b", weight: 3 },
    ])
    const confirmedHidden = applyGraphFilters(nodes, edges, makeFilters({ hiddenConfirmations: new Set(["confirmed"]) }))
    expect(confirmedHidden.nodes.map((n) => n.id)).toContain("index")
  })

  it("hides selected node types", () => {
    const out = applyGraphFilters(nodes, edges, makeFilters({
      hiddenTypes: new Set(["documento"]),
    }))

    expect(out.nodes.map((n) => n.id)).not.toContain("source-c")
    expect(out.edges.some((e) => e.source === "source-c" || e.target === "source-c")).toBe(false)
  })

  it("hides manually selected nodes", () => {
    const out = applyGraphFilters(nodes, edges, makeFilters({
      hiddenNodeIds: new Set(["entity-b"]),
    }))

    expect(out.nodes.map((n) => n.id)).not.toContain("entity-b")
    expect(out.edges).toEqual([{ source: "index", target: "concept-a", weight: 1 }])
  })

  it("hides hub nodes above the max link threshold", () => {
    const out = applyGraphFilters(nodes, edges, makeFilters({
      maxLinks: 2,
    }))

    expect(out.nodes.map((n) => n.id)).not.toContain("index")
    expect(out.nodes.map((n) => n.id)).not.toContain("entity-b")
    expect(out.edges).toEqual([])
  })

  it("hides weakly connected nodes below the min link threshold", () => {
    const out = applyGraphFilters(nodes, edges, makeFilters({
      minLinks: 2,
    }))

    expect(out.nodes.map((n) => n.id)).not.toContain("source-c")
    expect(out.nodes.map((n) => n.id)).not.toContain("isolated")
    expect(out.nodes.map((n) => n.id)).toEqual(["index", "concept-a", "entity-b"])
    expect(out.edges).toEqual([
      { source: "index", target: "concept-a", weight: 1 },
      { source: "index", target: "entity-b", weight: 1 },
      { source: "concept-a", target: "entity-b", weight: 2 },
    ])
  })

  it("hides isolated nodes when requested", () => {
    const out = applyGraphFilters(nodes, edges, makeFilters({
      hideIsolated: true,
    }))

    expect(out.nodes.map((n) => n.id)).not.toContain("isolated")
  })

  it("reports whether filters are active", () => {
    expect(hasActiveGraphFilters(makeFilters())).toBe(false)
    expect(hasActiveGraphFilters(makeFilters({ hiddenNodeIds: new Set(["x"]) }))).toBe(true)
    expect(hasActiveGraphFilters(makeFilters({ minLinks: 2 }))).toBe(true)
    expect(hasActiveGraphFilters(makeFilters({ hiddenConfirmations: new Set(["proposed"]) }))).toBe(true)
  })
})
