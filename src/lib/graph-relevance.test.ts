import { describe, expect, it } from "vitest"
import { buildRetrievalGraph, calculateRelevance, pairKey } from "./graph-relevance"

const nodes = [
  { id: "doc-1", type: "documento" },
  { id: "doc-2", type: "documento" },
  { id: "ent-a", type: "persona" },
  { id: "ent-b", type: "organizacion" },
  { id: "ent-c", type: "persona" },
  { id: "mat-1", type: "asunto" },
]

const edges = [
  { source: "doc-1", target: "ent-a", relations: [{}, {}] },
  { source: "doc-1", target: "ent-b", relations: [{}] },
  { source: "doc-2", target: "ent-a", relations: [{}] },
  { source: "doc-2", target: "ent-b", relations: [{}] },
  { source: "doc-2", target: "ent-c", relations: [{}] },
  { source: "mat-1", target: "doc-1", relations: [{}] },
  { source: "ent-a", target: "ent-a", relations: [{}] },
  { source: "ent-a", target: "nobody", relations: [{}] },
]

describe("relevance on the graph of the base", () => {
  const graph = buildRetrievalGraph(nodes, edges)
  const node = (id: string) => graph.nodes.get(id)!

  it("knows each node's neighbours and documents, without self or unknown ends", () => {
    expect([...node("ent-a").neighbors].sort()).toEqual(["doc-1", "doc-2"])
    expect([...node("ent-a").documents].sort()).toEqual(["doc-1", "doc-2"])
    expect([...node("doc-1").documents]).toEqual(["doc-1"])
    expect([...node("mat-1").documents]).toEqual(["doc-1"])
    expect(graph.relations.get(pairKey("ent-a", "doc-1"))).toBe(2)
    expect(graph.relations.get(pairKey("doc-1", "ent-a"))).toBe(2)
  })

  it("relates more two entities that share more documents", () => {
    const twoShared = calculateRelevance(node("ent-a"), node("ent-b"), graph)
    const oneShared = calculateRelevance(node("ent-a"), node("ent-c"), graph)
    expect(twoShared).toBeGreaterThan(oneShared)
  })

  it("relates a document more to whom it names twice than once", () => {
    const twice = calculateRelevance(node("doc-1"), node("ent-a"), graph)
    const once = calculateRelevance(node("doc-1"), node("ent-b"), graph)
    expect(twice).toBeGreaterThan(once)
  })

  it("is symmetric and zero for a node with itself", () => {
    expect(calculateRelevance(node("ent-a"), node("ent-b"), graph)).toBeCloseTo(
      calculateRelevance(node("ent-b"), node("ent-a"), graph),
    )
    expect(calculateRelevance(node("ent-a"), node("ent-a"), graph)).toBe(0)
  })
})
