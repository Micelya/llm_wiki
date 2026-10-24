import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import type { Template, TemplateField, TemplateSummary } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import {
  automaticTemplateName,
  choiceOf,
  decided,
  distinctValues,
  fieldName,
  fieldNote,
  fieldRows,
  fieldsLine,
  formatName,
  madeWithLine,
  nameSourceText,
  placeLabel,
  possiblySameNames,
  rowIndexOf,
  templateName,
  templateSummary,
  templatesSummary,
  whatName,
} from "./template"
import { FieldEditor } from "./template-field-editor"
import { TemplatePanel } from "./template-panel"
import { TemplateDetail, TemplateTable } from "./templates-view"
import { docRef } from "./similar.fixtures"

// The panel talks to the base only through its props here.
vi.mock("@/commands/docbase", () => ({}))

const words = vocabularyFor("legal", "es")

function field(kind: TemplateField["kind"], ordinal: number, values: string[], extra: Partial<TemplateField> = {}): TemplateField {
  return {
    kind,
    ordinal,
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

const WHATS = ["persona", "organizacion", "asunto", "fecha", "importe", "numero", "identificador", "texto"]

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
    name: null,
    whats: WHATS,
    generated: [],
  }
}

/** The receipts, with what a person and the model said of their fields. */
function named(): Template {
  const template = receipts()
  template.name = "Recibo de honorarios"
  template.fields = [
    field("name", 1, ["Ana Torres", "Bruno Vega", "Carla Ruiz"], { name: "Quien paga", nameSource: "person", what: "persona", whatSource: "model" }),
    field("amount", 1, ["1.500,00", "2.750,00", "980,50"], { name: "Importe", nameSource: "model", what: "importe", whatSource: "kind" }),
    field("date", 1, ["10/01/2026", "", "10/03/2026"], { what: "fecha", whatSource: "kind", fixed: true, fixedValue: "10/01/2026" }),
    field("amount", 1, ["1.500,00", "2.750,00", "980,50"], { sameAs: 1, name: "Importe", nameSource: "model" }),
  ]
  return template
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

  it("says which template a document was made with, and with how many others (D-497)", () => {
    expect(madeWithLine(receipts(), "Recibo enero.pdf", 2, words)).toBe("Hecho con la plantilla «Recibo», como 2 documentos más.")
    expect(madeWithLine({ ...receipts(), name: "Recibo de honorarios" }, "Recibo enero.pdf", 1, words)).toBe(
      "Hecho con la plantilla «Recibo de honorarios», como 1 documento más.",
    )
    expect(madeWithLine({ ...receipts(), docType: null }, "Recibo enero.pdf", 0, words)).toBe(
      "Hecho con la plantilla de «Recibo enero.pdf».",
    )
  })

  it("says where the format comes from and sums the list up", () => {
    expect(formatName("word")).toBe("Word")
    expect(formatName("scan")).toBe("Escaneo")
    expect(formatName("pdf-form")).toBe("Formulario PDF")
    const one = summary({ documents: 5 })
    // Nothing to confirm: the documents of a template are different documents (D-497).
    expect(templatesSummary([one, summary({ documents: 2 })], words)).toBe("2 plantillas con 7 documentos.")
    expect(templatesSummary([summary({ state: "confirmed" })], words)).toBe("1 plantilla con 3 documentos.")
    expect(fieldsLine({ fields: 8, wordingFields: 2 })).toBe("8 campos · 2 textos que cambian")
    expect(fieldsLine({ fields: 1, wordingFields: 0 })).toBe("1 campo")
    expect(fieldsLine({ fields: 0, wordingFields: 0 })).toBe("ningún campo cambia")
  })
})

