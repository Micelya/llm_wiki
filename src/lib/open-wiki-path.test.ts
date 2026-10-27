import { beforeEach, describe, expect, it, vi } from "vitest"
import type { PageObject } from "@/commands/docbase"

const objectOfPage = vi.fn<(reference: string) => Promise<PageObject | null>>()
vi.mock("@/commands/docbase", () => ({ objectOfPage: (reference: string) => objectOfPage(reference) }))

import { openWikiPath } from "./open-wiki-path"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"

function found(patch: Partial<PageObject>): PageObject {
  return { objectId: "ent-1", kind: "entity", path: "wiki/entities/ana-prueba-1.md", contentId: null, locationId: null, ...patch }
}

describe("openWikiPath: every page is the screen of what it names (D-407)", () => {
  beforeEach(() => {
    objectOfPage.mockReset()
    useWikiStore.getState().setActiveView("chat")
  })

  it("a person or organization opens its page", async () => {
    objectOfPage.mockResolvedValue(found({}))
    expect(await openWikiPath("ana-prueba-1")).toBe(true)
    expect(objectOfPage).toHaveBeenCalledWith("ana-prueba-1")
    expect(useWikiStore.getState().activeView).toBe("entities")
    expect(useKnowledgeStore.getState().entityId).toBe("ent-1")
  })

  it("a matter opens its screen", async () => {
    objectOfPage.mockResolvedValue(found({ objectId: "mat-7", kind: "matter", path: "wiki/entities/caso-7.md" }))
    expect(await openWikiPath("wiki/entities/caso-7.md")).toBe(true)
    expect(useWikiStore.getState().activeView).toBe("matters")
    expect(useKnowledgeStore.getState().matterId).toBe("mat-7")
  })

  it("a document opens its detail, at one of its places", async () => {
    objectOfPage.mockResolvedValue(found({ objectId: "doc-3", kind: "document", path: "wiki/sources/contrato-3.md", contentId: 3, locationId: 30 }))
    expect(await openWikiPath("contrato-3")).toBe(true)
    expect(useWikiStore.getState().activeView).toBe("documents")
    expect(useDocbaseStore.getState().openLocationId).toBe(30)
  })

  it("a document a joined file includes opens inside that file, as itself (D-560)", async () => {
    objectOfPage.mockResolvedValue(
      found({ objectId: "doc-8", kind: "document", path: "wiki/sources/anexo-8.md", contentId: 8, locationId: 30, included: true }),
    )
    expect(await openWikiPath("anexo-8")).toBe(true)
    expect(useDocbaseStore.getState()).toMatchObject({ openLocationId: 30, openVersionId: 8 })
    // A document with a file of its own opens just that file.
    objectOfPage.mockResolvedValue(found({ objectId: "doc-3", kind: "document", path: "wiki/sources/contrato-3.md", contentId: 3, locationId: 31 }))
    expect(await openWikiPath("contrato-3")).toBe(true)
    expect(useDocbaseStore.getState()).toMatchObject({ openLocationId: 31, openVersionId: null })
  })

  it("the overview opens Inicio", async () => {
    objectOfPage.mockResolvedValue(found({ objectId: "overview", kind: "overview", path: "wiki/overview.md" }))
    expect(await openWikiPath("overview")).toBe(true)
    expect(useWikiStore.getState().activeView).toBe("home")
  })

  it("a page the base does not know opens nothing, not a file", async () => {
    objectOfPage.mockResolvedValue(null)
    expect(await openWikiPath("wiki/concepts/otra.md")).toBe(false)
    objectOfPage.mockRejectedValue("sin base")
    expect(await openWikiPath("otra")).toBe(false)
    expect(useWikiStore.getState().activeView).toBe("chat")
  })
})
