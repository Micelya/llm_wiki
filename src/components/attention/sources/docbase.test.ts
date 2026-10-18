import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }))

import type { Health, Home, OriginSummary } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import { docRef, similarGroup } from "@/components/docbase/similar.fixtures"
import { docbaseNotices, similarNotice, type DocbaseFacts } from "./docbase"

const words = vocabularyFor("legal", "es")

function health(byState: Health["byState"], locations = 10): Health {
  return {
    origins: 1,
    unreachableOrigins: 0,
    locations,
    documents: locations,
    byState,
    copies: 0,
    words: 0,
    lastScanAt: null,
  }
}

function origin(patch: Partial<OriginSummary> = {}): OriginSummary {
  return {
    id: "o1",
    name: "Casos",
    provider: "local-folder",
    location: "C:/casos",
    addedAt: 0,
    lastScanAt: 0,
    reachable: true,
    files: 0,
    pending: 0,
    errors: 0,
    links: 0,
    ...patch,
  }
}

function facts(home: Partial<Home>, patch: Partial<DocbaseFacts> = {}): DocbaseFacts {
  return {
    home: { health: health({ processed: 10 }), origins: [origin()], activity: [], wiki: { pages: 0, links: 0, broken: 0 }, ...home },
    similarPending: { pairs: 0, groups: 0, sameDocumentPairs: 0 },
    groups: [],
    proposedTypes: [],
    ...patch,
  }
}

describe("what the document base raises", () => {
  it("raises nothing when everything is in order", () => {
    expect(docbaseNotices(facts({}), words)).toEqual([])
  })

  it("says what happens, why it matters and what to do, with the action in the row", () => {
    const [line] = docbaseNotices(facts({ origins: [origin({ reachable: false })] }), words)
    expect(line).toMatchObject({ id: "docbase:origins", kind: "problem", priority: "high", title: "No se puede leer «Casos»" })
    expect(line.explanation).toContain("se conserva")
    expect(line.todo).toContain("volvé a barrer")
    expect(line.actions.map((a) => a.label)).toEqual(["Volver a barrer", "Ver orígenes"])
  })

  it("groups the origins that cannot be read in one line with their cases", () => {
    const origins = [origin({ id: "a", name: "A", reachable: false }), origin({ id: "b", name: "B", reachable: false })]
    const [line] = docbaseNotices(facts({ origins }), words)
    expect(line.title).toBe("2 carpetas incorporadas no se pueden leer")
    expect(line.cases?.map((c) => c.title)).toEqual(["A", "B"])
  })

  it("does not count an origin never swept as unreachable", () => {
    expect(docbaseNotices(facts({ origins: [origin({ reachable: null })] }), words)).toEqual([])
  })

  it("classes each state of the documents as the design says", () => {
    const lines = docbaseNotices(
      facts({ health: health({ unreadable: 1, error: 2, "too-large": 3, link: 74, "needs-recognition": 5, unread: 7 }, 100) }),
      words,
    )
    const by = Object.fromEntries(lines.map((line) => [line.topic, [line.kind, line.priority]]))
    expect(by).toEqual({
      unreadable: ["problem", "high"],
      error: ["problem", "high"],
      "too-large": ["problem", "low"],
      link: ["problem", "low"],
      // Text in an image is recognized when ingested: not a problem.
      "needs-recognition": ["ongoing", "normal"],
      reading: ["ongoing", "normal"],
    })
    const unreadable = lines.find((line) => line.topic === "unreadable")!
    expect(unreadable.title).toBe("1 archivo no se pudo leer")
    expect(unreadable.loadCases).toBeTypeOf("function")
    expect(unreadable.actions[0]).toMatchObject({ kind: "go", target: { view: "documents", filter: { state: "unreadable" } } })
    expect(lines.find((line) => line.topic === "link")!.title).toBe("74 documentos de un disco en la nube quedaron afuera")
  })

  it("raises the types the model proposed as one decision per type", () => {
    const [line] = docbaseNotices(
      facts({}, {
        proposedTypes: [
          { id: "cronologia", name: "Cronología", description: "Una lista de hechos con fecha.", documents: 3 },
          { id: "intimacion", name: null, description: null, documents: 1 },
        ],
      }),
      words,
    )
    expect(line).toMatchObject({ kind: "decision", topic: "document-types", count: 2 })
    expect(line.title).toBe("2 tipos de documento nuevos propuestos por el modelo")
    expect(line.explanation).toContain("4 documentos")
    expect(line.cases?.map((c) => c.title)).toEqual(["Cronología", "Intimacion"])
  })
})

describe("similar documents in the attention view", () => {
  const principal = docRef(10, "Acuerdo modelo.docx")

  it("does not escalate what does not change the wiki: same template, contained in, plain similar", () => {
    const groups = [similarGroup(principal, [docRef(11, "a.docx"), docRef(12, "b.docx")], 2)]
    expect(similarNotice({ pairs: 5, groups: 1, sameDocumentPairs: 0 }, groups)).toBeNull()
  })

  it("counts one decision per family and one per pair outside families", () => {
    const groups = [
      similarGroup(docRef(20, "Facturas.pdf"), [docRef(21, "x.pdf")], 1),
      similarGroup(principal, [docRef(11, "a.docx"), docRef(12, "b.docx"), docRef(13, "c.docx")], 2, [], "signed-copy"),
    ]
    const line = similarNotice({ pairs: 6, groups: 2, sameDocumentPairs: 4 }, groups)!
    expect(line.title).toBe("4 pares de documentos podrían ser el mismo documento")
    // One family with two pairs, and two pairs outside families.
    expect(line.count).toBe(3)
    expect(line.cases?.map((c) => c.title)).toEqual(["«Acuerdo modelo.docx»", "2 pares fuera de las familias"])
    expect(line.explanation).toContain("al wiki va uno solo")
    // Each leads to the similar documents screen, on its family.
    expect(line.actions[0]).toMatchObject({ kind: "go", target: { view: "similar", principal: 10 } })
    expect(line.cases?.[0].actions?.[0]).toMatchObject({ target: { view: "similar", principal: 10 } })
  })

  it("goes away once nothing that makes one document waits", () => {
    const before = docbaseNotices(facts({}, { similarPending: { pairs: 1, groups: 0, sameDocumentPairs: 1 } }), words)
    const after = docbaseNotices(facts({}, { similarPending: { pairs: 1, groups: 0, sameDocumentPairs: 0 } }), words)
    expect(before.map((n) => n.id)).toEqual(["docbase:similar"])
    expect(after).toEqual([])
  })
})
