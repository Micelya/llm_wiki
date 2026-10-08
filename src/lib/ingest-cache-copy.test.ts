import { beforeEach, describe, expect, it, vi } from "vitest"

vi.mock("@/commands/fs", () => ({
  readFile: vi.fn(),
  writeFile: vi.fn(),
  fileExists: vi.fn(),
}))

import { readFile, writeFile, fileExists } from "@/commands/fs"
import { checkIngestCache, copyIngestCacheEntry, saveIngestCache } from "./ingest-cache"

describe("copyIngestCacheEntry", () => {
  let persisted = ""

  beforeEach(() => {
    persisted = JSON.stringify({ entries: {} })
    vi.mocked(readFile).mockImplementation(async () => persisted)
    vi.mocked(writeFile).mockImplementation(async (_path: string, content: string) => {
      persisted = content
    })
    vi.mocked(fileExists).mockResolvedValue(true)
  })

  it("makes the same file under another path count as already ingested", async () => {
    await saveIngestCache("/project", "Docs/carta.jpeg", "texto", ["wiki/sources/carta.md"])

    expect(await copyIngestCacheEntry("/project", "Docs/carta.jpeg", "Otra/carta.jpeg")).toBe(true)

    expect(await checkIngestCache("/project", "Otra/carta.jpeg", "texto")).toEqual(["wiki/sources/carta.md"])
    expect(await checkIngestCache("/project", "Docs/carta.jpeg", "texto")).toEqual(["wiki/sources/carta.md"])
  })

  it("does nothing when the original has no entry", async () => {
    expect(await copyIngestCacheEntry("/project", "Docs/carta.jpeg", "Otra/carta.jpeg")).toBe(false)
    expect(await checkIngestCache("/project", "Otra/carta.jpeg", "texto")).toBeNull()
  })
})
