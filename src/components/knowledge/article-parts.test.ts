import { describe, expect, it } from "vitest"
import {
  certaintyMark,
  certaintyWords,
  documentGroups,
  factGroups,
  initials,
  leadParts,
  parseArticle,
  partyRows,
  proposedRows,
  relatedCards,
  relationRows,
  saysParts,
} from "./article-parts"

// The words of the pages, as knowledge.json has them (only the ones read here).
const WORDS: Record<string, string> = {
  says: "Lo que dicen los documentos",
  documents: "Documentos",
  relations: "Vínculos",
  related: "Relacionados",
  matterParties: "Partes y personas",
  matterFacts: "Lo que dicen los documentos del asunto",
  matterProposed: "Posiblemente de este asunto",
  matterWhy: "Por qué estos documentos van juntos",
  proposed: "propuesto",
  certainLegend: "Cada dato tiene su cita. Lo marcado «sin confirmar» es dudoso.",
  conflicts: "Datos en los que los documentos difieren",
  saysMore: "Hay {n} datos más de los documentos, cada uno con su cita, en «Datos y correcciones».",
  saysMoreOne: "Hay un dato más de los documentos, con su cita, en «Datos y correcciones».",
  certainDoubtful: "sin confirmar",
  certainConfirmed: "confirmado por una persona",
  certainCorrected: "corregido por una persona",
}
const word = (id: string) => WORDS[id] ?? id

// A page of a made-up person, with the form the app writes.
const PERSON = [
  "*Persona* · También escrito como: Ana Inventada · INVENTADA ANA · DNI 11.111.111 (2 documentos)",
  "",
  "## Lo que dicen los documentos",
  "",
  "Ana Inventada trabaja para [[empresa-ficticia|Empresa Ficticia SA]].[1](#micelya-nota-1) Está casada *(sin confirmar)*.",
  "",
  `*${WORDS.certainLegend}*`,
  "",
  `**${WORDS.conflicts}**`,
  "",
  "- Domicilio: «Calle Falsa 1»[2](#micelya-nota-2) · «Calle Falsa 10»[3](#micelya-nota-3)",
  "",
  "Hay 3 datos más de los documentos, cada uno con su cita, en «Datos y correcciones».",
  "",
  "## Documentos (3)",
  "",
  "### [[reclamo-inventado|Reclamo inventado]]",
  "",
  "- [[carta-de-prueba|Carta de prueba]] — destinataria · carta documento",
  "- [[recibo-de-prueba|Recibo de prueba]] — parte (propuesto)",
  "",
  "### Sin asunto identificado",
  "",
  "- Nota suelta — firmante · nota",
  "",
  "## Vínculos",
  "",
  "- Empleado de [[empresa-ficticia|Empresa Ficticia SA]][1](#micelya-nota-1)",
  "- Cónyuge de *(sin confirmar: no es seguro de quién se habla)* [[pedro-inventado|Pedro Inventado]][4](#micelya-nota-4)",
  "- Hijo o hija de Nadie Conocido",
  "",
  "## Relacionados",
  "",
  "- [[empresa-ficticia|Empresa Ficticia SA]] · Organización — 2 documentos en común",
  "- [[pedro-inventado|Pedro Inventado]] · Persona",
].join("\n")

const MATTER = [
  "*Asunto* · Se lo nombra como: Inventada c/ Ficticia · Expediente 123/2026",
  "",
  "## Partes y personas",
  "",
  "- [[ana-inventada|Ana Inventada]] · Persona — parte, remitente (2 documentos)",
  "- Empresa Sin Página · Organización — parte (1 documento)",
  "",
  "## Lo que dicen los documentos del asunto",
  "",
  "- [[ana-inventada|Ana Inventada]] — Empleado de: [[empresa-ficticia|Empresa Ficticia SA]][1](#micelya-nota-1)",
  "- [[ana-inventada|Ana Inventada]] — Domicilio: Calle Falsa 1 (datos en los que los documentos difieren)[2](#micelya-nota-2)",
  "- [[empresa-ficticia|Empresa Ficticia SA]] — Domicilio: Avenida Inventada 5[3](#micelya-nota-3)",
  "",
  "## Documentos (2)",
  "",
  "- [[carta-de-prueba|Carta de prueba]] · carta documento",
  "- [[recibo-de-prueba|Recibo de prueba]]",
  "",
  "### Posiblemente de este asunto",
  "",
  "*sin confirmar: una persona los confirma o los descarta en «Datos y correcciones»*",
  "",
  "- [[correo-de-prueba|Correo de prueba]] — Ofrece pagar en cuotas el mismo reclamo.",
  "",
  "*Por qué estos documentos van juntos* — 2 documentos: comparten el número (2)",
].join("\n")

