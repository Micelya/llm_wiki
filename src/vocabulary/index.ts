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
  /** Label groups offered to start with, most general first. */
  suggestedGroups: string[]
  /** The language its wiki is written in when the user chose none, as an output-language value. */
  wikiLanguage: string
}

export type VerticalId = "legal"
export type VocabularyLanguage = "es" | "en"

/** Each vertical in each language it has; `default` is used for the rest. */
export interface Vertical {
  default: VocabularyLanguage
  languages: Partial<Record<VocabularyLanguage, Vocabulary>>
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
  const entry = VERTICALS[vertical] ?? VERTICALS[DEFAULT_VERTICAL]
  const short = language.toLowerCase().split("-")[0] as VocabularyLanguage
  const found = entry.languages[short] ?? entry.languages[entry.default]
  if (!found) throw new Error(`Vertical '${vertical}' has no vocabulary`)
  return found
}

let currentVertical: VerticalId = DEFAULT_VERTICAL

/** Chooses the profession the app speaks to. */
export function setVertical(vertical: VerticalId): void {
  currentVertical = vertical
}

/** The vocabulary the screens use right now. */
export function vocabulary(): Vocabulary {
  return vocabularyFor(currentVertical, SCREEN_LANGUAGE)
}
