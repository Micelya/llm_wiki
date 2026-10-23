import type {
  EntityDetail,
  EntitySummary,
  GenerateRequest,
  GeneratedValueSource,
  MatterParty,
  MatterSummary,
  Template,
  TemplateField,
} from "@/commands/docbase"
import { identifierName, predicateName, vocabulary, type Vocabulary } from "@/vocabulary"
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

/**
 * The values a document starts with: each date, today, written as the
 * documents of the template write their dates; nothing else.
 */
export function initialValues(rows: GenerationRow[] = [], today: string = todayIso()): FieldValues {
  const values: FieldValues = {}
  for (const row of rows) {
    if (row.input !== "date") continue
    values[row.index] = { value: formatDateIn(today, dateStyleOf(row.field.values)), source: "date", reference: null }
  }
  return values
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

// ── Dates ───────────────────────────────────────────────────────────────

/** How the documents of a template write a date: "10/02/2026", "10-2-26" or "10 de febrero de 2026". */
export type DateStyle = { kind: "numeric"; separator: string; padded: boolean; fullYear: boolean } | { kind: "long" }

const MONTHS = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"]
const NUMERIC_DATE = /^(\d{1,2})([/.-])(\d{1,2})\2(\d{4}|\d{2})$/
const LONG_DATE = /^(\d{1,2})\s+de\s+([a-záéíóú]+)\s+(?:de|del)\s+(\d{4})$/i

/** The way most values of a field write their dates; "10/02/2026" when none says. */
export function dateStyleOf(values: string[]): DateStyle {
  let long = 0
  const numeric: { separator: string; padded: boolean; fullYear: boolean }[] = []
  for (const raw of values) {
    const value = raw.trim()
    const n = NUMERIC_DATE.exec(value)
    const l = LONG_DATE.exec(value)
    if (n) numeric.push({ separator: n[2], padded: n[1].length === 2 && n[3].length === 2, fullYear: n[4].length === 4 })
    else if (l && MONTHS.includes(l[2].toLowerCase())) long++
  }
  if (long > numeric.length) return { kind: "long" }
  if (numeric.length === 0) return { kind: "numeric", separator: "/", padded: true, fullYear: true }
  const first = numeric[0]
  return { kind: "numeric", separator: first.separator, padded: numeric.every((n) => n.padded), fullYear: first.fullYear }
}

/** A date of the calendar ("2026-10-22") written in a style. */
export function formatDateIn(iso: string, style: DateStyle): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso)
  if (!match) return iso
  const [year, month, day] = [Number(match[1]), Number(match[2]), Number(match[3])]
  if (style.kind === "long") return `${day} de ${MONTHS[month - 1]} de ${year}`
  const pad = (n: number) => (style.padded ? String(n).padStart(2, "0") : String(n))
  const shownYear = style.fullYear ? String(year) : String(year % 100).padStart(2, "0")
  return [pad(day), pad(month), shownYear].join(style.separator)
}

/** The date of the calendar a value says, if it says one ("10/02/2026" → "2026-02-10"). */
export function isoFromDate(value: string): string | null {
  const text = value.trim()
  const pad = (n: number) => String(n).padStart(2, "0")
  const n = NUMERIC_DATE.exec(text)
  if (n) {
    const year = n[4].length === 2 ? 2000 + Number(n[4]) : Number(n[4])
    const [day, month] = [Number(n[1]), Number(n[3])]
    return day >= 1 && day <= 31 && month >= 1 && month <= 12 ? `${year}-${pad(month)}-${pad(day)}` : null
  }
  const long = LONG_DATE.exec(text)
  if (long) {
    const month = MONTHS.indexOf(long[2].toLowerCase()) + 1
    return month > 0 ? `${long[3]}-${pad(month)}-${pad(Number(long[1]))}` : null
  }
  return null
}

// ── People, organizations and matters of the base ───────────────────────

/** What a field says of the person or organization before it in the text: its DNI, its CUIT, its address. */
export type CardKind = "dni" | "cuit" | "domicilio"

function plainWords(text: string): string[] {
  return text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter((word) => word.length >= 3)
}

/**
 * Whether a field is the DNI, the CUIT or the address of someone: by its
 * name, with the words of the vocabulary ("DNI", "CUIT o CUIL",
 * "Domicilio"), or by the form of its values (a CUIT: "20-12345678-3").
 */
export function cardKindOf(row: GenerationRow, words: Vocabulary = vocabulary()): CardKind | null {
  if (row.input !== "typed" && row.input !== "wording") return null
  const name = plainWords(row.name)
  const kinds: [CardKind, string][] = [
    ["dni", identifierName("dni", words)],
    ["cuit", identifierName("cuit", words)],
    ["domicilio", predicateName("domicilio", false, words)],
  ]
  for (const [kind, said] of kinds) {
    const own = plainWords(said).filter((word) => word !== "numero")
    if (own.some((word) => name.includes(word))) return kind
  }
  if (row.field.kind === "identifier" && row.versions.length > 0 && row.versions.every((v) => /^\d{2}-?\d{8}-?\d$/.test(v.trim()))) {
    return "cuit"
  }
  return null
}