describe("the parts of an article", () => {
  it("finds each section of a person's page by the words of its headings, with its count", () => {
    const parts = parseArticle(PERSON, word)
    expect(parts.lead).toContain("*Persona*")
    expect(parts.sections.map((s) => [s.kind, s.heading, s.count])).toEqual([
      ["says", "Lo que dicen los documentos", null],
      ["documents", "Documentos", 3],
      ["relations", "Vínculos", null],
      ["related", "Relacionados", null],
    ])
    expect(new Set(parts.sections.map((s) => s.id)).size).toBe(4)
  })

  it("takes the documents a model proposes for a matter out as their own section, and leaves why they go together", () => {
    const parts = parseArticle(MATTER, word)
    expect(parts.sections.map((s) => s.kind)).toEqual(["parties", "facts", "documents", "proposed"])
    const documents = parts.sections[2]
    expect(documents.body).toContain("*Por qué estos documentos van juntos*")
    expect(documents.body).not.toContain("Correo de prueba")
    const proposed = parts.sections[3]
    expect(proposed.body).toContain("Correo de prueba")
    expect(proposed.body).not.toContain("Por qué")
  })

  it("an unknown heading is a section shown as written", () => {
    const parts = parseArticle("Texto suelto.\n\n## Algo nuevo\n\nUn párrafo.", word)
    expect(parts.lead).toBe("Texto suelto.")
    expect(parts.sections).toEqual([{ kind: "other", heading: "Algo nuevo", count: null, body: "Un párrafo.", id: "seccion-1" }])
  })

  it("says what follows the type in the first line, and keeps the rest", () => {
    expect(leadParts("*Asunto* · Se lo nombra como: X · Expediente 1/2026")).toEqual({
      facts: "Se lo nombra como: X · Expediente 1/2026",
      rest: [],
    })
    expect(leadParts("*Grupo de sociedades*\n\nSociedades del grupo: A · B").rest).toEqual(["Sociedades del grupo: A · B"])
    expect(leadParts("Sin tipo.").facts).toBe("")
  })
})

describe("what the documents say", () => {
  it("separates the synthesis, the legend, the conflicts and how many data more", () => {
    const says = saysParts(parseArticle(PERSON, word).sections[0].body, word)
    expect(says.lede).toHaveLength(1)
    expect(says.lede[0]).toContain("Ana Inventada trabaja")
    expect(says.legend).toBe(WORDS.certainLegend)
    expect(says.conflicts).toEqual(["Domicilio: «Calle Falsa 1»[2](#micelya-nota-2) · «Calle Falsa 10»[3](#micelya-nota-3)"])
    expect(says.more).toContain("Hay 3 datos más")
    expect(says.rest).toEqual([])
  })
})

