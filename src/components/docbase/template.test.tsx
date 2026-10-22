import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import type { Template, TemplateField, TemplateSummary } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import {
  fieldName,
  fieldRows,
  fieldsLine,
  formatName,
  placeLabel,
  possiblySameNames,
  rowIndexOf,
  templateName,
  templateSummary,
  templatesSummary,
} from "./template"
import { TemplatePanel } from "./template-panel"
import { TemplateDetail, TemplateTable } from "./templates-view"
import { docRef } from "./similar.fixtures"

// The panel talks to the base only through its props here.
vi.mock("@/commands/docbase", () => ({}))

const words = vocabularyFor("legal", "es")

function field(kind: TemplateField["kind"], ordinal: number, values: string[], extra: Partial<TemplateField> = {}): TemplateField {
  return { kind, ordinal, formName: null, sameAs: null, values, ...extra }
}

/** Three invented receipts: a name, an amount said twice and a date change. */
function receipts(): Template {
  return {
    principal: 1,
    asked: 1,
    documents: [
      { doc: docRef(1, "Recibo enero.pdf"), state: null },
      { doc: docRef(2, "Recibo febrero.pdf"), state: "proposed" },
      { doc: docRef(3, "Recibo marzo.pdf"), state: "joined" },
    ],
    state: "proposed",
    segments: [
      { text: "Recibí de " },
      { field: 0 },
      { text: " la suma de $ " },
      { field: 1 },
      { text: " el día " },
      { field: 2 },
      { text: ". Son $ " },
      { field: 3 },
      { text: "." },
    ],
    fields: [
      field("name", 1, ["Ana Torres", "Bruno Vega", "Carla Ruiz"]),
      field("amount", 1, ["1.500,00", "2.750,00", "980,50"]),
      field("date", 1, ["10/01/2026", "", "10/03/2026"]),
      field("amount", 1, ["1.500,00", "2.750,00", "980,50"], { sameAs: 1 }),
    ],
    possiblySame: [],
    computedAt: 1,
    docType: "recibo",
    proposedTypeName: null,
    typeRank: null,
    format: "pdf-text",
  }
}

function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim()
}

describe("what the template screen says", () => {
  it("names each field by its kind and its order, from the vocabulary", () => {
    expect(fieldName(field("date", 2, []), words)).toBe("Fecha 2")
    expect(fieldName(field("amount", 1, []), words)).toBe("Importe 1")
    expect(fieldName(field("identifier", 3, []), vocabularyFor("legal", "en"))).toBe("Identifier 3")
  })

  it("lists a field said again once, with its places", () => {
    const rows = fieldRows(receipts())
    expect(rows.map((row) => [row.index, row.places])).toEqual([
      [0, 1],
      [1, 2],
      [2, 1],
    ])
    expect(rowIndexOf(receipts(), 3)).toBe(1)
  })

  it("sums the template up in one sentence", () => {
    expect(templateSummary(receipts(), words)).toBe(
      "La misma plantilla en 3 documentos; cambian 3 campos: Nombre (1), Importe (1), Fecha (1).",
    )
    const none = { ...receipts(), segments: [{ text: "Igual." }], fields: [] }
    expect(templateSummary(none, words)).toBe("La misma plantilla en 3 documentos; no cambia ningún dato.")
  })

  it("shows in a field its name, or what the chosen document says there", () => {
    expect(placeLabel(receipts(), 2, null, words)).toBe("Fecha 1")
    expect(placeLabel(receipts(), 2, 0, words)).toBe("10/01/2026")
    expect(placeLabel(receipts(), 2, 1, words)).toBe("(nada)")
  })

  it("names the two documents possibly the same", () => {
    const template = { ...receipts(), possiblySame: [[1, 3]] as [number, number][] }
    expect(possiblySameNames(template)).toEqual([["Recibo enero.pdf", "Recibo marzo.pdf"]])
  })

  it("names a template by the type of its principal, with its order when another has the same type", () => {
    expect(templateName(receipts(), "Recibo enero.pdf", words)).toBe("Recibo")
    expect(templateName({ ...receipts(), typeRank: 1 }, "Recibo enero.pdf", words)).toBe("Recibo")
    expect(templateName({ ...receipts(), typeRank: 2 }, "Recibo enero.pdf", words)).toBe("Recibo (2)")
    // A type the model proposed, by the name it proposed.
    expect(templateName({ ...receipts(), docType: "poder-general", proposedTypeName: "Poder general" }, "x.pdf", words)).toBe(
      "Poder general",
    )
    // No type yet: by its principal, with the word of the profession.
    expect(templateName({ ...receipts(), docType: null }, "Recibo enero.pdf", words)).toBe("Plantilla «Recibo enero.pdf»")
    expect(templateName({ ...receipts(), docType: null }, "a.pdf", vocabularyFor("legal", "en"))).toBe("Template «a.pdf»")
  })

  it("says where the format comes from and sums the list up", () => {
    expect(formatName("word")).toBe("Word")
    expect(formatName("scan")).toBe("Escaneo")
    expect(formatName("pdf-form")).toBe("Formulario PDF")
    const one = summary({ documents: 5 })
    expect(templatesSummary([one, summary({ documents: 2 })], words)).toBe(
      "2 plantillas con 7 documentos. Todavía no se confirmó ninguna familia.",
    )
    expect(templatesSummary([one, summary({ state: "confirmed" })], words)).toBe("2 plantillas con 8 documentos. 1 de 2 con la familia confirmada.")
    expect(templatesSummary([summary({ state: "confirmed" })], words)).toBe("1 plantilla con 3 documentos. Las familias de todas están confirmadas.")
    expect(fieldsLine({ fields: 8, wordingFields: 2 })).toBe("8 campos · 2 textos que cambian")
    expect(fieldsLine({ fields: 1, wordingFields: 0 })).toBe("1 campo")
    expect(fieldsLine({ fields: 0, wordingFields: 0 })).toBe("ningún campo cambia")
  })
})

