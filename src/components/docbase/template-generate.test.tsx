import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import type { EntityDetail, GeneratedDocument, MatterParty, MatterSummary, Template, TemplateField } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import {
  applySuggestions,
  cardKindOf,
  dateStyleOf,
  entityOptions,
  formatDateIn,
  generationRequest,
  generationRows,
  initialValues,
  isoFromDate,
  matterValues,
  missingNames,
  preview,
  relatedRows,
  suggestedFileName,
  suggestionsFrom,
  todayIso,
  valuesFromDocument,
  withMatter,
  type FieldValues,
} from "./template-generate"
import { GeneratedNotice, GenerateForm } from "./template-generate-view"
import { TemplateDetail, TemplateTable } from "./templates-view"
import { docRef } from "./similar.fixtures"

// The form talks to the base only through its props here.
vi.mock("@/commands/docbase", () => ({}))

const words = vocabularyFor("legal", "es")
const WHATS = ["persona", "organizacion", "asunto", "fecha", "importe", "numero", "identificador", "texto"]

function field(kind: TemplateField["kind"], values: string[], extra: Partial<TemplateField> = {}): TemplateField {
  return {
    kind,
    ordinal: 1,
    formName: null,
    sameAs: null,
    values,
    name: null,
    nameSource: null,
    what: null,
    whatSource: null,
    fixed: false,
    fixedValue: null,
    ...extra,
  }
}

/** Two invented receipts: who pays, the amount (said twice), the date, an optional clause and a fixed place. */
function receipts(): Template {
  return {
    principal: 1,
    asked: 1,
    documents: [
      { doc: docRef(1, "Recibo enero.pdf"), state: null },
      { doc: docRef(2, "Recibo febrero.pdf"), state: "proposed" },
    ],
    state: "proposed",
    segments: [
      { text: "Recibí de " },
      { field: 0 },
      { text: " la suma de $ " },
      { field: 1 },
      { text: " el día " },
      { field: 2 },
      { text: ". " },
      { field: 3 },
      { text: " Son $ " },
      { field: 4 },
      { text: ". Lugar: " },
      { field: 5 },
      { text: "." },
    ],
    fields: [
      field("name", ["Ana Torres", "Bruno Vega"], { name: "Quien paga", nameSource: "person", what: "persona", whatSource: "person" }),
      field("amount", ["1.500,00", "2.750,00"], { name: "Importe", nameSource: "label", what: "importe", whatSource: "kind" }),
      field("date", ["10/01/2026", "10/02/2026"], { what: "fecha", whatSource: "kind" }),
      field("wording", ["", "Se paga en efectivo."], { what: "texto", whatSource: "kind" }),
      field("amount", ["1.500,00", "2.750,00"], { sameAs: 1, name: "Importe", nameSource: "label" }),
      field("wording", ["Córdoba", "Rosario"], { name: "Lugar", nameSource: "label", fixed: true, fixedValue: "Mendoza" }),
    ],
    possiblySame: [],
    computedAt: 1,
    docType: "recibo",
    proposedTypeName: null,
    typeRank: null,
    format: "pdf-text",
    name: "Recibo de honorarios",
    whats: WHATS,
    generated: [],
  }
}

const matters: MatterSummary[] = [
  { id: "m1", name: "Torres c/ Vega", status: "open", origin: "person", confirmation: "confirmed", documents: 3, proposed: 0, pagePath: null },
  { id: "m2", name: "Un caso descartado", status: "open", origin: "proposed", confirmation: "discarded", documents: 1, proposed: 1, pagePath: null },
]

function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim()
}

