import { beforeEach, describe, expect, it, vi } from "vitest"

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  files: new Map<string, string>(),
  saveIngestCache: vi.fn(async () => {}),
  copyIngestCacheEntry: vi.fn(async () => true),
}))

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }))
vi.mock("@/commands/fs", () => ({
  fileExists: vi.fn(async (path: string) => mocks.files.has(path)),
  readFile: vi.fn(async (path: string) => {
    const content = mocks.files.get(path)
    if (content === undefined) throw new Error(`missing ${path}`)
    return content
  }),
  writeFile: vi.fn(async (path: string, content: string) => {
    mocks.files.set(path, content)
  }),
}))
vi.mock("@/lib/ingest-cache", () => ({
  saveIngestCache: mocks.saveIngestCache,
  copyIngestCacheEntry: mocks.copyIngestCacheEntry,
}))

import { parseSources } from "@/lib/sources-merge"
import { useActivityStore } from "@/stores/activity-store"
import {
  adoptCataloguedContent,
  describeAdoption,
  leaveOutIdenticalFiles,
  recordCataloguedIngest,
  releaseCataloguedContent,
} from "./source-catalog"

const PP = "/p"
const commit = <T,>(operation: () => Promise<T>) => operation()

function request(identity: string) {
  return {
    projectPath: PP,
    sourcePath: `${PP}/raw/sources/${identity}`,
    identity,
    text: "the text",
    commit,
  }
}

function page(sources: string[]): string {
  return `---\ntitle: Acuerdo\nsources: [${sources.map((s) => `"${s}"`).join(", ")}]\n---\n\nBody\n`
}