describe("the names of a template and of its fields", () => {
  it("shows the name a person, the text or the model gave a field, and says where it comes from", () => {
    const [who, amount, date] = named().fields
    expect(fieldName(who, words)).toBe("Quien paga")
    expect(nameSourceText(who)).toBe("nombre puesto por una persona")
    expect(nameSourceText(amount)).toBe("nombre puesto por el modelo")
    expect(nameSourceText({ nameSource: "label", formName: null })).toBe("rótulo del texto")
    expect(nameSourceText({ nameSource: "label", formName: "Nombre" })).toBe("nombre del formulario")
    expect(fieldName(date, words)).toBe("Fecha 1")
    expect(nameSourceText(date)).toBeNull()
    expect(placeLabel(named(), 3, null, words)).toBe("Importe")
  })

  it("says what a field is with the words of the profession, and whether it is fixed", () => {
    expect(whatName("asunto", words)).toBe("Caso")
    expect(whatName("persona", words)).toBe("Persona")
    expect(whatName("organizacion", words)).toBe("Organización")
    expect(whatName("fecha", words)).toBe("Fecha")
    expect(whatName("identificador", words)).toBe("Identificador")
    expect(whatName("texto", words)).toBe("Texto")
    const [who, , date] = named().fields
    expect(fieldNote(who, words)).toBe("Persona")
    expect(fieldNote(date, words)).toBe("Fecha · fijo: «10/01/2026»")
    expect(fieldNote(field("name", 1, []), words)).toBeNull()
  })

  it("starts the editor with what a person decided, and only that", () => {
    const [who, amount, date] = named().fields
    expect(choiceOf(who)).toEqual({ name: "Quien paga", what: null, fixed: false, fixedValue: null })
    expect(choiceOf(amount)).toEqual({ name: null, what: null, fixed: false, fixedValue: null })
    expect(choiceOf(date)).toEqual({ name: null, what: null, fixed: true, fixedValue: "10/01/2026" })
    expect([who, amount, date].map(decided)).toEqual([true, false, true])
    expect(distinctValues(field("name", 1, ["Ana", "", "Ana", "Bruno"]))).toEqual(["Ana", "Bruno"])
  })

  it("names a template by what a person wrote before its type", () => {
    expect(templateName(named(), "Recibo enero.pdf", words)).toBe("Recibo de honorarios")
    expect(automaticTemplateName(named(), "Recibo enero.pdf", words)).toBe("Recibo")
  })

  it("lets a person edit each field in the section, and nowhere else", () => {
    const editable = text(
      renderToStaticMarkup(
        <TemplatePanel template={named()} initialColumn={null} onOpen={() => {}} onChooseField={() => true} words={words} />,
      ),
    )
    expect(editable).toContain("Quien paga nombre puesto por una persona Persona")
    expect(editable).toContain("Importe nombre puesto por el modelo Importe 2 lugares del texto")
    expect(editable).toContain("Fecha 1 sin nombre todavía: lo nombra el modelo Fecha · fijo: «10/01/2026»")
    expect(editable).toContain("Recibí de Quien paga la suma de $ Importe el día Fecha 1")
    const html = renderToStaticMarkup(<TemplatePanel template={named()} initialColumn={null} onOpen={() => {}} words={words} />)
    expect(html).not.toContain("Cambiar el nombre, qué es o si es fijo")
    expect(
      renderToStaticMarkup(<TemplatePanel template={named()} initialColumn={null} onOpen={() => {}} onChooseField={() => true} words={words} />),
    ).toContain("Cambiar el nombre, qué es o si es fijo")
  })

  it("offers in the editor the automatic name and what it is, the values to fix it with and the way back", () => {
    const [who, amount] = named().fields
    const html = text(
      renderToStaticMarkup(<FieldEditor field={amount} whats={WHATS} busy={false} onSave={() => {}} onCancel={() => {}} words={words} />),
    )
    expect(html).toContain("Automático: Importe")
    expect(html).toContain("Caso")
    expect(html).toContain("Fijo: no se pide al generar un documento y se escribe siempre igual")
    expect(html).not.toContain("Volver a lo automático")
    const decidedHtml = renderToStaticMarkup(
      <FieldEditor field={who} whats={WHATS} busy={false} onSave={() => {}} onCancel={() => {}} words={words} />,
    )
    expect(decidedHtml).toContain('value="Quien paga"')
    expect(decidedHtml).toContain('placeholder="Nombre 1"')
    expect(text(decidedHtml)).toContain("Volver a lo automático")
    const fixed = text(
      renderToStaticMarkup(
        <FieldEditor field={named().fields[2]} whats={WHATS} busy={false} onSave={() => {}} onCancel={() => {}} words={words} />,
      ),
    )
    expect(fixed).toContain("10/01/2026 10/03/2026 (nada) Otro…")
  })

  it("shows the name a person gave the template, with the way back to the automatic one", () => {
    const html = text(renderToStaticMarkup(<TemplateDetail template={named()} onOpen={() => {}} onRename={async () => true} words={words} />))
    expect(html).toContain("Recibo de honorarios")
    expect(html).toContain("Volver al nombre automático")
    expect(text(renderToStaticMarkup(<TemplateDetail template={receipts()} onOpen={() => {}} onRename={async () => true} words={words} />))).not.toContain(
      "Volver al nombre automático",
    )
  })
})