describe("generating a document from a template", () => {
  it("asks for each field once, in the order of the text, and says how each one is filled", () => {
    const rows = generationRows(receipts(), words)
    expect(rows.map((row) => [row.index, row.name, row.input])).toEqual([
      [0, "Quien paga", "entity"],
      [1, "Importe", "typed"],
      [2, "Fecha 1", "date"],
      [3, "Texto 1", "wording"],
      [5, "Lugar", "fixed"],
    ])
    expect(rows[0].entityKind).toBe("persona")
    expect(rows[1].places).toBe(2)
    expect(rows[1].example).toBe("1.500,00")
    expect(rows.map((row) => row.optional)).toEqual([false, false, false, true, true])
    expect(rows[3].versions).toEqual(["Se paga en efectivo."])
    // A name nobody said what it is: a person or organization, of any type.
    const unknown = { ...receipts(), fields: [field("name", ["Ana Torres", "Bruno Vega"])] }
    expect(generationRows(unknown, words).map((row) => [row.input, row.entityKind])).toEqual([["entity", null]])
    // A matter: by its reference type.
    const matter = { ...receipts(), fields: [field("name", ["Torres c/ Vega", "Ruiz c/ Paz"], { what: "asunto" })] }
    expect(generationRows(matter, words)[0].input).toBe("matter")
  })

  it("starts with the data of one of its documents, but for what is fixed", () => {
    const template = receipts()
    const rows = generationRows(template, words)
    const values = valuesFromDocument(template, rows, 1)
    expect(values[0]).toEqual({ value: "Bruno Vega", source: "document", reference: "content:2" })
    expect(values[3].value).toBe("Se paga en efectivo.")
    expect(values[5]).toBeUndefined()
  })

  it("previews the text with the values in place, what is missing in brackets and what is optional left out", () => {
    const template = receipts()
    const rows = generationRows(template, words)
    const values: FieldValues = { 1: { value: "3.000,00", source: "hand", reference: null } }
    const shown = preview(template, rows, values)
    const plain = shown.text
      .map((piece) => (piece.kind === "empty" ? "" : piece.text))
      .join("")
    expect(plain).toBe("Recibí de [Quien paga] la suma de $ 3.000,00 el día [Fecha 1].  Son $ 3.000,00. Lugar: Mendoza.")
    expect(shown.text.filter((piece) => piece.kind === "missing").map((piece) => piece.kind === "missing" && piece.text)).toEqual([
      "[Quien paga]",
      "[Fecha 1]",
    ])
    expect(shown.form).toEqual([])
    expect(missingNames(rows, values)).toEqual(["Quien paga", "Fecha 1"])
  })

  it("names the file by the template, its parties and the date", () => {
    const rows = generationRows(receipts(), words)
    const values: FieldValues = { 0: { value: "Carla Ruiz", source: "hand", reference: null } }
    expect(suggestedFileName("Recibo de honorarios", rows, values, "2026-10-22")).toBe("Recibo de honorarios - Carla Ruiz - 2026-10-22")
    expect(suggestedFileName("Recibo de honorarios", rows, {}, "2026-10-22")).toBe("Recibo de honorarios - 2026-10-22")
    expect(todayIso(new Date(2026, 9, 2))).toBe("2026-10-02")
  })

  it("sends every field once, with its name, its value and where it came from", () => {
    const template = receipts()
    const rows = generationRows(template, words)
    const values: FieldValues = {
      0: { value: "Carla Ruiz", source: "hand", reference: null },
      1: { value: "2.750,00", source: "document", reference: "content:2" },
    }
    const request = generationRequest(template, "Recibo de honorarios", rows, values, { matterId: "m1", fileName: "  Mi recibo ", fromDocument: 2 })
    expect(request.principal).toBe(1)
    expect(request.matterId).toBe("m1")
    expect(request.fileName).toBe("Mi recibo")
    expect(request.fromDocument).toBe(2)
    expect(request.values.map((v) => [v.position, v.name, v.value, v.source, v.reference])).toEqual([
      [0, "Quien paga", "Carla Ruiz", "hand", null],
      [1, "Importe", "2.750,00", "document", "content:2"],
      [2, "Fecha 1", "", "hand", null],
      [3, "Texto 1", "", "hand", null],
      [5, "Lugar", "Mendoza", "fixed", null],
    ])
  })

  it("shows the form with its notices, its fields, the preview and the name of the file", () => {
    const markup = renderToStaticMarkup(
      <GenerateForm template={receipts()} matters={matters} onGenerate={() => {}} today="2026-10-22" words={words} />,
    )
    const html = text(markup)
    expect(html).toContain("Generar un documento")
    expect(html).toContain("Desde la plantilla «Recibo de honorarios»: 4 campos para completar.")
    expect(html).toContain("Sale un Word simple, sin membrete")
    expect(html).not.toContain("es una propuesta")
    expect(html).toContain("Para qué caso (opcional)")
    expect(html).toContain("Torres c/ Vega")
    expect(html).not.toContain("Un caso descartado")
    expect(html).toContain("Empezar con los datos de un documento")
    expect(html).toContain("Recibo febrero.pdf")
    expect(html).toContain("Quien paga")
    expect(html).toContain("2 lugares del texto")
    expect(html).toContain("Fijo: «Mendoza»")
    expect(html).toContain("Ninguna: queda afuera")
    expect(html).toContain("Recibí de [Quien paga] la suma de $ [Importe]")
    // The date starts as today, written as the documents write it.
    expect(html).toContain("el día 22/10/2026")
    expect(html).toContain("Faltan 2 campos: «Quien paga», «Importe». Se puede generar igual")
    expect(markup).toContain('value="Recibo de honorarios - 2026-10-22"')
    expect(html).not.toContain("Lugar puede quedar vacío")
    expect(html).toContain("Generar el Word")
    const confirmed = text(
      renderToStaticMarkup(
        <GenerateForm template={{ ...receipts(), state: "confirmed" }} matters={[]} onGenerate={() => {}} today="2026-10-22" words={words} />,
      ),
    )
    expect(confirmed).not.toContain("es una propuesta")
  })

  it("is offered from each template of the list and from its detail", () => {
    const summary = {
      principal: docRef(1, "Recibo enero.pdf"),
      docType: "recibo",
      proposedTypeName: null,
      typeRank: null,
      name: null,
      state: "proposed" as const,
      documents: 2,
      fields: 4,
      wordingFields: 1,
      format: "pdf-text" as const,
      possiblySame: 0,
      waitingFields: 0,
      generated: 2,
    }
    const list = text(renderToStaticMarkup(<TemplateTable templates={[summary]} onOpen={() => {}} onGenerate={() => {}} words={words} />))
    expect(list).toContain("Generar")
    expect(list).toContain("2 documentos · 2 generados · 4 campos")
    const detail = text(renderToStaticMarkup(<TemplateDetail template={receipts()} onOpen={() => {}} onGenerate={() => {}} words={words} />))
    expect(detail).toContain("Generar un documento")
    const without = text(renderToStaticMarkup(<TemplateDetail template={receipts()} onOpen={() => {}} words={words} />))
    expect(without).not.toContain("Generar un documento")
  })

  it("says where the Word went and offers to open it", () => {
    const result: GeneratedDocument = {
      id: 7,
      path: "C:/trabajo/Generados/Recibo.docx",
      fileName: "Recibo.docx",
      locationId: 70,
      contentId: 9,
      inMatter: true,
      missing: ["Fecha 1"],
    }
    const html = text(
      renderToStaticMarkup(
        <GenerateForm
          template={receipts()}
          matters={matters}
          result={result}
          onGenerate={() => {}}
          onOpenResult={() => {}}
          onShowDocument={() => {}}
          onAgain={() => {}}
          words={words}
        />,
      ),
    )
    expect(html).toContain("Listo: «Recibo.docx» quedó en «Generados», en la carpeta de trabajo.")
    expect(html).toContain("Quedaron para completar en el Word, entre corchetes: «Fecha 1».")
    expect(html).toContain("ya está en su caso")
    expect(html).toContain("Abrir en Word")
    expect(html).toContain("Mostrar en la carpeta")
    expect(html).toContain("Ver el documento")
    expect(html).toContain("Generar otro")
  })
})


