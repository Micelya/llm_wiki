import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }))

import { invoke } from "@tauri-apps/api/core"
import type { DoubtView, KnowledgeAttention, KnowledgeStatus } from "@/commands/docbase"
import { attentionCounts } from "@/lib/attention/order"
import { vocabularyFor } from "@/vocabulary"
import { knowledgeNotices } from "./knowledge"

const words = vocabularyFor("legal", "es")

function status(patch: Partial<KnowledgeStatus> = {}): KnowledgeStatus {
  return {
    documents: 10,
    extracted: 10,
    stale: 0,
    older: 0,
    reread: 0,
    pending: 0,
    failed: 0,
    mentions: 40,
    identifiers: 5,
    evidence: 20,
    discarded: 0,
    discardedByReason: {},
    answers: 12,
    entities: 8,
    entitiesByKind: { persona: 5, organizacion: 3 },
    doubts: 0,
    claims: 15,
    conflicts: 0,
    toResolve: 0,
    pages: 6,
    toProject: 0,
    engine: { engine: "codex-cli", model: "m" },
    ...patch,
  }
}

function attention(patch: Partial<KnowledgeAttention> = {}): KnowledgeAttention {
  return {
    status: status(),
    failed: [],
    noisy: [],
    conflicts: [],
    identifierJoins: [],
    proposedMatters: [],
    mattersToConfirm: [],
    synthesisPending: 0,
    discardedSentences: 0,
    keptPages: [],
    leftWithDocuments: [],
    decisionsToReview: [],
    ...patch,
  }
}

const doc = (contentId: number, name: string) => ({ contentId, locationId: contentId + 100, name })

function doubt(a: string, b: string): DoubtView {
  return {
    a: `cluster:1:persona:${a}`,
    b: `cluster:2:persona:${b}`,
    kind: "persona",
    entityA: `ent-${a}`,
    entityB: `ent-${b}`,
    nameA: a,
    nameB: b,
    reason: "identificador-parecido",
    detail: "dni: 1 / 2",
    evidenceHash: "h",
  }
}

