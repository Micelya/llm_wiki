/**
 * The graph of the wiki of the archive comes from the base (D-351): nodes
 * are its pages and edges the relations the base keeps, with their type,
 * quote and state. The links written in the files are not read.
 */
import { describe, it, expect, vi, beforeEach } from "vitest"

const mockInvoke = vi.fn()
const mockListDirectory = vi.fn()
const mockReadFile = vi.fn()

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}))
vi.mock("@/commands/fs", () => ({
  listDirectory: (...args: unknown[]) => mockListDirectory(...args),
  readFile: (...args: unknown[]) => mockReadFile(...args),
}))

beforeEach(() => {
  mockInvoke.mockReset()
  mockListDirectory.mockReset()
  mockReadFile.mockReset()
})

describe("buildWikiGraph from the base", () => {
  it("uses the pages and relations of the base and reads no file", async () => {
    mockInvoke.mockResolvedValue({
      nodes: [
        { id: "contrato-1", label: "Contrato de prueba", pageType: "source", kind: "document", path: "wiki/sources/contrato-1.md" },
        { id: "ana-prueba", label: "Ana Prueba", pageType: "entity", kind: "persona", path: "wiki/entities/ana-prueba.md" },
        { id: "beto-ensayo", label: "Beto Ensayo", pageType: "entity", kind: "persona", path: "wiki/entities/beto-ensayo.md" },
      ],
      edges: [
        { source: "contrato-1", target: "ana-prueba", family: "mention", kind: "parte", label: "parte", quote: "Ana", state: "read" },
        { source: "contrato-1", target: "ana-prueba", family: "mention", kind: "firmante", label: "firmante", quote: null, state: "read" },
        { source: "beto-ensayo", target: "ana-prueba", family: "claim", kind: "representa_a", label: "representa a", quote: "en nombre de", state: "quoted" },
        { source: "contrato-1", target: "nadie", family: "mention", kind: "parte", label: "parte", quote: null, state: "read" },
      ],
    })
    const { buildWikiGraph } = await import("../wiki-graph")

    const graph = await buildWikiGraph("/proyecto")

    expect(mockInvoke).toHaveBeenCalledWith("docbase_graph", { projectPath: "/proyecto" })
    expect(mockListDirectory).not.toHaveBeenCalled()
    expect(graph.nodes.map((n) => n.id).sort()).toEqual(["ana-prueba", "beto-ensayo", "contrato-1"])
    expect(graph.nodes.find((n) => n.id === "contrato-1")?.path).toBe("/proyecto/wiki/sources/contrato-1.md")
    expect(graph.nodes.find((n) => n.id === "contrato-1")?.type).toBe("source")
    // Two relations between the same pages are one edge carrying both.
    expect(graph.edges).toHaveLength(2)
    const party = graph.edges.find((e) => e.source === "contrato-1")
    expect(party?.relations?.map((r) => r.kind)).toEqual(["parte", "firmante"])
    expect(party?.weight).toBe(2)
    const says = graph.edges.find((e) => e.source === "beto-ensayo")
    expect(says?.relations?.[0]).toMatchObject({ label: "representa a", quote: "en nombre de", state: "quoted" })
  })

  it("falls back to the links of the pages for any other project", async () => {
    mockInvoke.mockResolvedValue(null)
    mockListDirectory.mockResolvedValue([{ name: "a.md", path: "/otro/wiki/a.md", is_dir: false }])
    mockReadFile.mockResolvedValue("---\ntitle: A\n---\n# A\n")
    const { buildWikiGraph } = await import("../wiki-graph")

    const graph = await buildWikiGraph("/otro")

    expect(mockListDirectory).toHaveBeenCalled()
    expect(graph.nodes.map((n) => n.id)).toEqual(["a"])
  })
})