/** An invented lease: a tenant with her DNI and address, a landlord company with its CUIT, a date and the matter. */
function lease(): Template {
  return {
    ...receipts(),
    segments: [
      { text: "Entre " },
      { field: 0 },
      { text: ", DNI " },
      { field: 1 },
      { text: ", con domicilio en " },
      { field: 2 },
      { text: ", y " },
      { field: 3 },
      { text: ", CUIT " },
      { field: 4 },
      { text: ", el día " },
      { field: 5 },
      { text: ", en el caso " },
      { field: 6 },
      { text: "." },
    ],
    fields: [
      field("name", ["Ana Torres", "Carla Ruiz"], { name: "Locataria", nameSource: "model", what: "persona", whatSource: "model" }),
      field("identifier", ["30.123.456", "28.987.654"], { name: "DNI", nameSource: "label", what: "identificador", whatSource: "kind" }),
      field("wording", ["Calle Falsa 123", "Av. Inventada 456"], { name: "Domicilio de la locataria", nameSource: "model", what: "texto" }),
      field("name", ["Inmobiliaria Ejemplo S.A.", "Rentas Inventadas S.R.L."], { name: "Locadora", nameSource: "model", what: "organizacion" }),
      field("identifier", ["30-11111111-1", "30-22222222-2"], { what: "identificador", whatSource: "kind" }),
      field("date", ["10 de febrero de 2026", "3 de marzo de 2026"], { what: "fecha", whatSource: "kind" }),
      field("name", ["Torres c/ Vega", "Ruiz c/ Paz"], { name: "Expediente", nameSource: "model", what: "asunto" }),
    ],
  }
}

