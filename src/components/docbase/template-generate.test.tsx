import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import type { GeneratedDocument, MatterSummary, Template, TemplateField } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import {
  generationRequest,
  generationRows,
  missingNames,
  preview,
  suggestedFileName,
  todayIso,
  valuesFromDocument,
  type FieldValues,
} from "./template-generate"
import { GenerateForm } from "./template-generate-view"
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
    expect(html).toContain("Esta plantilla es una propuesta: nadie confirmó todavía que sus documentos son de la misma familia. Se puede generar igual")
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
    expect(html).toContain("Faltan 3 campos: «Quien paga», «Importe», «Fecha 1». Se puede generar igual")
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
    }
    const list = text(renderToStaticMarkup(<TemplateTable templates={[summary]} onOpen={() => {}} onGenerate={() => {}} words={words} />))
    expect(list).toContain("Generar")
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
