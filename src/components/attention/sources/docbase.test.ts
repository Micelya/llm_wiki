import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }))

import type { Health, Home, OriginSummary } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import { docRef, similarGroup } from "@/components/docbase/similar.fixtures"
import { clipsNotice, coreLeftNotice, docbaseNotices, similarNotice, type DocbaseFacts } from "./docbase"

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
    home: { health: health({ processed: 10 }), origins: [origin()], activity: [], ...home },
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
    // Text in an image waits for its recognition and files are still being
    // read: not a problem; they are in the stages of «En curso» (D-424).
    expect(by).toEqual({
      unreadable: ["problem", "high"],
      error: ["problem", "high"],
      "too-large": ["problem", "low"],
      link: ["problem", "low"],
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

  it("escalates two of the same template where no field changes: possibly the same document", () => {
    const group = { ...similarGroup(principal, [docRef(11, "a.docx"), docRef(12, "b.docx")], 2), possiblySame: 1 }
    const line = similarNotice({ pairs: 2, groups: 1, sameDocumentPairs: 0 }, [group])!
    expect(line.title).toBe("1 par de documentos podría ser el mismo documento")
    expect(line.count).toBe(1)
    expect(line.cases?.[0].detail).toContain("1 par de la misma plantilla no cambia en ningún dato")
    expect(line.cases?.[0].actions?.[0]).toMatchObject({ target: { view: "similar", principal: 10 } })
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
    expect(line.explanation).toContain("cuenta uno solo")
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

describe("the new web clips", () => {
  const clip = (locationId: number, name: string) => ({
    locationId,
    name,
    folder: "Recortes web",
    originName: "Carpeta de trabajo",
    firstSeenAt: Date.now(),
  })

  it("says nothing when no clip came in since they were seen", () => {
    expect(clipsNotice({ total: 0, files: [] })).toBeNull()
    expect(clipsNotice(null)).toBeNull()
    expect(docbaseNotices(facts({}, { newClips: { total: 0, files: [] } }), words)).toEqual([])
  })

  it("lists each new clip, opens it as a document and goes away once seen", () => {
    const line = clipsNotice({ total: 3, files: [clip(7, "2026-03-04 Una nota.md"), clip(8, "2026-03-04 Otra nota.md")] })!
    expect(line).toMatchObject({ id: "docbase:clips", kind: "decision", count: 3, title: "3 recortes web nuevos" })
    expect(line.actions.map((a) => a.label)).toEqual(["Ya los vi", "Abrir la carpeta"])
    expect(line.resolvedWhen).toContain("Ya los vi")
    expect(line.cases?.map((c) => c.title)).toEqual(["2026-03-04 Una nota", "2026-03-04 Otra nota"])
    expect(line.cases?.[0].actions?.[0]).toMatchObject({ kind: "go", target: { view: "document", locationId: 7 } })
    expect(line.moreCases).toBe(1)
    const lines = docbaseNotices(facts({}, { newClips: { total: 1, files: [clip(7, "a.md")] } }), words)
    expect(lines.map((n) => n.title)).toEqual(["1 recorte web nuevo"])
  })
})

describe("the readings Micelya Core refused to delete", () => {
  const now = Date.UTC(2026, 9, 22, 20, 0)
  const left = (id: string, name: string, refused: string | null) => ({
    id,
    name,
    askedAt: now - 5 * 60_000,
    kept: true,
    refused,
    triedAt: now,
  })

  it("says nothing while Core deleted everything this computer asked for", () => {
    expect(coreLeftNotice([])).toBeNull()
    expect(coreLeftNotice(null)).toBeNull()
    expect(docbaseNotices(facts({}, { coreLeft: [] }), words)).toEqual([])
  })

  it("is a problem with each reading, its reason, a retry and the connections", () => {
    const reason = "Micelya Core rejected the credential (API, 403 PERMISSION_DENIED)"
    const line = coreLeftNotice([left("l-1", "pagina-3.jpg", reason), left("l-2", "pagina-4.jpg", reason)], now)!
    expect(line).toMatchObject({ id: "docbase:core-readings", kind: "problem", priority: "high", count: 2 })
    expect(line.title).toBe("2 lecturas no se pudieron borrar en Micelya Core")
    expect(line.explanation).toContain("texto de sus páginas")
    expect(line.todo).toContain("Ajustes → Conexiones")
    expect(line.todo).toContain("cada 10 minutos")
    expect(line.resolvedWhen).toBe("Desaparece cuando se borran.")
    expect(line.actions.map((a) => a.label)).toEqual(["Reintentar ahora", "Ver conexiones"])
    expect(line.actions[1]).toMatchObject({ kind: "go", target: { view: "settings" } })
    expect(line.cases?.map((c) => c.title)).toEqual(["pagina-3.jpg", "pagina-4.jpg"])
    expect(line.cases?.[0].detail).toBe(`${reason} · pedida hace 5 min`)
    const lines = docbaseNotices(facts({}, { coreLeft: [left("l-1", "pagina-1.jpg", reason)] }), words)
    expect(lines.map((n) => n.title)).toEqual(["1 lectura no se pudo borrar en Micelya Core"])
  })
})