function card(): EntityDetail {
  return {
    id: "e-ana",
    kind: "persona",
    name: "Ana Torres",
    state: "active",
    mergedInto: null,
    aliases: [],
    identifiers: [
      { kind: "cuit", value: "27-30123456-4", documents: 1 },
      { kind: "dni", value: "30.123.456", documents: 4 },
    ],
    mentions: [],
    claims: [
      {
        id: 1,
        predicate: "domicilio",
        objectEntity: null,
        objectName: null,
        objectValue: "Calle Falsa 123, Córdoba",
        state: "supported",
        validFrom: null,
        evidence: [],
        certainty: "quoted",
        reasons: [],
      },
    ],
    doubts: [],
    pagePath: null,
    matters: [],
    decisions: [],
    incoming: [],
  }
}

describe("filling a document with data of the base", () => {
  it("writes today as the documents write their dates, and reads back what the calendar chose", () => {
    expect(dateStyleOf(["10/02/2026", "03/04/2026"])).toEqual({ kind: "numeric", separator: "/", padded: true, fullYear: true })
    expect(dateStyleOf(["10 de febrero de 2026", "3 de marzo de 2026"])).toEqual({ kind: "long" })
    expect(dateStyleOf(["1-2-26", "10-12-26"]).kind).toBe("numeric")
    expect(dateStyleOf([])).toEqual({ kind: "numeric", separator: "/", padded: true, fullYear: true })
    expect(formatDateIn("2026-10-02", dateStyleOf(["10/02/2026"]))).toBe("02/10/2026")
    expect(formatDateIn("2026-10-02", { kind: "long" })).toBe("2 de octubre de 2026")
    expect(formatDateIn("2026-10-02", dateStyleOf(["1-2-26"]))).toBe("2-10-26")
    expect(isoFromDate("02/10/2026")).toBe("2026-10-02")
    expect(isoFromDate("2 de octubre de 2026")).toBe("2026-10-02")
    expect(isoFromDate("el lunes")).toBeNull()
    const rows = generationRows(lease(), words)
    expect(initialValues(rows, "2026-10-22")[5]).toEqual({ value: "22 de octubre de 2026", source: "date", reference: null })
    expect(Object.keys(initialValues(rows, "2026-10-22"))).toEqual(["5"])
  })

  it("knows which fields are the DNI, the CUIT and the address of the person before them", () => {
    const rows = generationRows(lease(), words)
    expect(rows.map((row) => row.input)).toEqual(["entity", "typed", "wording", "entity", "typed", "date", "matter"])
    expect(rows.map((row) => cardKindOf(row, words))).toEqual([null, "dni", "domicilio", null, "cuit", null, null])
    const tenant = relatedRows(rows, rows[0], words)
    expect(tenant.map((r) => [r.row.index, r.card])).toEqual([
      [1, "dni"],
      [2, "domicilio"],
    ])
    expect(relatedRows(rows, rows[3], words).map((r) => [r.row.index, r.card])).toEqual([[4, "cuit"]])
  })

  it("suggests the DNI and the address from what the base knows, never over what a person wrote", () => {
    const rows = generationRows(lease(), words)
    const suggested = suggestionsFrom(card(), relatedRows(rows, rows[0], words))
    expect(suggested).toEqual({
      1: { value: "30.123.456", source: "card", reference: "entity:e-ana" },
      2: { value: "Calle Falsa 123, Córdoba", source: "card", reference: "entity:e-ana" },
    })
    const written: FieldValues = { 2: { value: "Otra calle 1", source: "hand", reference: null } }
    const applied = applySuggestions(written, suggested)
    expect(applied[1].value).toBe("30.123.456")
    expect(applied[2].value).toBe("Otra calle 1")
    // A later choice replaces an earlier suggestion.
    const again = applySuggestions(applied, { 1: { value: "99.999.999", source: "card", reference: "entity:e-otra" } })
    expect(again[1].value).toBe("99.999.999")
  })

  it("offers the people of the matter first, of the field's type, then those the search found", () => {
    const parties: MatterParty[] = [
      { entityId: "e-ana", kind: "persona", name: "Ana Torres", roles: ["parte"], documents: 3 },
      { entityId: "o-1", kind: "organizacion", name: "Inmobiliaria Ejemplo S.A.", roles: ["parte"], documents: 2 },
    ]
    const found = [
      { id: "e-ana", kind: "persona", name: "Ana Torres", mentions: 9, documents: 4, aliases: [] },
      { id: "e-ant", kind: "persona", name: "Antonio Pérez", mentions: 2, documents: 1, aliases: [] },
    ]
    expect(entityOptions("persona", "", parties, found).map((o) => [o.name, o.ofMatter])).toEqual([
      ["Ana Torres", true],
      ["Antonio Pérez", false],
    ])
    // What is typed narrows the matter's; the search already looked for it.
    expect(entityOptions("persona", "anto", parties, [found[1]]).map((o) => o.name)).toEqual(["Antonio Pérez"])
    expect(entityOptions(null, "", parties, []).map((o) => o.name)).toEqual(["Ana Torres", "Inmobiliaria Ejemplo S.A."])
  })

  it("fills the fields that are a matter with the matter chosen, by its name or the number of its record", () => {
    expect(matterValues({ name: "Torres c/ Vega (CIV 1234/2026)" })).toEqual([
      { label: "su nombre", value: "Torres c/ Vega (CIV 1234/2026)" },
      { label: "su número de expediente", value: "CIV 1234/2026" },
    ])
    expect(matterValues({ name: "Torres c/ Vega" })).toHaveLength(1)
    const rows = generationRows(lease(), words)
    const filled = withMatter({}, rows, { id: "m1", name: "Torres c/ Vega" })
    expect(filled[6]).toEqual({ value: "Torres c/ Vega", source: "matter", reference: "matter:m1" })
    expect(withMatter({ 6: { value: "Otro", source: "hand", reference: null } }, rows, { id: "m1", name: "Torres c/ Vega" })[6].value).toBe("Otro")
    expect(withMatter(filled, rows, null)[6]).toBeUndefined()
  })

  it("shows a person to choose from the base, the calendar of a date and the matters for a matter", () => {
    const markup = renderToStaticMarkup(
      <GenerateForm template={lease()} matters={matters} onGenerate={() => {}} today="2026-10-22" words={words} />,
    )
    const html = text(markup)
    expect(html).toContain("Elegí de «Personas y organizaciones» o escribilo a mano.")
    expect(markup).toContain('type="date"')
    expect(markup).toContain('value="22 de octubre de 2026"')
    expect(markup).toContain('value="2026-10-22"')
    expect(html).toContain("Elegir un caso…")
    expect(html).toContain("Torres c/ Vega")
    expect(html).toContain("el día 22 de octubre de 2026")
  })
})


