/**
 * The graph of the archive comes from the base (D-351, D-410): nodes are
 * its objects (people, organizations, groups, documents, matters with what
 * a person decided of them) and edges the relations the base keeps, with
 * their type, quote and state. No file is read.
 */
import { describe, it, expect, vi, beforeEach } from "vitest"

const mockInvoke = vi.fn()

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}))

const base = {
  nodes: [
    { id: "doc-1", label: "Contrato de prueba", kind: "documento", path: "wiki/sources/contrato-1.md", locationId: 7, confirmation: null, status: null },
    { id: "ent-a", label: "Ana Prueba", kind: "persona", path: null, locationId: null, confirmation: null, status: null },
    { id: "ent-b", label: "Beto Ensayo", kind: "persona", path: null, locationId: null, confirmation: null, status: null },
    { id: "mat-1", label: "Cobro de prueba", kind: "asunto", path: null, locationId: null, confirmation: "proposed", status: "open" },
  ],
  edges: [
    { source: "doc-1", target: "ent-a", family: "mention", kind: "parte", label: "parte", quote: "Ana", state: "read" },
    { source: "doc-1", target: "ent-a", family: "mention", kind: "firmante", label: "firmante", quote: null, state: "read" },
    { source: "ent-b", target: "ent-a", family: "claim", kind: "representa_a", label: "representa a", quote: "en nombre de", state: "quoted" },
    { source: "mat-1", target: "doc-1", family: "matter", kind: "matter", label: "caso", quote: null, state: "firm" },
    { source: "doc-1", target: "nadie", family: "mention", kind: "parte", label: "parte", quote: null, state: "read" },
  ],
}

beforeEach(() => {
  mockInvoke.mockReset()
  mockInvoke.mockResolvedValue(base)
})

describe("the graph of the base", () => {
  it("uses the objects and relations of the base, with no project", async () => {
    const { buildBaseGraph } = await import("../wiki-graph")

    const graph = await buildBaseGraph()

    expect(mockInvoke).toHaveBeenCalledWith("docbase_graph")
    expect(graph.nodes.map((n) => n.id).sort()).toEqual(["doc-1", "ent-a", "ent-b", "mat-1"])
    expect(graph.nodes.find((n) => n.id === "doc-1")).toMatchObject({ type: "documento", locationId: 7 })
    expect(graph.nodes.find((n) => n.id === "mat-1")).toMatchObject({ type: "asunto", confirmation: "proposed", status: "open" })
    // Two relations between the same two are one edge carrying both.
    expect(graph.edges).toHaveLength(3)
    const party = graph.edges.find((e) => e.source === "doc-1")
    expect(party?.relations?.map((r) => r.kind)).toEqual(["parte", "firmante"])
    const says = graph.edges.find((e) => e.source === "ent-b")
    expect(says?.relations?.[0]).toMatchObject({ label: "representa a", quote: "en nombre de", state: "quoted" })
    // The relevance is computed: two relations weigh more than one.
    const matter = graph.edges.find((e) => e.source === "mat-1")
    expect(party!.weight).toBeGreaterThan(matter!.weight)
    expect(graph.nodes.find((n) => n.id === "ent-a")?.linkCount).toBe(2)
  })

  it("opens the screen of each node's object", async () => {
    const { buildBaseGraph, objectOfNode } = await import("../wiki-graph")
    const graph = await buildBaseGraph()
    const byId = (id: string) => objectOfNode(graph.nodes.find((n) => n.id === id)!)

    expect(byId("doc-1")).toMatchObject({ kind: "document", objectId: "doc-1", contentId: 1, locationId: 7 })
    expect(byId("ent-a")).toMatchObject({ kind: "entity", objectId: "ent-a" })
    expect(byId("mat-1")).toMatchObject({ kind: "matter", objectId: "mat-1" })
  })

  it("reuses an unchanged graph by data version", async () => {
    const { buildBaseGraph } = await import("../wiki-graph")

    const first = await buildBaseGraph(42)
    const second = await buildBaseGraph(42)

    expect(second).toBe(first)
    expect(mockInvoke).toHaveBeenCalledTimes(1)
  })
})
