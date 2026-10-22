import type { GenerateRequest, GeneratedValueSource, Template, TemplateField } from "@/commands/docbase"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { distinctValues, fieldName, fieldRows, rowIndexOf } from "./template"

/**
 * Generating a document from a template (D-488), apart from the screen so
 * it can be tested: the fields to fill in the order of the text, each once,
 * how each one is filled, starting from the data of one of the template's
 * documents, the preview of the text with the values in place, the name
 * of the file and what is sent to the base.
 */

/** The value given to a field, and where it came from. */
export interface FieldValue {
  value: string
  source: GeneratedValueSource
  /** What it came from: `entity:<id>`, `matter:<id>`, `content:<id>`. */
  reference: string | null
}

/** The values of the fields, by the index of each field's first place. */
export type FieldValues = Record<number, FieldValue>

/**
 * How a field is filled, by what it is: a person or organization of the
 * base (`entity`), a matter (`matter`), a date (`date`), a wording that
 * changes (`wording`: one of the versions of the documents, none or
 * another), written (`typed`: an amount, a number, an identifier) or not
 * asked (`fixed`).
 */
export type FieldInput = "entity" | "matter" | "date" | "typed" | "wording" | "fixed"

/** A field to fill, once however often the text says it. */
export interface GenerationRow {
  /** Index of the field's first place in `Template.fields`. */
  index: number
  field: TemplateField
  name: string
  input: FieldInput
  /** For `entity`: the type of entity ("persona"); `null`: any. */
  entityKind: string | null
  /** A value of the documents, to show the form it has. */
  example: string | null
  /** The versions the documents have of it, each once. */
  versions: string[]
  /** It may stay empty: it is a wording that changes, or some document leaves it empty. */
  optional: boolean
  /** In how many places the text says it. */
  places: number
}

/** What a field can be that is a kind of data, not an entity. */
const DATA_WHATS = new Set(["fecha", "importe", "numero", "identificador", "texto"])

/** The fields to fill, in the order of the text; those of a form last. */
export function generationRows(template: Template, words: Vocabulary = vocabulary()): GenerationRow[] {
  const references = words.knowledge.referenceTypes
  return fieldRows(template).map(({ index, field, places }) => {
    const what = field.what
    let input: FieldInput = "typed"
    let entityKind: string | null = null
    if (field.fixed) input = "fixed"
    else if (what && references.includes(what)) input = "matter"
    else if (what && !DATA_WHATS.has(what)) {
      input = "entity"
      entityKind = what
    } else if (what === "fecha" || (!what && field.kind === "date")) input = "date"
    else if (what === "texto" || (!what && field.kind === "wording")) input = "wording"
    else if (!what && field.kind === "name") input = "entity"
    const versions = distinctValues(field)
    return {
      index,
      field,
      name: fieldName(field, words),
      input,
      entityKind,
      example: versions[0] ?? null,
      versions,
      optional: field.kind === "wording" || field.values.some((value) => value.trim() === ""),
      places,
    }
  })
}

/** What a field gets: its fixed value, or the one given. */
export function valueOf(row: GenerationRow, values: FieldValues): string {
  if (row.input === "fixed") return row.field.fixedValue ?? ""
  return values[row.index]?.value ?? ""
}

/** The values a document starts with: nothing given yet. */
export function initialValues(): FieldValues {
  return {}
}

/**
 * The values of one of the template's documents (`column`, in the order of
 * `Template.documents`), to change only what changes: a renewal, say.
 */
export function valuesFromDocument(template: Template, rows: GenerationRow[], column: number): FieldValues {
  const document = template.documents[column]
  const values: FieldValues = {}
  if (!document) return values
  for (const row of rows) {
    if (row.input === "fixed") continue
    values[row.index] = { value: row.field.values[column] ?? "", source: "document", reference: `content:${document.doc.contentId}` }
  }
  return values
}

/** The names of the fields that need a value and have none: the Word shows them between brackets. */
export function missingNames(rows: GenerationRow[], values: FieldValues): string[] {
  return rows.filter((row) => !row.optional && valueOf(row, values).trim() === "").map((row) => row.name)
}

/** A piece of the preview: text of the template, a value, a needed field with none, an optional one left out. */
export type PreviewPiece =
  | { kind: "text"; text: string }
  | { kind: "value"; row: number; text: string }
  | { kind: "missing"; row: number; text: string }
  | { kind: "empty"; row: number }

/** The preview: the text with each value in its place, and the fields of a form, which have no place in it. */
export interface Preview {
  text: PreviewPiece[]
  form: { row: GenerationRow; piece: PreviewPiece }[]
}

function pieceOf(row: GenerationRow, values: FieldValues): PreviewPiece {
  const value = valueOf(row, values).trim()
  if (value) return { kind: "value", row: row.index, text: value }
  if (row.optional) return { kind: "empty", row: row.index }
  return { kind: "missing", row: row.index, text: `[${row.name}]` }
}

/** What the Word will say, as the base writes it (`generate.rs`). */
export function preview(template: Template, rows: GenerationRow[], values: FieldValues): Preview {
  const byIndex = new Map(rows.map((row) => [row.index, row]))
  const inText = new Set<number>()
  const text: PreviewPiece[] = template.segments.map((segment) => {
    if ("text" in segment) return { kind: "text", text: segment.text }
    const index = rowIndexOf(template, segment.field)
    inText.add(index)
    const row = byIndex.get(index)
    return row ? pieceOf(row, values) : { kind: "text", text: "" }
  })
  const form = rows.filter((row) => !inText.has(row.index)).map((row) => ({ row, piece: pieceOf(row, values) }))
  return { text, form }
}

/** Today, as the calendar writes it: "2026-10-22". */
export function todayIso(now: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0")
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

/**
 * The name the file gets unless a person writes another: the template's,
 * its parties (the people or organizations given, up to two) and the date:
 * "Contrato de locación - Ana Torres y Bruno Vega - 2026-10-22".
 */
export function suggestedFileName(templateName: string, rows: GenerationRow[], values: FieldValues, today: string): string {
  const parties: string[] = []
  for (const row of rows) {
    if (row.input !== "entity") continue
    const value = valueOf(row, values).trim()
    if (value && !parties.includes(value)) parties.push(value)
  }
  const shown = parties.slice(0, 2).join(" y ")
  return [templateName.trim(), shown, today].filter(Boolean).join(" - ")
}

/** What is sent to the base: every field once, with its name, its value and where it came from. */
export function generationRequest(
  template: Template,
  templateName: string,
  rows: GenerationRow[],
  values: FieldValues,
  options: { matterId: string | null; fileName: string | null; fromDocument: number | null },
): GenerateRequest {
  return {
    principal: template.principal,
    templateName,
    values: rows.map((row) => {
      const given = values[row.index]
      return {
        position: row.index,
        name: row.name,
        value: valueOf(row, values),
        source: row.input === "fixed" ? "fixed" : (given?.source ?? "hand"),
        reference: row.input === "fixed" ? null : (given?.reference ?? null),
      }
    }),
    matterId: options.matterId,
    fileName: options.fileName?.trim() || null,
    fromDocument: options.fromDocument,
  }
}