function summary(patch: Partial<TemplateSummary> = {}): TemplateSummary {
  return {
    principal: docRef(1, "Recibo enero.pdf"),
    docType: "recibo",
    proposedTypeName: null,
    typeRank: null,
    state: "proposed",
    documents: 3,
    fields: 3,
    wordingFields: 0,
    format: "pdf-text",
    possiblySame: 0,
    ...patch,
  }
}

describe("the template panel", () => {
  it("shows the shared text with the fields in their place and a table of values", () => {
    const html = text(renderToStaticMarkup(<TemplatePanel template={receipts()} initialColumn={null} onOpen={() => {}} words={words} />))
    expect(html).toContain("Propuesta")
    expect(html).toContain("es una propuesta hasta que se confirme la familia como «Misma plantilla, otros datos»")
    expect(html).toContain("Recibí de Nombre 1 la suma de $ Importe 1 el día Fecha 1 . Son $ Importe 1 .")
    expect(html).toContain("Los campos, documento por documento (3)")
    expect(html).toContain("2 lugares del texto")
    expect(html).toContain("Bruno Vega")
    expect(html).toContain("sin confirmar")
    expect(html).toContain("confirmado")
  })

  it("shows the values of the document it was opened from", () => {
    const html = text(renderToStaticMarkup(<TemplatePanel template={receipts()} initialColumn={1} onOpen={() => {}} words={words} />))
    expect(html).toContain("Recibí de Bruno Vega la suma de $ 2.750,00 el día (nada)")
  })

  it("marks two documents where nothing changes as possibly the same, and says no field changes", () => {
    const same: Template = {
      ...receipts(),
      documents: receipts().documents.slice(0, 2),
      segments: [{ text: "Intimo a usted a pagar lo adeudado." }],
      fields: [],
      possiblySame: [[1, 2]],
    }
    const html = text(renderToStaticMarkup(<TemplatePanel template={same} initialColumn={null} onOpen={() => {}} words={words} />))
    expect(html).toContain("En «Recibo enero.pdf» y «Recibo febrero.pdf» no cambia ningún dato")
    expect(html).toContain("posiblemente sean el mismo documento")
    expect(html).toContain("Ningún campo cambia entre estos documentos.")
  })

  it("leaves its title to the section when asked", () => {
    const html = text(
      renderToStaticMarkup(<TemplatePanel template={receipts()} initialColumn={null} onOpen={() => {}} heading={false} words={words} />),
    )
    expect(html).not.toContain("Plantilla Propuesta")
    expect(html).toContain("Los campos, documento por documento (3)")
  })

  it("says a confirmed template is confirmed", () => {
    const html = text(
      renderToStaticMarkup(<TemplatePanel template={{ ...receipts(), state: "confirmed" }} initialColumn={null} onOpen={() => {}} words={words} />),
    )
    expect(html).toContain("Confirmada")
    expect(html).not.toContain("es una propuesta")
  })
})

describe("the section «Plantillas»", () => {
  it("lists each template with its name, state, documents, fields, format and notices", () => {
    const list = [
      summary({ documents: 5, fields: 8, wordingFields: 2 }),
      summary({ principal: docRef(4, "Ficha.pdf"), docType: null, format: "scan", possiblySame: 1, fields: 1 }),
    ]
    const html = text(renderToStaticMarkup(<TemplateTable templates={list} onOpen={() => {}} words={words} />))
    expect(html).toContain("2 plantillas con 8 documentos. Todavía no se confirmó ninguna familia.")
    expect(html).toContain("Recibo Propuesta")
    expect(html).toContain("5 documentos · 8 campos · 2 textos que cambian · PDF con texto")
    expect(html).toContain("Plantilla «Ficha.pdf»")
    expect(html).toContain("posiblemente el mismo documento")
    expect(html).toContain("el texto salió de un escaneo")
    expect(html).toContain("3 documentos · 1 campo · Escaneo")
  })

  it("shows the detail of a proposal with its notice, the way to confirm its family and the panel of today", () => {
    const html = text(renderToStaticMarkup(<TemplateDetail template={receipts()} onOpen={() => {}} words={words} />))
    expect(html).toContain("Recibo Propuesta")
    expect(html).toContain("3 documentos · 3 campos · formato del principal: PDF con texto")
    expect(html).toContain("Es una propuesta: nadie confirmó todavía que estos documentos son de la misma familia. Se puede ver y usar igual.")
    expect(html).toContain("Confirmar la familia")
    expect(html).toContain("Recibí de Nombre 1 la suma de $ Importe 1")
    expect(html).not.toContain("salió de un escaneo")
    const scan = text(
      renderToStaticMarkup(<TemplateDetail template={{ ...receipts(), state: "confirmed", format: "scan" }} onOpen={() => {}} words={words} />),
    )
    expect(scan).toContain("El texto salió de un escaneo: lo leyó el reconocimiento y puede tener errores.")
    expect(scan).not.toContain("Confirmar la familia")
  })
})
