/**
 * The merge of duplicates of Maintenance, over pages projected by the
 * knowledge layer: joined (or kept apart) on the record, never by
 * rewriting files (D-220).
 */
import { describe, it, expect, vi, beforeEach } from "vitest"

const { mockListDirectory, mockReadFile, mockWriteFile, mockDeleteFile, mockDecideMany } = vi.hoisted(() => ({
  mockListDirectory: vi.fn(),
  mockReadFile: vi.fn(),
  mockWriteFile: vi.fn(),
  mockDeleteFile: vi.fn(),
  mockDecideMany: vi.fn(),
}))
vi.mock("@/commands/fs", () => ({
  listDirectory: mockListDirectory,
  readFile: mockReadFile,
  writeFile: mockWriteFile,
  deleteFile: mockDeleteFile,
}))
vi.mock("@/commands/docbase", () => ({ decideIdentityMany: mockDecideMany }))
vi.mock("./llm-client", () => ({ streamChat: vi.fn() }))

import { executeMerge, loadAllEntitySummaries, projectedEntityId, separateOnTheRecord } from "./dedup-runner"
import type { LlmConfig } from "@/stores/wiki-store"

const projected = (id: string, title: string, extra = "") =>
  `---\ntype: entity\ntitle: "${title}"\nentity_kind: persona\nentity_id: ${id}\n${extra}generated_by: micelya-conocimiento\n---\n\n# ${title}\n`
const written = (title: string) => `---\ntype: entity\ntitle: ${title}\n---\n\n# ${title}\n`

const files: Record<string, string> = {
  "/p/wiki/entities/persona-uno-aaaa1111.md": projected("ent-aaaa1111", "Persona Uno"),
  "/p/wiki/entities/p-uno-bbbb2222.md": projected("ent-bbbb2222", "P. Uno"),
  "/p/wiki/entities/asunto-x-cccc3333.md": projected("mat-cccc3333", "Asunto X").replace("entity_kind: persona", "entity_kind: asunto"),
  "/p/wiki/entities/vieja-dddd4444.md": projected("ent-dddd4444", "Vieja", "state: kept\n"),
  "/p/wiki/entities/escrita.md": written("Escrita"),
}

const tree = [
  {
    name: "wiki",
    path: "/p/wiki",
    is_dir: true,
    children: [
      {
        name: "entities",
        path: "/p/wiki/entities",
        is_dir: true,
        children: Object.keys(files).map((path) => ({ name: path.split("/").pop(), path, is_dir: false })),
      },
    ],
  },
]

const cfg = { provider: "ollama", apiKey: "", model: "m" } as unknown as LlmConfig

beforeEach(() => {
  vi.clearAllMocks()
  mockListDirectory.mockResolvedValue(tree)
  mockReadFile.mockImplementation(async (path: string) => files[path])
  mockDecideMany.mockResolvedValue([1])
})

describe("pages of the record in the merge of duplicates", () => {
  it("knows the entity of a projected page, but not of a matter or a kept page", () => {
    expect(projectedEntityId(files["/p/wiki/entities/persona-uno-aaaa1111.md"])).toBe("ent-aaaa1111")
    expect(projectedEntityId(files["/p/wiki/entities/asunto-x-cccc3333.md"])).toBeNull()
    expect(projectedEntityId(files["/p/wiki/entities/vieja-dddd4444.md"])).toBeNull()
    expect(projectedEntityId(files["/p/wiki/entities/escrita.md"])).toBeNull()
  })

  it("offers the pages of entities of the record, not those of matters or kept ones", async () => {
    const slugs = (await loadAllEntitySummaries("/p")).map((s) => s.slug).sort()
    expect(slugs).toEqual(["escrita", "p-uno-bbbb2222", "persona-uno-aaaa1111"])
  })

  it("joins two pages of the record on the record and writes no file", async () => {
    const result = await executeMerge(
      "/p",
      { slugs: ["persona-uno-aaaa1111", "p-uno-bbbb2222"], reason: "", confidence: "high" },
      "persona-uno-aaaa1111",
      cfg,
    )
    expect(mockDecideMany).toHaveBeenCalledWith([{ kind: "join", a: "entity:ent-aaaa1111", b: "entity:ent-bbbb2222" }], expect.any(String))
    expect(mockWriteFile).not.toHaveBeenCalled()
    expect(mockDeleteFile).not.toHaveBeenCalled()
    expect(result.pagesToDelete).toEqual([])
  })

  it("does not merge a page of the record with one the wiki wrote", async () => {
    await expect(
      executeMerge("/p", { slugs: ["persona-uno-aaaa1111", "escrita"], reason: "", confidence: "high" }, "escrita", cfg),
    ).rejects.toThrow(/registro/)
    expect(mockDecideMany).not.toHaveBeenCalled()
    expect(mockWriteFile).not.toHaveBeenCalled()
  })

  it("keeps apart on the record the pages a person says are not duplicates", async () => {
    expect(await separateOnTheRecord("/p", ["persona-uno-aaaa1111", "p-uno-bbbb2222", "escrita"])).toBe(1)
    expect(mockDecideMany).toHaveBeenCalledWith(
      [{ kind: "separate", a: "entity:ent-aaaa1111", b: "entity:ent-bbbb2222" }],
      expect.any(String),
    )
    mockDecideMany.mockClear()
    expect(await separateOnTheRecord("/p", ["escrita"])).toBe(0)
    expect(mockDecideMany).not.toHaveBeenCalled()
  })
})