describe("the lists of a page", () => {
  it("reads the documents of a person by matter, with their roles, type and proposals", () => {
    const read = documentGroups(parseArticle(PERSON, word).sections[1].body, word, true)
    expect(read?.groups).toEqual([
      {
        heading: "[[reclamo-inventado|Reclamo inventado]]",
        rows: [
          { document: "[[carta-de-prueba|Carta de prueba]]", roles: "destinataria", type: "carta documento", proposed: false },
          { document: "[[recibo-de-prueba|Recibo de prueba]]", roles: "parte", type: null, proposed: true },
        ],
      },
      { heading: "Sin asunto identificado", rows: [{ document: "Nota suelta", roles: "firmante", type: "nota", proposed: false }] },
    ])
  })

  it("reads the documents of a matter with their type, and keeps the line of why", () => {
    const read = documentGroups(parseArticle(MATTER, word).sections[2].body, word, false)
    expect(read?.groups[0].rows).toEqual([
      { document: "[[carta-de-prueba|Carta de prueba]]", roles: null, type: "carta documento", proposed: false },
      { document: "[[recibo-de-prueba|Recibo de prueba]]", roles: null, type: null, proposed: false },
    ])
    expect(read?.extra[0]).toContain("Por qué estos documentos van juntos")
  })

  it("gives up on a list with another form, so the section is shown as written", () => {
    expect(documentGroups("- Un documento sin rol", word, true)).toBeNull()
  })

  it("reads the documents a model proposes, without the note under the heading", () => {
    expect(proposedRows(parseArticle(MATTER, word).sections[3].body)).toEqual([
      { document: "[[correo-de-prueba|Correo de prueba]]", reason: "Ofrece pagar en cuotas el mismo reclamo." },
    ])
  })

  it("reads the links, with the mark of how sure after the name", () => {
    expect(relationRows(parseArticle(PERSON, word).sections[2].body)).toEqual([
      { label: "Empleado de", value: "[[empresa-ficticia|Empresa Ficticia SA]][1](#micelya-nota-1)" },
      {
        label: "Cónyuge de",
        value: "[[pedro-inventado|Pedro Inventado]] *(sin confirmar: no es seguro de quién se habla)*[4](#micelya-nota-4)",
      },
      { label: "", value: "Hijo o hija de Nadie Conocido" },
    ])
  })

  it("reads the related ones as cards, with their initials", () => {
    expect(relatedCards(parseArticle(PERSON, word).sections[3].body)).toEqual([
      { link: "[[empresa-ficticia|Empresa Ficticia SA]]", name: "Empresa Ficticia SA", type: "Organización", shared: "2 documentos en común" },
      { link: "[[pedro-inventado|Pedro Inventado]]", name: "Pedro Inventado", type: "Persona", shared: null },
    ])
    expect(initials("Talleres Inventados S.A.")).toBe("TI")
    expect(initials("Luis Alberto Ficticio")).toBe("LF")
    expect(initials("«Pedro» de la Nada")).toBe("PN")
    expect(initials("acme")).toBe("A")
  })

  it("reads the parties of a matter, linked or not", () => {
    expect(partyRows(parseArticle(MATTER, word).sections[0].body)).toEqual([
      { who: "[[ana-inventada|Ana Inventada]]", type: "Persona", roles: "parte, remitente", documents: 2 },
      { who: "Empresa Sin Página", type: "Organización", roles: "parte", documents: 1 },
    ])
  })

  it("groups what the documents of a matter say by whom", () => {
    const groups = factGroups(parseArticle(MATTER, word).sections[1].body)
    expect(groups?.map((g) => [g.subject, g.rows.map((r) => r.label)])).toEqual([
      ["[[ana-inventada|Ana Inventada]]", ["Empleado de", "Domicilio"]],
      ["[[empresa-ficticia|Empresa Ficticia SA]]", ["Domicilio"]],
    ])
  })
})

describe("how sure a page is of a datum", () => {
  const words = certaintyWords(word)

  it("tells «sin confirmar», with why, from a person's word and from any other emphasis", () => {
    expect(certaintyMark("(sin confirmar)", words)).toEqual({ tone: "doubt", label: "sin confirmar", why: null })
    expect(certaintyMark("(sin confirmar: no es seguro de quién se habla)", words)).toEqual({
      tone: "doubt",
      label: "sin confirmar",
      why: "no es seguro de quién se habla",
    })
    expect(certaintyMark("(confirmado por una persona)", words)?.tone).toBe("person")
    expect(certaintyMark("Persona", words)).toBeNull()
    expect(certaintyMark("(otra cosa)", words)).toBeNull()
  })
})
