import type { Template, TemplateField, TemplateFormat, TemplateKind, TemplateSummary } from "@/commands/docbase"
import { documentTypeName, templateFieldName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatCount, plural } from "./format"

/**
 * What the template screen says, apart from the screen so it can be
 * tested: the name of each field, the rows of the table (each field
 * once, however often the text says it), the sentence that sums it up
 * and the pairs that are possibly the same document.
 */

/** The name of a field: its kind and its order ("Fecha 2"). */
export function fieldName(field: TemplateField, words: Vocabulary = vocabulary()): string {
  return templateFieldName(field.kind, field.ordinal, words)
}

/** A row of the table: a field once, and in how many places the text says it. */
export interface FieldRow {
  /** Index of the field in `Template.fields`: its first place. */
  index: number
  field: TemplateField
  places: number
}

/** The fields once each, in the order the text says them first; those of a form last. */
export function fieldRows(template: Template): FieldRow[] {
  const rows: FieldRow[] = []
  const byIndex = new Map<number, FieldRow>()
  template.fields.forEach((field, index) => {
    const original = field.sameAs ?? index
    const found = byIndex.get(original)
    if (found) {
      found.places += 1
      return
    }
    const row = { index, field, places: 1 }
    byIndex.set(index, row)
    rows.push(row)
  })
  return rows
}

/** The field a place of the text stands for: the first place of the same field. */
export function rowIndexOf(template: Template, field: number): number {
  return template.fields[field]?.sameAs ?? field
}

/**
 * One sentence about the template: "La misma plantilla en 3 documentos;
 * cambian 8 campos: Fecha (4), Importe (3), Identificador (1)."
 */
export function templateSummary(template: Template, words: Vocabulary = vocabulary()): string {
  const rows = fieldRows(template)
  const head = `La misma plantilla en ${plural(template.documents.length, "documento", "documentos")}`
  if (rows.length === 0) return `${head}; no cambia ningún dato.`
  const byKind = new Map<string, number>()
  for (const row of rows) byKind.set(row.field.kind, (byKind.get(row.field.kind) ?? 0) + 1)
  const kinds = [...byKind.entries()]
    .sort((a, b) => b[1] - a[1])
    .map(([kind, count]) => `${words.templateFields[kind] ?? kind} (${formatCount(count)})`)
  const changes = rows.length === 1 ? "cambia un campo" : `cambian ${formatCount(rows.length)} campos`
  return `${head}; ${changes}: ${kinds.join(", ")}.`
}

/** The names of the two documents of each pair possibly the same, as the template shows them. */
export function possiblySameNames(template: Template): [string, string][] {
  const name = (id: number) => template.documents.find((d) => d.doc.contentId === id)?.doc.name ?? String(id)
  return template.possiblySame.map(([a, b]) => [name(a), name(b)])
}

/**
 * What a template is called: the type of its principal ("Recibo"), with
 * its order when another template has the same type ("Recibo (2)"); with
 * no type yet, by its principal.
 */
export function templateName(kind: TemplateKind, principalName: string, words: Vocabulary = vocabulary()): string {
  if (!kind.docType) return `${capitalized(words.templates.one)} «${principalName}»`
  const name = documentTypeName(kind.docType, kind.proposedTypeName, words)
  return kind.typeRank && kind.typeRank > 1 ? `${name} (${kind.typeRank})` : name
}

function capitalized(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1)
}

/** Where the format of a template comes from, in words. */
export function formatName(format: TemplateFormat): string {
  const names: Record<TemplateFormat, string> = {
    word: "Word",
    "pdf-text": "PDF con texto",
    "pdf-form": "Formulario PDF",
    scan: "Escaneo",
    text: "Texto",
  }
  return names[format] ?? format
}

/** The line over the list: "12 plantillas con 49 documentos. Todavía no se confirmó ninguna familia." */
export function templatesSummary(templates: TemplateSummary[], words: Vocabulary = vocabulary()): string {
  const documents = templates.reduce((sum, t) => sum + t.documents, 0)
  const confirmed = templates.filter((t) => t.state === "confirmed").length
  const head = `${plural(templates.length, words.templates.one, words.templates.many)} con ${plural(documents, "documento", "documentos")}.`
  if (confirmed === templates.length) return `${head} Las familias de todas están confirmadas.`
  if (confirmed === 0) return `${head} Todavía no se confirmó ninguna familia.`
  return `${head} ${formatCount(confirmed)} de ${formatCount(templates.length)} con la familia confirmada.`
}

/** The fields of a template in the list: "8 campos · 2 textos que cambian". */
export function fieldsLine(template: Pick<TemplateSummary, "fields" | "wordingFields">): string {
  if (template.fields === 0) return "ningún campo cambia"
  const head = plural(template.fields, "campo", "campos")
  if (template.wordingFields === 0) return head
  return `${head} · ${plural(template.wordingFields, "texto que cambia", "textos que cambian")}`
}

/**
 * What a place of the text shows: the field's name, or the value of the
 * document chosen (empty: "(nada)").
 */
export function placeLabel(template: Template, field: number, column: number | null, words: Vocabulary = vocabulary()): string {
  const item = template.fields[field]
  if (!item) return ""
  if (column === null) return fieldName(item, words)
  return item.values[column] || "(nada)"
}