function summary(patch: Partial<TemplateSummary> = {}): TemplateSummary {
  return {
    principal: docRef(1, "Recibo enero.pdf"),
    name: null,
    docType: "recibo",
    proposedTypeName: null,
    typeRank: null,
    state: "proposed",
    documents: 3,
    fields: 3,
    wordingFields: 0,
    format: "pdf-text",
    possiblySame: 0,
    waitingFields: 0,
    generated: 0,
    ...patch,
  }
}

describe("the template panel", () => {
  it("shows the shared text with the fields in their place and a table of values", () => {
    const html = text(renderToStaticMarkup(<TemplatePanel template={receipts()} initialColumn={null} onOpen={() => {}} words={words} />))
    // Made with one template: different documents, nothing to decide (D-497).
    expect(html).not.toContain("Propuesta")
    expect(html).not.toContain("es una propuesta")
    expect(html).toContain("Los documentos hechos con ella son documentos distintos: no hay nada que decidir.")
    expect(html).toContain("Recibí de Nombre 1 la suma de $ Importe 1 el día Fecha 1 . Son $ Importe 1 .")
    expect(html).toContain("Los campos, documento por documento (3)")
    expect(html).toContain("2 lugares del texto")
    expect(html).toContain("Bruno Vega")
    expect(html).not.toContain("sin confirmar")
    expect(html).toContain("hecho con ella")
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
    expect(html).toContain("2 plantillas con 8 documentos.")
    expect(html).not.toContain("Propuesta")
    expect(html).toContain("5 documentos · 8 campos · 2 textos que cambian · PDF con texto")
    expect(html).toContain("Plantilla «Ficha.pdf»")
    expect(html).toContain("posiblemente el mismo documento")
    expect(html).toContain("el texto salió de un escaneo")
    expect(html).toContain("3 documentos · 1 campo · Escaneo")
    expect(html).not.toContain("esperan su nombre")
  })

  it("says which fields wait for the model, which names them by itself", () => {
    const list = [summary({ waitingFields: 2 }), summary({ principal: docRef(4, "Otro.pdf"), waitingFields: 1 })]
    const html = text(renderToStaticMarkup(<TemplateTable templates={list} onOpen={() => {}} words={words} />))
    expect(html).toContain(
      "3 campos esperan su nombre: el texto no los nombra y los nombra el modelo, solo, mientras la lectura automática está activa. Lo que escribas vos nunca se pisa.",
    )
    expect(html).toContain("3 documentos · 3 campos · PDF con texto · 2 sin nombre todavía")
    expect(html).not.toContain("Nombrar los campos con el modelo")
    const panel = text(
      renderToStaticMarkup(<TemplatePanel template={receipts()} initialColumn={null} onOpen={() => {}} onChooseField={() => true} words={words} />),
    )
    expect(panel).toContain("Nombre 1 sin nombre todavía: lo nombra el modelo")
    expect(panel).toContain("el modelo solo les pone nombre a los campos que el texto no nombra")
  })

  it("shows the detail of a template with the panel of today, with no family to confirm", () => {
    const html = text(renderToStaticMarkup(<TemplateDetail template={receipts()} onOpen={() => {}} words={words} />))
    expect(html).not.toContain("Propuesta")
    expect(html).toContain("3 documentos · 3 campos · formato del principal: PDF con texto")
    expect(html).not.toContain("Es una propuesta")
    expect(html).not.toContain("Confirmar la familia")
    expect(html).toContain("Recibí de Nombre 1 la suma de $ Importe 1")
    expect(html).not.toContain("salió de un escaneo")
    const scan = text(
      renderToStaticMarkup(<TemplateDetail template={{ ...receipts(), state: "confirmed", format: "scan" }} onOpen={() => {}} words={words} />),
    )
    expect(scan).toContain("El texto salió de un escaneo: lo leyó el reconocimiento y puede tener errores.")
    expect(scan).not.toContain("Confirmar la familia")
  })
})