describe("knowledgeNotices", () => {
  it("raises nothing when the record is read and nothing waits", () => {
    expect(knowledgeNotices({ attention: attention(), doubts: [] }, words)).toEqual([])
  })

  it("makes one decision per group of doubts, not per pair", () => {
    const notices = knowledgeNotices({ attention: attention(), doubts: [doubt("uno", "uno bis"), doubt("uno", "u bis"), doubt("dos", "d")] }, words)
    const doubts = notices.find((n) => n.topic === "identity-doubts")
    expect(doubts?.kind).toBe("decision")
    expect(doubts?.count).toBe(2)
    expect(doubts?.cases).toHaveLength(2)
    expect(doubts?.actions[0]).toMatchObject({ kind: "go", target: { view: "identity-doubts" } })
    expect(attentionCounts(notices).decisions).toBe(2)
  })

  it("names the problems of reading with their documents", () => {
    const notices = knowledgeNotices(
      {
        attention: attention({
          failed: [{ document: doc(1, "carta.pdf"), error: "La respuesta no tiene la forma pedida" }],
          noisy: [{ document: doc(2, "escaneo.pdf"), kept: 3, discarded: 9 }],
        }),
        doubts: [],
      },
      words,
    )
    const failed = notices.find((n) => n.topic === "reading-failed")
    expect(failed?.kind).toBe("problem")
    expect(failed?.cases?.[0]).toMatchObject({ title: "carta.pdf", detail: "La respuesta no tiene la forma pedida" })
    expect(failed?.cases?.[0].actions?.[0]).toMatchObject({ target: { view: "document", locationId: 101 } })
    expect(failed?.actions?.[0]).toMatchObject({ kind: "run", label: "Volver a leer" })
    const noisy = notices.find((n) => n.topic === "reading-noisy")
    expect(noisy?.cases?.[0].detail).toContain("9 citas descartadas")
    expect(attentionCounts(notices).problems).toBe(2)
  })

  it("lists the matters the app proposes, with why their documents are together", () => {
    const notices = knowledgeNotices(
      {
        attention: attention({
          mattersToConfirm: [{ id: "mat-1", name: "Reclamo de Una a Otra", documents: 3, reasons: { numero: 2, asunto: 3 } }],
        }),
        doubts: [],
      },
      words,
    )
    const toConfirm = notices.find((n) => n.topic === "matters-to-confirm")
    expect(toConfirm?.kind).toBe("decision")
    expect(toConfirm?.title).toContain(words.knowledge.matter.toLowerCase())
    expect(toConfirm?.cases?.[0]).toMatchObject({
      title: "Reclamo de Una a Otra",
      detail: "3 documentos · comparten la misma referencia (3), el mismo número (2)",
    })
    expect(toConfirm?.cases?.[0].actions?.map((a) => a.label)).toEqual(["Confirmar", "Descartar", "Ver"])
  })

  it("counts conflicts and proposals, and leaves joins by identifier and kept pages out of the counter", () => {
    const notices = knowledgeNotices(
      {
        attention: attention({
          conflicts: [
            { entityId: "ent-1", entityName: "Persona Uno", kind: "persona", predicate: "domicilio", within: null, values: ["Calle 1", "Calle 2"], claims: [1, 2] },
          ],
          proposedMatters: [{ document: doc(3, "correo.eml"), matterId: "mat-1", matterName: "Reclamo", reason: "por los hechos" }],
          identifierJoins: [
            { entityId: "ent-2", name: "Uno", kind: "persona", names: ["Uno", "U. Bis"], identifiers: ["20.000.000"], documents: 2, readAt: 0 },
          ],
          keptPages: [{ entityId: "ent-3", path: "wiki/entities/x.md", name: "X", mergedInto: null }],
        }),
        doubts: [],
      },
      words,
    )
    const conflicts = notices.find((n) => n.topic === "conflicts")
    expect(conflicts?.cases?.[0]).toMatchObject({ title: "Persona Uno: domicilio", detail: "«Calle 1» / «Calle 2»" })
    const proposed = notices.find((n) => n.topic === "proposed-matters")
    expect(proposed?.title).toContain(words.knowledge.matter.toLowerCase())
    expect(proposed?.cases?.[0].actions?.map((a) => a.label)).toEqual(["Confirmar", "Descartar", "Ver"])
    expect(notices.find((n) => n.topic === "identifier-joins")?.priority).toBe("low")
    expect(notices.find((n) => n.topic === "kept-pages")?.priority).toBe("low")
    expect(attentionCounts(notices).decisions).toBe(2)
  })

  it("lists the decisions of a person that no longer find what they named, and undoes one from its line", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined)
    const decision = {
      id: 7,
      kind: "join",
      a: "cluster:3:persona:persona uno",
      b: "cluster:4:persona:persona bis",
      note: null,
      shownEvidence: "[]",
      state: "active" as const,
      decidedAt: 0,
      undoneAt: null,
      review: "no-encontrada",
      appliedAs: null,
    }
    const notices = knowledgeNotices(
      { attention: attention({ decisionsToReview: [{ decision, document: doc(3, "nota.pdf"), reason: "no-encontrada" }] }), doubts: [] },
      words,
    )
    const review = notices.find((n) => n.topic === "decisions-to-review")
    expect(review?.kind).toBe("decision")
    expect(review?.cases?.[0].title).toBe("Son la misma: «persona uno» y «persona bis» (en nota.pdf)")
    expect(review?.cases?.[0].detail).toContain("ya no aparece")
    expect(review?.cases?.[0].actions?.map((a) => a.label)).toEqual(["Abrir", "Deshacer la decisión"])
    const undo = review?.cases?.[0].actions?.[1]
    if (undo?.kind === "run") await undo.run()
    expect(invoke).toHaveBeenCalledWith("docbase_knowledge_undo", { id: 7 })
  })

  it("confirms or turns down a proposal from its line", async () => {
    const notices = knowledgeNotices(
      {
        attention: attention({
          proposedMatters: [{ document: doc(3, "correo.eml"), matterId: "mat-1", matterName: "Reclamo", reason: null }],
        }),
        doubts: [],
      },
      words,
    )
    const actions = notices.find((n) => n.topic === "proposed-matters")?.cases?.[0].actions ?? []
    const confirm = actions[0]
    const reject = actions[1]
    if (confirm.kind !== "run" || reject.kind !== "run") throw new Error("expected run actions")
    await confirm.run()
    expect(invoke).toHaveBeenCalledWith("docbase_knowledge_join_matter", expect.objectContaining({ document: "content:3", matterId: "mat-1" }))
    await reject.run()
    expect(invoke).toHaveBeenCalledWith("docbase_knowledge_leave_matter", expect.objectContaining({ document: "content:3", matterId: "mat-1" }))
  })

  it("leaves the reading, the resolution and the pages in progress to the stages of «En curso»", () => {
    const notices = knowledgeNotices(
      {
        attention: attention({ status: status({ pending: 4, extracted: 6, toResolve: 2, toProject: 3, engine: null }), synthesisPending: 2 }),
        doubts: [],
      },
      words,
    )
    expect(notices.filter((n) => n.kind === "ongoing")).toEqual([])
    expect(attentionCounts(notices)).toEqual({ problems: 0, decisions: 0, ongoing: 0 })
  })
})
