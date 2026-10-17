import { describe, it, expect, beforeEach, vi } from "vitest"

const files = new Map<string, string>()
vi.mock("@/commands/fs", () => ({
  readFile: vi.fn(async (path: string) => {
    // A disk read takes a moment: two opens at once both see no file.
    await new Promise((resolve) => setTimeout(resolve, 5))
    const content = files.get(path)
    if (content === undefined) throw new Error("not found")
    return content
  }),
  writeFile: vi.fn(async (path: string, content: string) => {
    await new Promise((resolve) => setTimeout(resolve, 5))
    files.set(path, content)
  }),
}))

const store = new Map<string, unknown>()
vi.mock("@tauri-apps/plugin-store", () => ({
  load: vi.fn(async () => ({
    get: async (key: string) => {
      await new Promise((resolve) => setTimeout(resolve, 5))
      const value = store.get(key)
      return value === undefined ? undefined : structuredClone(value)
    },
    set: async (key: string, value: unknown) => {
      await new Promise((resolve) => setTimeout(resolve, 5))
      store.set(key, structuredClone(value))
    },
  })),
}))

import { ensureProjectId, getProjectPathById, loadRegistry, upsertProjectInfo } from "./project-identity"

describe("project identity", () => {
  beforeEach(() => {
    files.clear()
    store.clear()
  })

  it("gives a new folder opened twice at once a single id", async () => {
    const [a, b] = await Promise.all([ensureProjectId("C:/work/new"), ensureProjectId("C:/work/new/")])
    expect(a).toBe(b)
    expect(JSON.parse(files.get("C:/work/new/.llm-wiki/project.json")!).id).toBe(a)
    expect(await ensureProjectId("C:/work/new")).toBe(a)
  })

  it("keeps every entry when the registry is written twice at once", async () => {
    await Promise.all([
      upsertProjectInfo("id-1", "C:/work/one", "one"),
      upsertProjectInfo("id-2", "C:/work/two", "two"),
    ])
    expect(Object.keys(await loadRegistry()).sort()).toEqual(["id-1", "id-2"])
    expect(await getProjectPathById("id-1")).toBe("C:/work/one")
  })

  it("opening a new folder twice at once leaves the id it returns in the registry", async () => {
    const open = async () => {
      const id = await ensureProjectId("C:/work/fresh")
      await upsertProjectInfo(id, "C:/work/fresh", "fresh")
      return id
    }
    const [a, b] = await Promise.all([open(), open()])
    expect(a).toBe(b)
    expect(await getProjectPathById(a)).toBe("C:/work/fresh")
  })
})
