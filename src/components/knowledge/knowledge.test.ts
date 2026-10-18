import { describe, expect, it } from "vitest"
import type { DocumentMatter, DoubtView, EntityMention, IdentityDecisionView } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import { certaintyInWords, decisionInWords, documentsByMatter, doubtReason, groupDoubts, reviewInWords, rolesInWords } from "./knowledge"

const words = vocabularyFor("legal", "es")

function doubt(a: string, b: string, patch: Partial<DoubtView> = {}): DoubtView {
  return {
    a: `cluster:1:persona:${a.toLowerCase()}`,
    b: `cluster:2:persona:${b.toLowerCase()}`,
    kind: "persona",
    entityA: `ent-${a}`,
    entityB: `ent-${b}`,
    nameA: a,
    nameB: b,
    reason: "modelo-dudosa",
    detail: null,
    evidenceHash: "h",
    ...patch,
  }
}

function mention(contentId: number, document: string, role: string, patch: Partial<EntityMention> = {}): EntityMention {
  return {
    mentionId: contentId * 10 + role.length,
    contentId,
    locationId: contentId + 100,
    document,
    role,
    written: "Persona Uno",
    quote: "firmado por Persona Uno",
    page: 1,
    resolvedBy: "nombre",
    ...patch,
  }
}

describe("groupDoubts", () => {
  it("joins the doubts that share an entity into one decision", () => {
    const groups = groupDoubts([doubt("Uno", "Uno Bis"), doubt("Uno", "U Bis"), doubt("Dos", "D")])
    expect(groups).toHaveLength(2)
    // The larger group first, named by the side in most doubts.
    expect(groups[0].sides.map((s) => s.name)).toEqual(["Uno", "U Bis", "Uno Bis"])
    expect(groups[0].doubts).toHaveLength(2)
    expect(groups[1].sides.map((s) => s.name)).toEqual(["D", "Dos"])
  })

  it("chains through a shared side and keeps groups not yet resolved by their key", () => {
    const groups = groupDoubts([
      doubt("A", "B"),
      doubt("B", "C"),
      doubt("X", "Y", { entityA: null, entityB: null, a: "cluster:9:persona:x", b: "cluster:8:persona:y" }),
    ])
    expect(groups.map((g) => g.sides.length)).toEqual([3, 2])
    expect(groups[1].sides.every((s) => s.entityId === null)).toBe(true)
  })

  it("is empty without doubts", () => {
    expect(groupDoubts([])).toEqual([])
  })
})

describe("documentsByMatter", () => {
  const mentions = [
    mention(1, "carta.pdf", "remitente"),
    mention(1, "carta.pdf", "firmante", { written: "P. Uno" }),
    mention(2, "factura.pdf", "destinatario"),
    mention(3, "nota.txt", "parte"),
    mention(4, "correo.eml", "remitente"),
  ]
  const matters: DocumentMatter[] = [
    { contentId: 1, matterId: "mat-1", name: "Reclamo", proposed: false, pagePath: "wiki/entities/reclamo.md" },
    { contentId: 2, matterId: "mat-1", name: "Reclamo", proposed: false, pagePath: "wiki/entities/reclamo.md" },
    { contentId: 4, matterId: "mat-1", name: "Reclamo", proposed: true, pagePath: "wiki/entities/reclamo.md" },
  ]

  it("groups the documents under their matter with every role, the loose ones last", () => {
    const groups = documentsByMatter(mentions, matters, words.knowledge.withoutMatter)
    expect(groups.map((g) => g.name)).toEqual(["Reclamo", "Sin asunto identificado"])
    expect(groups[0].documents.map((d) => d.name)).toEqual(["carta.pdf", "correo.eml", "factura.pdf"])
    const letter = groups[0].documents.find((d) => d.name === "carta.pdf")
    expect(letter?.roles).toEqual(["remitente", "firmante"])
    expect(letter?.mentions).toHaveLength(2)
    // A proposal is shown under the matter, marked, and also among the loose ones.
    expect(groups[0].documents.find((d) => d.name === "correo.eml")?.proposed).toBe(true)
    expect(groups[1].documents.map((d) => d.name)).toEqual(["correo.eml", "nota.txt"])
  })
})

describe("words", () => {
  it("says why two could be the same", () => {
    expect(doubtReason("identificador-parecido", "dni: 1 / 2")).toContain("dni: 1 / 2")
    expect(doubtReason("modelo-dudosa", "faltan datos")).toContain("«faltan datos»")
    expect(doubtReason("algo-nuevo")).toBe("La app no pudo decidir si son la misma.")
    expect(doubtReason("modelo-misma-nombre-distinto", "certeza media: una letra")).toContain("difieren en una letra")
  })

  it("names the roles with the vocabulary", () => {
    expect(rolesInWords(["domicilio_constituido", "parte"], words)).toBe("domicilio constituido, parte")
    expect(rolesInWords(["rol-nuevo"], words)).toBe("rol nuevo")
  })

  it("tells a decision about the entity", () => {
    const decision = (kind: string, a: string, b: string | null): IdentityDecisionView => ({
      id: 1,
      kind,
      a,
      b,
      note: null,
      shownEvidence: "{}",
      state: "active",
      decidedAt: 0,
      undoneAt: null,
    })
    const entity = { mentions: [mention(7, "acta.pdf", "parte")] }
    expect(decisionInWords(decision("join", "cluster:7:persona:uno", "cluster:8:persona:uno bis"), entity)).toBe(
      "Son la misma: «uno» y «uno bis»",
    )
    expect(decisionInWords(decision("separate", "cluster:7:persona:uno", "cluster:8:persona:otra"), entity)).toContain("(en acta.pdf)")
    expect(decisionInWords(decision("wrong", "claim:ent-1:cargo:value:x", null), entity)).toBe("Un dato marcado como incorrecto")
    expect(decisionInWords(decision("wrong", "cluster:7:persona:mi", null), entity)).toBe("Una mención que no es una entidad (en acta.pdf)")
  })
})

describe("certaintyInWords", () => {
  it("says a claim has its quote checked, never that it is verified", () => {
    expect(certaintyInWords({ certainty: "quoted", reasons: [] })).toEqual({ label: "con cita comprobada", tone: "plain", why: "" })
  })

  it("says why a doubtful claim is not confirmed", () => {
    const said = certaintyInWords({ certainty: "doubtful", reasons: ["attribution", "by-person"] })
    expect(said.label).toBe("sin confirmar")
    expect(said.tone).toBe("doubt")
    expect(said.why).toBe("la cita nombra a otra persona u organización")
  })

  it("names the word of a person", () => {
    expect(certaintyInWords({ certainty: "confirmed", reasons: [] }).label).toBe("confirmado por una persona")
    expect(certaintyInWords({ certainty: "corrected", reasons: ["by-person"] }).tone).toBe("person")
  })

  it("tells why a decision waits for a review", () => {
    expect(reviewInWords("no-encontrada")).toContain("ya no aparece")
    expect(reviewInWords("varias")).toContain("más de una")
  })
})