/** A field that says something of a person or organization, and what. */
export interface RelatedRow {
  row: GenerationRow
  card: CardKind
}

/**
 * The fields of the person or organization of `owner`: its DNI, its CUIT
 * or its address, after it in the text and before the next person or
 * organization ("Ana Torres, DNI [campo], con domicilio en [campo]").
 */
export function relatedRows(rows: GenerationRow[], owner: GenerationRow, words: Vocabulary = vocabulary()): RelatedRow[] {
  const start = rows.findIndex((row) => row.index === owner.index)
  if (start < 0) return []
  const out: RelatedRow[] = []
  for (const row of rows.slice(start + 1)) {
    if (row.input === "entity") break
    if (row.field.formName !== null) continue
    const card = cardKindOf(row, words)
    if (card && !out.some((r) => r.card === card)) out.push({ row, card })
  }
  return out
}

/** What the base knows of a person or organization for each of those fields: its identifiers, its address. */
export function suggestionsFrom(entity: EntityDetail, related: RelatedRow[]): FieldValues {
  const values: FieldValues = {}
  const reference = `entity:${entity.id}`
  for (const { row, card } of related) {
    let value: string | undefined
    if (card === "domicilio") {
      const claims = entity.claims.filter((c) => c.predicate === "domicilio" && c.state !== "retired" && c.objectValue)
      const best = claims.find((c) => c.state === "supported") ?? claims[0]
      value = best?.objectValue ?? undefined
    } else {
      const kinds = card === "cuit" ? ["cuit", "cuil"] : ["dni"]
      value = [...entity.identifiers].sort((a, b) => b.documents - a.documents).find((i) => kinds.includes(i.kind))?.value
    }
    if (value) values[row.index] = { value, source: "card", reference }
  }
  return values
}

/** Puts the suggestions where nothing was given yet, or where an earlier suggestion was: never over what a person wrote. */
export function applySuggestions(values: FieldValues, suggestions: FieldValues): FieldValues {
  const out = { ...values }
  for (const [index, suggested] of Object.entries(suggestions)) {
    const current = out[Number(index)]
    if (!current || current.value.trim() === "" || current.source === "card") out[Number(index)] = suggested
  }
  return out
}

/** Someone to choose for a field: from the matter chosen, or from the search. */
export interface EntityOption {
  id: string
  kind: string
  name: string
  /** One of the matter's people or organizations. */
  ofMatter: boolean
}

/**
 * The people and organizations to offer for a field: those of the matter
 * first (whose name has the words typed), then those the search found;
 * only of the field's type, when it says one.
 */
export function entityOptions(kind: string | null, typed: string, parties: MatterParty[], found: EntitySummary[], limit = 10): EntityOption[] {
  const wanted = plainWords(typed)
  const matches = (name: string) => {
    const own = plainWords(name)
    return wanted.every((word) => own.some((w) => w.startsWith(word)))
  }
  const out: EntityOption[] = []
  for (const party of parties) {
    if ((kind && party.kind !== kind) || !matches(party.name) || out.some((o) => o.id === party.entityId)) continue
    out.push({ id: party.entityId, kind: party.kind, name: party.name, ofMatter: true })
  }
  for (const entity of found) {
    if ((kind && entity.kind !== kind) || out.some((o) => o.id === entity.id)) continue
    out.push({ id: entity.id, kind: entity.kind, name: entity.name, ofMatter: false })
  }
  return out.slice(0, limit)
}

/**
 * What a matter offers for a field that is a matter: its name, and the
 * number of its record when its name ends with it ("Torres c/ Vega
 * (CIV 1234/2026)").
 */
export function matterValues(matter: Pick<MatterSummary, "name">): { label: string; value: string }[] {
  const out = [{ label: "su nombre", value: matter.name }]
  const number = /\(([^()]*\d[^()]*)\)\s*$/.exec(matter.name)
  if (number) out.push({ label: "su número de expediente", value: number[1].trim() })
  return out
}

/** The fields that are a matter take the matter the document is generated for, where nothing was given. */
export function withMatter(values: FieldValues, rows: GenerationRow[], matter: Pick<MatterSummary, "id" | "name"> | null): FieldValues {
  const out = { ...values }
  for (const row of rows) {
    if (row.input !== "matter") continue
    const current = out[row.index]
    if (matter && (!current || current.value.trim() === "" || current.source === "matter")) {
      out[row.index] = { value: matter.name, source: "matter", reference: `matter:${matter.id}` }
    } else if (!matter && current?.source === "matter") {
      delete out[row.index]
    }
  }
  return out
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
