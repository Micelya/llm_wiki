import { beforeEach, describe, expect, it, vi } from "vitest"
import type { GeneralProject, WikiSource } from "@/commands/docbase"
import type { WikiProject } from "@/types/wiki"

// Invented documents and folders (R-24).
const mocks = vi.hoisted(() => ({
  general: null as GeneralProject | null,
  sources: [] as WikiSource[],
  ingested: [] as string[],
  queue: [] as { projectId: string; sourcePath: string; status: string }[],
  excluded: [] as string[],
  excludedSources: vi.fn(),
  discardTasksForSources: vi.fn(),
  enqueueSourceIngest: vi.fn(),
}))

vi.mock("@/commands/docbase", () => ({
  getGeneralProject: () => Promise.resolve(mocks.general),
  listWikiSources: () => Promise.resolve(mocks.sources),
  excludedSources: mocks.excludedSources,
  setWikiActive: vi.fn(),
}))
vi.mock("@/lib/ingest-cache", () => ({
  listIngestedSourceIdentities: () => Promise.resolve(mocks.ingested),
}))
vi.mock("@/lib/ingest-queue", () => ({
  getQueue: () => mocks.queue,
  discardTasksForSources: mocks.discardTasksForSources,
  pauseProcessing: vi.fn(),
  resumeProcessing: vi.fn(),
}))
vi.mock("@/lib/project-store", () => ({
  loadSourceWatchConfig: vi.fn(),
  saveSourceWatchConfig: vi.fn(),
}))
vi.mock("@/lib/source-lifecycle", () => ({
  enqueueSourceIngest: mocks.enqueueSourceIngest,
  isIngestableSourcePath: () => true,
}))
vi.mock("@/stores/wiki-store", () => ({
  useWikiStore: { getState: () => ({ llmConfig: {} }) },
}))

import { followSimilarDecision, sourcesOfDocuments } from "./docbase-wiki-bridge"

const project = { id: "p1", name: "General", path: "C:/Espacio/default" } as WikiProject

function source(identity: string, contentId: number | null): WikiSource {
  return { originId: "o1", identity, sourcePath: `raw/sources/${identity}`, ext: "md", contentId }
}

function general(wikiActive: boolean, path = "C:/Espacio/default"): GeneralProject {
  return { id: "g", name: "General", path, members: "open", wikiActive, origins: [] }
}

beforeEach(() => {
  mocks.general = general(true)
  mocks.sources = []
  mocks.ingested = []
  mocks.queue = []
  mocks.excluded = []
  mocks.excludedSources.mockReset().mockImplementation(() => Promise.resolve(mocks.excluded))
  mocks.discardTasksForSources.mockReset().mockResolvedValue(0)
  mocks.enqueueSourceIngest.mockReset().mockImplementation((_project, paths: string[]) => Promise.resolve(paths))
})

describe("sources of some documents", () => {
  it("keeps the sources of those documents, every copy, and none not read yet", () => {
    const sources = [source("O/a.md", 1), source("O/copia de a.md", 1), source("O/b.md", 2), source("O/nuevo.md", null)]
    expect(sourcesOfDocuments(sources, [1]).map((s) => s.identity)).toEqual(["O/a.md", "O/copia de a.md"])
    expect(sourcesOfDocuments(sources, [])).toEqual([])
  })
})

describe("the wiki after a decision about similar documents", () => {
  it("queues the chosen source when it was never ingested and the wiki is active", async () => {
    // The base already leaves the one not chosen out of its sources.
    mocks.sources = [source("O/final.md", 2), source("O/otro.md", 9)]
    expect(await followSimilarDecision(project, [1, 2])).toBe(1)
    expect(mocks.enqueueSourceIngest).toHaveBeenCalledWith(project, ["raw/sources/O/final.md"], {})
  })

  it("queues nothing that is already ingested", async () => {
    mocks.sources = [source("O/final.md", 2)]
    mocks.ingested = ["O/final.md"]
    expect(await followSimilarDecision(project, [1, 2])).toBe(0)
    expect(mocks.enqueueSourceIngest).not.toHaveBeenCalled()
  })

  it("after an undo offers both again", async () => {
    mocks.sources = [source("O/borrador.md", 1), source("O/final.md", 2)]
    mocks.ingested = ["O/final.md"]
    expect(await followSimilarDecision(project, [1, 2])).toBe(1)
    expect(mocks.enqueueSourceIngest).toHaveBeenCalledWith(project, ["raw/sources/O/borrador.md"], {})
  })

  it("drops from the queue what waits and no longer feeds the wiki, but not what is in progress", async () => {
    mocks.queue = [
      { projectId: "p1", sourcePath: "raw/sources/O/borrador.md", status: "pending" },
      { projectId: "p1", sourcePath: "raw/sources/O/en curso.md", status: "processing" },
      { projectId: "otro", sourcePath: "raw/sources/O/ajeno.md", status: "pending" },
    ]
    mocks.excluded = ["raw/sources/O/borrador.md"]
    await followSimilarDecision(project, [1, 2])
    expect(mocks.excludedSources).toHaveBeenCalledWith("C:/Espacio/default", ["raw/sources/O/borrador.md"])
    expect(mocks.discardTasksForSources).toHaveBeenCalledWith(["raw/sources/O/borrador.md"])
  })

  it("with the wiki paused queues nothing, but still drops what no longer feeds it", async () => {
    mocks.general = general(false)
    mocks.sources = [source("O/final.md", 2)]
    mocks.queue = [{ projectId: "p1", sourcePath: "raw/sources/O/borrador.md", status: "pending" }]
    mocks.excluded = ["raw/sources/O/borrador.md"]
    expect(await followSimilarDecision(project, [1, 2])).toBe(0)
    expect(mocks.discardTasksForSources).toHaveBeenCalled()
    expect(mocks.enqueueSourceIngest).not.toHaveBeenCalled()
  })

  it("does nothing when the open project is not the general one, or none is open", async () => {
    mocks.general = general(true, "C:/Espacio/otro")
    mocks.sources = [source("O/final.md", 2)]
    expect(await followSimilarDecision(project, [1, 2])).toBe(0)
    expect(await followSimilarDecision(null, [1, 2])).toBe(0)
    expect(mocks.enqueueSourceIngest).not.toHaveBeenCalled()
  })
})