describe("what was generated, known as such", () => {
  it("says in the document when and from which template it was generated, and for which matter", () => {
    const at = new Date(2026, 9, 22, 12).getTime()
    const line = { id: 3, generatedAt: at, templateName: "Recibo de honorarios", templateLocation: 10, matterId: "m1", matterName: "Torres c/ Vega" }
    const html = text(renderToStaticMarkup(<GeneratedNotice line={line} onOpenTemplate={() => {}} words={words} />))
    expect(html).toContain("Generado el 22/10/2026 desde la plantilla «Recibo de honorarios» para el caso «Torres c/ Vega».")
    expect(html).toContain("Ver la plantilla")
    const alone = text(
      renderToStaticMarkup(<GeneratedNotice line={{ ...line, matterId: null, matterName: null, templateLocation: null }} onOpenTemplate={() => {}} words={words} />),
    )
    expect(alone).toContain("Generado el 22/10/2026 desde la plantilla «Recibo de honorarios».")
    expect(alone).not.toContain("Ver la plantilla")
  })

  it("lists in the template the documents generated with it, apart from the documents it came from", () => {
    const at = new Date(2026, 9, 22, 12).getTime()
    const template: Template = {
      ...receipts(),
      generated: [
        { id: 2, generatedAt: at, fileName: "Recibo - Carla Ruiz - 2026-10-22.docx", matterName: "Torres c/ Vega", locationId: 90 },
        { id: 1, generatedAt: at, fileName: "Recibo - 2026-10-22.docx", matterName: null, locationId: null },
      ],
    }
    const html = text(renderToStaticMarkup(<TemplateDetail template={template} onOpen={() => {}} words={words} />))
    expect(html).toContain("Documentos generados con esta plantilla (2)")
    expect(html).toContain("Recibo - Carla Ruiz - 2026-10-22.docx el 22/10/2026 · para el caso «Torres c/ Vega» Abrir")
    expect(html).toContain("Recibo - 2026-10-22.docx el 22/10/2026 todavía no está en la base")
    // The documents of the template are still its two.
    expect(html).toContain("2 documentos · 5 campos")
    const none = text(renderToStaticMarkup(<TemplateDetail template={receipts()} onOpen={() => {}} words={words} />))
    expect(none).toContain("Documentos generados con esta plantilla (0) Todavía no se generó ninguno.")
  })
})
