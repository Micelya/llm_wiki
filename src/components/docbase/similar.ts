import type {
  RelationDef,
  RelationId,
  SimilarDocument,
  SimilarGroup,
  SimilarPending,
} from "@/commands/docbase"
import { relationName, vocabulary, type Vocabulary } from "@/vocabulary"
import type { AttentionItem } from "./attention"
import { formatCount, plural } from "./format"

/**
 * What the screens say about similar documents, apart from the screens
 * themselves so it can be tested: the measures in plain words, the
 * relation in a sentence, the relations a person can choose, the source
 * the app suggests and the lines of Inicio.
 */

/** "92 %": shares are cut, never rounded up, so 99.6 % is not "100 %". */
export function percent(share: number): string {
  return `${Math.floor(Math.max(0, Math.min(1, share)) * 100)} %`
}

/** The relation the pair stands for now: the chosen one once joined, the proposed one otherwise. */
export function currentRelation(similar: SimilarDocument): RelationId {
  return similar.state === "joined" && similar.chosenRelation ? similar.chosenRelation : similar.relation
}

/** The two sides of the evidence, which says `a` for the smaller content id. */
function sides(similar: SimilarDocument): { mine: "a" | "b"; theirs: "a" | "b" } {
  return similar.contentId < similar.other.contentId ? { mine: "a", theirs: "b" } : { mine: "b", theirs: "a" }
}

/** The measures of a pair, in sentences anyone reads. */
export function measuresInWords(similar: SimilarDocument): string[] {
  const out = [`Comparten el ${percent(similar.similarity)} del texto.`]
  // When one is much shorter, how much of it is in the other says more.
  if (similar.cover - similar.similarity >= 0.05) {
    out.push(`El ${percent(similar.cover)} del más corto está, palabra por palabra, en el otro.`)
  }
  const evidence = similar.evidence
  if (evidence?.words) {
    const { mine, theirs } = sides(similar)
    out.push(
      `Este documento tiene ${plural(evidence.words[mine], "palabra", "palabras")}; el otro, ${formatCount(evidence.words[theirs])}.`,
    )
  }
  if (evidence?.recognized && (evidence.recognized.a || evidence.recognized.b)) {
    out.push("Parte del texto se leyó de una imagen y puede tener errores de lectura.")
  }
  return out
}

/**
 * The relation in a sentence, with its direction: "«Contrato firmado.pdf»
 * es el ejemplar firmado de este documento". `null` for a relation
 * without direction or when the direction is not known.
 */
export function relationSentence(
  similar: SimilarDocument,
  relation: RelationId,
  words: Vocabulary = vocabulary(),
): string | null {
  const of = words.relations[relation]?.of
  // The direction the engine found holds for the relation it proposed.
  if (!of || relation !== similar.relation || similar.subjectContentId === null) return null
  if (similar.subjectContentId === similar.contentId) return `Este documento ${of} «${similar.other.name}».`
  if (similar.subjectContentId === similar.other.contentId) return `«${similar.other.name}» ${of} este documento.`
  return null
}

export interface RelationChoice {
  id: RelationId
  name: string
  sameDocument: boolean
}

/**
 * The relations a person can join two documents with, as the catalog
 * lists them: first those found by comparing texts, then those inferred.
 * Not the automatic one (the same text is joined by the base itself) nor
 * plain "similar", which says no relation was found.
 */
export function relationChoices(
  definitions: RelationDef[],
  words: Vocabulary = vocabulary(),
): { measured: RelationChoice[]; inferred: RelationChoice[] } {
  const choices = definitions
    .filter((definition) => !definition.automatic && definition.id !== "similar")
    .map((definition) => ({
      id: definition.id,
      name: relationName(definition.id, words),
      sameDocument: definition.sameDocument,
      measured: definition.measured,
    }))
  const strip = ({ id, name, sameDocument }: RelationChoice) => ({ id, name, sameDocument })
  return {
    measured: choices.filter((choice) => choice.measured).map(strip),
    inferred: choices.filter((choice) => !choice.measured).map(strip),
  }
}

/** Whether joining with this relation leaves only one of the two as the wiki's source. */
export function makesOneDocument(relation: RelationId, definitions: RelationDef[]): boolean {
  return definitions.find((definition) => definition.id === relation)?.sameDocument ?? false
}

/**
 * Which of the two the app suggests as the wiki's source: the engine's
 * suggestion when it made one (the signed copy, the text not read from
 * an image, the longer one); otherwise the one with a party's signature,
 * then the one with more words, then the older content.
 */
