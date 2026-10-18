import { describe, it, expect, beforeEach, vi } from "vitest"
import type { LlmConfig } from "@/stores/wiki-store"
import type { FileNode } from "@/types/wiki"

// Mock LLM + Tauri FS — the lint runner also touches the activity store
// (we leave that real so we can assert status transitions).
vi.mock("./llm-client", () => ({
  streamChat: vi.fn(),
}))
vi.mock("@/commands/fs", () => ({
  readFile: vi.fn(),
  writeFile: vi.fn(),
  listDirectory: vi.fn(),
}))

import { runStructuralLint } from "./lint"
import { streamChat } from "./llm-client"
import { readFile, listDirectory } from "@/commands/fs"
import { useWikiStore } from "@/stores/wiki-store"
import { useActivityStore } from "@/stores/activity-store"

const mockStreamChat = vi.mocked(streamChat)
const mockReadFile = vi.mocked(readFile)
const mockListDirectory = vi.mocked(listDirectory)

function fakeLlmConfig(): LlmConfig {
  return {
    provider: "openai",
    apiKey: "k",
    model: "m",
    ollamaUrl: "",
    customEndpoint: "",
    maxContextSize: 128000,
  }
}

function makeFileNode(name: string, content: string): { node: FileNode; content: string } {
  return {
    node: {
      name,
      path: `/project/wiki/${name}`,
      is_dir: false,
      children: [],
    } as FileNode,
    content,
  }
}

beforeEach(() => {
  mockStreamChat.mockReset()
  mockReadFile.mockReset()
  mockListDirectory.mockReset()
  useWikiStore.getState().setOutputLanguage("auto")
  useActivityStore.setState({ items: [] })
})

describe("runStructuralLint — link suggestions", () => {
  it("stops before reading pages when cancellation is already requested", async () => {
    const pages = [makeFileNode("a.md", "# A")]
    mockListDirectory.mockResolvedValue(pages.map((entry) => entry.node))
    const controller = new AbortController()
    controller.abort()

    await expect(runStructuralLint("/project", { signal: controller.signal })).rejects.toMatchObject({
      name: "AbortError",
    })
    expect(mockReadFile).not.toHaveBeenCalled()
  })

  it("suggests the closest existing page for a broken wikilink", async () => {
    const pages = [
      makeFileNode("transformer.md", "---\ntitle: Transformer\n---\n# Transformer\nAttention model."),
      makeFileNode("attention.md", "# Attention\nSee [[transfomer]] for the architecture."),
    ]
    mockListDirectory.mockResolvedValue(pages.map((p) => p.node))
    mockReadFile.mockImplementation(async (path) => {
      const match = pages.find((p) => p.node.path === path)
      return match?.content ?? ""
    })

    const results = await runStructuralLint("/project")
    const broken = results.find((result) => result.type === "broken-link")

    expect(broken).toMatchObject({
      page: "attention.md",
      brokenTarget: "transfomer",
      suggestedTarget: "transformer.md",
    })
  })

  it("suggests related pages for orphan and no-outlinks findings", async () => {
    const pages = [
      makeFileNode("rag.md", "# RAG\nRetrieval augmented generation uses vector search."),
      makeFileNode("vector-search.md", "# Vector Search\nVector search retrieval finds related chunks."),
    ]
    mockListDirectory.mockResolvedValue(pages.map((p) => p.node))
    mockReadFile.mockImplementation(async (path) => {
      const match = pages.find((p) => p.node.path === path)
      return match?.content ?? ""
    })

    const results = await runStructuralLint("/project")
    const noOutlinks = results.find((result) => result.type === "no-outlinks" && result.page === "rag.md")
    const orphan = results.find((result) => result.type === "orphan" && result.page === "rag.md")

    expect(noOutlinks?.suggestedTarget).toBe("vector-search.md")
    expect(orphan?.suggestedSource).toBe("vector-search.md")
  })

  it("does not attach self-referential suggestions when no related page exists", async () => {
    const pages = [
      makeFileNode("alpha.md", "# Alpha\nAardvark apricot."),
      makeFileNode("beta.md", "# Beta\nZeppelin zircon."),
    ]
    mockListDirectory.mockResolvedValue(pages.map((p) => p.node))
    mockReadFile.mockImplementation(async (path) => {
      const match = pages.find((p) => p.node.path === path)
      return match?.content ?? ""
    })

    const results = await runStructuralLint("/project")
    const orphan = results.find((result) => result.type === "orphan" && result.page === "alpha.md")

    expect(orphan?.suggestedSource).toBeUndefined()
    expect(orphan?.suggestedTarget).toBeUndefined()
  })

  it("does not suggest same-folder pages without shared terms", async () => {
    const pages = [
      makeFileNode("concepts/alpha.md", "# Alpha\nAardvark apricot."),
      makeFileNode("concepts/beta.md", "# Beta\nZeppelin zircon."),
    ]
    mockListDirectory.mockResolvedValue(pages.map((p) => p.node))
    mockReadFile.mockImplementation(async (path) => {
      const match = pages.find((p) => p.node.path === path)
      return match?.content ?? ""
    })

    const results = await runStructuralLint("/project")
    const orphan = results.find((result) => result.type === "orphan" && result.page === "concepts/alpha.md")

    expect(orphan?.suggestedSource).toBeUndefined()
  })

  it("does not suggest short unrelated typo targets", async () => {
    const pages = [
      makeFileNode("bat.md", "# Bat\nFlying mammal."),
      makeFileNode("note.md", "# Note\nSee [[cat]]."),
    ]
    mockListDirectory.mockResolvedValue(pages.map((p) => p.node))
    mockReadFile.mockImplementation(async (path) => {
      const match = pages.find((p) => p.node.path === path)
      return match?.content ?? ""
    })

    const results = await runStructuralLint("/project")
    const broken = results.find((result) => result.type === "broken-link" && result.brokenTarget === "cat")

    expect(broken?.suggestedTarget).toBeUndefined()
  })
})