describe("source catalog at ingest time", () => {
  beforeEach(() => {
    mocks.invoke.mockReset()
    mocks.saveIngestCache.mockClear()
    mocks.copyIngestCacheEntry.mockClear()
    useActivityStore.setState({ items: [] })
    mocks.files.clear()
    releaseCataloguedContent(PP, `${PP}/raw/sources/a.docx`)
    releaseCataloguedContent(PP, `${PP}/raw/sources/a.pdf`)
  })

  it("ingests normally a document seen for the first time", async () => {
    mocks.invoke.mockResolvedValue({ id: "c1", recognized: false, wordCount: 90, locations: ["a.docx"] })
    expect(await adoptCataloguedContent(request("a.docx"))).toBeNull()
    expect(mocks.saveIngestCache).not.toHaveBeenCalled()
  })

  it("ingests normally a text too short to be catalogued", async () => {
    mocks.invoke.mockResolvedValue(null)
    expect(await adoptCataloguedContent(request("portada.pdf"))).toBeNull()
  })

  it("attaches a second file of an ingested document to its pages instead of ingesting it", async () => {
    mocks.files.set(`${PP}/wiki/sources/a.md`, page(["a.docx"]))
    mocks.files.set(`${PP}/wiki/entities/sovra.md`, page(["otro.pdf", "a.docx"]))
    mocks.files.set(`${PP}/wiki/concepts/ajeno.md`, page(["otro.pdf"]))
    mocks.invoke.mockResolvedValue({
      id: "c1",
      recognized: false,
      wordCount: 90,
      locations: ["a.docx", "a.pdf"],
      ingested: {
        identity: "a.docx",
        files: ["wiki/sources/a.md", "wiki/entities/sovra.md", "wiki/concepts/ajeno.md"],
      },
    })

    const adopted = await adoptCataloguedContent(request("a.pdf"))

    expect(adopted).toEqual({
      files: ["wiki/sources/a.md", "wiki/entities/sovra.md", "wiki/concepts/ajeno.md"],
      original: "a.docx",
      recognized: false,
    })
    expect(parseSources(mocks.files.get(`${PP}/wiki/sources/a.md`)!)).toEqual(["a.docx", "a.pdf"])
    expect(parseSources(mocks.files.get(`${PP}/wiki/entities/sovra.md`)!)).toEqual(["otro.pdf", "a.docx", "a.pdf"])
    // A page that no longer lists the original does not belong to this document.
    expect(parseSources(mocks.files.get(`${PP}/wiki/concepts/ajeno.md`)!)).toEqual(["otro.pdf"])
    expect(mocks.saveIngestCache).toHaveBeenCalledWith(PP, "a.pdf", "the text", adopted!.files)
  })

  it("does not list the same file twice when it is checked again", async () => {
    mocks.files.set(`${PP}/wiki/sources/a.md`, page(["a.docx", "A.PDF"]))
    mocks.invoke.mockResolvedValue({
      id: "c1",
      recognized: false,
      wordCount: 90,
      locations: ["a.docx", "a.pdf"],
      ingested: { identity: "a.docx", files: ["wiki/sources/a.md"] },
    })
    await adoptCataloguedContent(request("a.pdf"))
    expect(parseSources(mocks.files.get(`${PP}/wiki/sources/a.md`)!)).toEqual(["a.docx", "A.PDF"])
  })

  it("ingests normally when the pages of the earlier ingest are gone", async () => {
    mocks.invoke.mockResolvedValue({
      id: "c1",
      recognized: false,
      wordCount: 90,
      locations: ["a.docx", "a.pdf"],
      ingested: { identity: "a.docx", files: ["wiki/sources/a.md"] },
    })
    expect(await adoptCataloguedContent(request("a.pdf"))).toBeNull()
  })

  it("re-ingests the original itself", async () => {
    mocks.files.set(`${PP}/wiki/sources/a.md`, page(["a.docx"]))
    mocks.invoke.mockResolvedValue({
      id: "c1",
      recognized: false,
      wordCount: 90,
      locations: ["a.docx"],
      ingested: { identity: "a.docx", files: ["wiki/sources/a.md"] },
    })
    expect(await adoptCataloguedContent(request("a.docx"))).toBeNull()
  })

  it("makes a second copy wait for the ingest of the first and then share its pages", async () => {
    const pending = { id: "c1", recognized: false, wordCount: 90, locations: ["a.docx", "a.pdf"] }
    mocks.invoke.mockResolvedValue(pending)
    expect(await adoptCataloguedContent(request("a.docx"))).toBeNull()

    let settled = false
    const second = adoptCataloguedContent(request("a.pdf")).then((result) => {
      settled = true
      return result
    })
    await new Promise((resolve) => setTimeout(resolve, 10))
    expect(settled).toBe(false)

    mocks.files.set(`${PP}/wiki/sources/a.md`, page(["a.docx"]))
    mocks.invoke.mockResolvedValue({ ...pending, ingested: { identity: "a.docx", files: ["wiki/sources/a.md"] } })
    releaseCataloguedContent(PP, `${PP}/raw/sources/a.docx`)

    expect((await second)?.original).toBe("a.docx")
  })

  it("lets the second copy ingest when the first one failed", async () => {
    const pending = { id: "c1", recognized: false, wordCount: 90, locations: ["a.docx", "a.pdf"] }
    mocks.invoke.mockResolvedValue(pending)
    await adoptCataloguedContent(request("a.docx"))
    const second = adoptCataloguedContent(request("a.pdf"))
    releaseCataloguedContent(PP, `${PP}/raw/sources/a.docx`)
    expect(await second).toBeNull()
  })

  it("never blocks ingestion when the catalog fails", async () => {
    mocks.invoke.mockRejectedValue(new Error("catalog unreadable"))
    expect(await adoptCataloguedContent(request("a.pdf"))).toBeNull()
    await expect(recordCataloguedIngest(PP, "a.pdf", ["wiki/sources/a.md"])).resolves.toBeUndefined()
  })

  it("says when the match rests on recognized text", () => {
    expect(describeAdoption({ files: ["x"], original: "a.docx", recognized: false }))
      .toBe("Skipped (same document as a.docx) — 1 files shared")
    expect(describeAdoption({ files: ["x"], original: "a.docx", recognized: true }))
      .toContain("matched on recognized text")
  })

  describe("identical files, before queueing", () => {
    const files = [
      { sourcePath: `${PP}/raw/sources/Docs/carta.jpeg`, folderContext: "Docs" },
      { sourcePath: `${PP}/raw/sources/Otra/carta (copia).jpeg`, folderContext: "Otra" },
    ]

    it("queues every file when none is a known file", async () => {
      mocks.invoke.mockResolvedValue(null)
      expect(await leaveOutIdenticalFiles(PP, files)).toEqual(files)
      expect(mocks.invoke).toHaveBeenCalledWith("catalog_match_file", {
        projectPath: PP,
        identity: "Otra/carta (copia).jpeg",
      })
    })

    it("leaves out a copy of an ingested file and attaches it to the pages", async () => {
      mocks.files.set(`${PP}/wiki/sources/carta.md`, page(["Docs/carta.jpeg"]))
      mocks.invoke.mockImplementation(async (_command: string, args: { identity: string }) =>
        args.identity === "Otra/carta (copia).jpeg"
          ? {
              id: "c1",
              recognized: true,
              wordCount: 500,
              locations: ["Docs/carta.jpeg", "Otra/carta (copia).jpeg"],
              ingested: { identity: "Docs/carta.jpeg", files: ["wiki/sources/carta.md"] },
            }
          : null,
      )

      const pending = await leaveOutIdenticalFiles(PP, files)

      expect(pending).toEqual([files[0]])
      expect(parseSources(mocks.files.get(`${PP}/wiki/sources/carta.md`)!)).toEqual([
        "Docs/carta.jpeg",
        "Otra/carta (copia).jpeg",
      ])
      expect(mocks.copyIngestCacheEntry).toHaveBeenCalledWith(PP, "Docs/carta.jpeg", "Otra/carta (copia).jpeg")
      const [activity] = useActivityStore.getState().items
      expect(activity.status).toBe("done")
      expect(activity.detail).toContain("same file as Docs/carta.jpeg")
    })

    it("queues the original itself, and a copy whose document has no pages yet", async () => {
      const known = {
        id: "c1",
        recognized: false,
        wordCount: 90,
        locations: ["Docs/carta.jpeg", "Otra/carta (copia).jpeg"],
      }
      mocks.invoke.mockResolvedValue({ ...known, ingested: { identity: "Docs/carta.jpeg", files: ["wiki/sources/carta.md"] } })
      // The pages are missing, so nothing can be shared.
      expect(await leaveOutIdenticalFiles(PP, files)).toEqual(files)

      mocks.files.set(`${PP}/wiki/sources/carta.md`, page(["Docs/carta.jpeg"]))
      expect(await leaveOutIdenticalFiles(PP, [files[0]])).toEqual([files[0]])

      mocks.invoke.mockResolvedValue(known)
      expect(await leaveOutIdenticalFiles(PP, [files[1]])).toEqual([files[1]])
    })

    it("queues the file when the check fails", async () => {
      mocks.invoke.mockRejectedValue(new Error("catalog unreadable"))
      expect(await leaveOutIdenticalFiles(PP, files)).toEqual(files)
    })
  })
})