export function suggestedWikiSource(similar: SimilarDocument): number {
  const ids = [similar.contentId, similar.other.contentId]
  if (similar.suggestedSource !== null && ids.includes(similar.suggestedSource)) return similar.suggestedSource
  const { mine, theirs } = sides(similar)
  const signed = (side: "a" | "b") => similar.evidence?.marks?.[side]?.includes("party-signature") ?? false
  if (signed(mine) !== signed(theirs)) return signed(mine) ? similar.contentId : similar.other.contentId
  const words = similar.evidence?.words
  if (words && words[mine] !== words[theirs]) return words[mine] > words[theirs] ? similar.contentId : similar.other.contentId
  return Math.min(...ids)
}

/**
 * When this document was joined with another as the same document and
 * the other was chosen as the wiki's source, the sentence that says so;
 * `null` while it feeds the wiki. The list is the document's similar
 * documents (each one has `contentId` = this document).
 */
export function passedOverNotice(items: SimilarDocument[]): string | null {
  const chosen = items.filter(
    (item) => item.state === "joined" && item.wikiSource !== null && item.wikiSource === item.other.contentId,
  )
  if (chosen.length === 0) return null
  const names = chosen.map((item) => `«${item.other.name}»`)
  const list = names.length === 1 ? names[0] : `${names.slice(0, -1).join(", ")} y ${names[names.length - 1]}`
  return names.length === 1
    ? `Este documento no alimenta el wiki: se unió con ${list}, que es la fuente elegida.`
    : `Este documento no alimenta el wiki: se unió con ${list}, que son las fuentes elegidas.`
}

/** How many families get a line of their own in Inicio; the rest go in one line. */
export const ATTENTION_GROUP_LINES = 5

/**
 * The lines of "requiere atención" for similar documents. Only the
 * pairs whose proposed relation makes both one document are escalated
 * (practically the same, signed copy, version): deciding them changes
 * which one feeds the wiki. "Same template", "contained in" and plain
 * "similar" are information about two different documents; they are
 * seen in each document and are not escalated (design, attention view).
 *
 * One line per family with such a pair, the largest first, leading to
 * its principal document, where the pairs are decided; one more for what
 * is left (pairs outside families, or families past the first few).
 */
export function buildSimilarAttention(
  pending: SimilarPending,
  groups: SimilarGroup[],
  maxGroups: number = ATTENTION_GROUP_LINES,
): AttentionItem[] {
  const waiting = groups.filter((group) => group.pendingSameDocument > 0)
  const shown = waiting.slice(0, maxGroups)
  const items: AttentionItem[] = shown.map((group) => ({
    id: `similar-group:${group.principal.contentId}`,
    tone: "busy",
    title: plural(
      group.pendingSameDocument,
      `documento podría ser el mismo que «${group.principal.name}»`,
      `documentos podrían ser el mismo que «${group.principal.name}»`,
    ),
    detail: groupDetail(group),
    target: { view: "document", locationId: group.principal.locationId },
  }))
  const inShown = shown.reduce((sum, group) => sum + group.pendingSameDocument, 0)
  const rest = Math.max(0, pending.sameDocumentPairs - inShown)
  if (rest > 0) {
    const more = shown.length > 0 ? " más" : ""
    items.push({
      id: "similar-pending",
      tone: "busy",
      title: plural(
        rest,
        `par de documentos${more} podría ser el mismo documento`,
        `pares de documentos${more} podrían ser el mismo documento`,
      ),
      detail:
        "Si se unen, al wiki va uno solo, el que elijas. Abrí cada documento y, en sus parecidos, unilo o descartalo.",
      target: { view: "documents", similarPending: true },
    })
  }
  return items
}

function groupDetail(group: SimilarGroup): string {
  const parts = [
    `Si se unen, al wiki va uno solo, el que elijas. En el documento principal, unilos o descartalos.`,
    `Familia de ${formatCount(group.members.length)} documentos.`,
  ]
  if (group.flags.includes("large")) parts.push("Creció mucho: conviene revisarla con cuidado.")
  if (group.flags.includes("dispersed")) parts.push("Algunos documentos se parecen entre sí pero no al principal.")
  return parts.join(" ")
}

/** How a family is named in a list: its principal, with how many documents it has. */
export function groupLabel(group: SimilarGroup): string {
  return `${group.principal.name} (${formatCount(group.members.length)})`
}
