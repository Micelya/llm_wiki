/**
 * The words of the user's profession.
 *
 * What a law firm calls its "archivo", a notary's office or an
 * accounting firm may call something else. Those words are not written
 * in the screens: each vertical has a vocabulary, and each vocabulary
 * exists once per language. Generic interface words ("Buscar",
 * "Cancelar") are ordinary translation and do not belong here; technical
 * names (folders on disk, identifiers) are neutral and never change with
 * the vertical.
 */
import { legal } from "./legal"

export interface Vocabulary {
  /** The general project of a space, as the user calls it. */
  archive: {
    /** The bare noun: "Archivo". */
    name: string
    /** The one of the private space: "Mi archivo". */
    mine: string
    /** The one of the shared space: "Archivo del estudio". */
    shared: string
    /** Title of the figures about its state: "Salud del archivo". */
    health: string
    /** Its wiki: "Wiki del archivo". */
    wiki: string
    /** One sentence saying what its wiki is. */
    wikiDescription: string
  }
  /**
   * The single view of what needs a person (problems, decisions, what is
   * in progress), as the screens name it: "Atención". A notary's office
   * may prefer "Pendientes" or "Bandeja".
   */
  attention: {
    /** Its name in the navigation and as its title. */
    name: string
    /** One sentence under the title. */
    description: string
  }
  /** Label groups offered to start with, most general first. */
  suggestedGroups: string[]
  /** The language its wiki is written in when the user chose none, as an output-language value. */
  wikiLanguage: string
  /**
   * The names of the relations between similar documents, by the neutral
   * id the catalog gives each one (`signed-copy`, `version`…). A relation
   * the catalog adds later and the vocabulary does not name yet is shown
   * by its id until someone names it here.
   */
  relations: Record<string, RelationTerms>
  /**
   * The names of the document types, by the neutral id the catalog gives
   * each one (`carta-documento`, `factura`…). A type the catalog adds
   * later is shown by its id until someone names it here; a type the
   * model proposed is shown by the name it proposed.
   */
  documentTypes: Record<string, string>
  /** The names of the stages a document can be in (`vista-previa`, `firmado`…), by id. */
  stages: Record<string, string>
  /** The names of the data of each type (`numero-de-envio`, `total`…), by id. */
  dataFields: Record<string, string>
}

export interface RelationTerms {
  /** Its name in lists and choices: "Ejemplar firmado". */
  name: string
  /**
   * For a relation with a direction, what the subject is of the other,
   * to be read between the two names: "es el ejemplar firmado de".
   */
  of?: string
}

/** Any part of a vocabulary: what a country changes of its language's one. */
export type VocabularyPatch = {
  [K in keyof Vocabulary]?: Vocabulary[K] extends string | string[] ? Vocabulary[K] : Partial<Vocabulary[K]>
}

export type VerticalId = "legal"
export type VocabularyLanguage = "es" | "en"

/** Each vertical in each language it has; `default` is used for the rest. */
export interface Vertical {
  default: VocabularyLanguage
  languages: Partial<Record<VocabularyLanguage, Vocabulary>>
  /**
   * What a country changes of its language's vocabulary, by language
   * tag ("es-MX"). Only the terms that differ go here; the rest come
   * from the language.
   */
  countries?: Record<string, VocabularyPatch>
}

const VERTICALS: Record<VerticalId, Vertical> = { legal }

export const DEFAULT_VERTICAL: VerticalId = "legal"

/**
 * The language the document base screens are written in today. They are
 * Spanish-only until the whole app is translated; from then on this is
 * the app's language.
 */
export const SCREEN_LANGUAGE: VocabularyLanguage = "es"

export function vocabularyFor(vertical: VerticalId, language: string): Vocabulary {
  return resolveVocabulary(VERTICALS[vertical] ?? VERTICALS[DEFAULT_VERTICAL], language)
}

/**
 * The vocabulary of a vertical for a language tag: the language's own
 * (or the vertical's default language), with what the country of the
 * tag changes, when the vertical says something for that country.
 */
export function resolveVocabulary(entry: Vertical, language: string): Vocabulary {
  const tag = language.toLowerCase()
  const short = tag.split("-")[0] as VocabularyLanguage
  const own = entry.languages[short]
  const found = own ?? entry.languages[entry.default]
  if (!found) throw new Error("The vertical has no vocabulary")
  // A country's terms only apply over its own language.
  const patch = own
    ? Object.entries(entry.countries ?? {}).find(([key]) => key.toLowerCase() === tag)?.[1]
    : undefined
  if (!patch) return found
  const merged: Record<string, unknown> = { ...found }
  for (const [key, value] of Object.entries(patch)) {
    const base = merged[key]
    merged[key] =
      value && typeof value === "object" && !Array.isArray(value) && base && typeof base === "object"
        ? { ...(base as object), ...value }
        : value
  }
  return merged as unknown as Vocabulary
}

let currentVertical: VerticalId = DEFAULT_VERTICAL
let currentCountry: string | null = null

/**
 * Chooses the profession the app speaks to and, when known, the
 * country ("AR", "MX"), whose own terms replace the language's.
 */
export function setVertical(vertical: VerticalId, country: string | null = null): void {
  currentVertical = vertical
  currentCountry = country
}

/** The vocabulary the screens use right now. */
export function vocabulary(): Vocabulary {
  const tag = currentCountry ? `${SCREEN_LANGUAGE}-${currentCountry}` : SCREEN_LANGUAGE
  return vocabularyFor(currentVertical, tag)
}

/** The name of a relation between similar documents, or its id when no vocabulary names it. */
export function relationName(id: string, words: Vocabulary = vocabulary()): string {
  return words.relations[id]?.name ?? id
}

/** An id shown as words when no vocabulary names it: "numero-de-envio" → "Numero de envio". */
function idInWords(id: string): string {
  const words = id.replace(/-/g, " ").trim()
  return words.charAt(0).toUpperCase() + words.slice(1)
}

/**
 * The name of a document type. A type the model proposed is shown by
 * the name it proposed; one nobody named yet, by its id in words.
 */
export function documentTypeName(id: string, proposedName: string | null = null, words: Vocabulary = vocabulary()): string {
  return words.documentTypes[id] ?? proposedName ?? idInWords(id)
}

/** The name of a stage of a document. */
export function stageName(id: string, words: Vocabulary = vocabulary()): string {
  return words.stages[id] ?? idInWords(id)
}

/** The name of a datum of a document type. */
export function dataFieldName(id: string, words: Vocabulary = vocabulary()): string {
  return words.dataFields[id] ?? idInWords(id)
}
