import type {
  OpenPair,
  PairDecision,
  RelationDef,
  RelationId,
  SimilarDocument,
  SimilarGroup,
} from "@/commands/docbase"
import { relationName, vocabulary, type Vocabulary } from "@/vocabulary"
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

/** How a family is named in a list: its principal, with how many documents it has. */
export function groupLabel(group: SimilarGroup): string {
  return `${group.principal.name} (${formatCount(group.members.length)})`
}

// ── The similar documents screen ────────────────────────────────────────

/**
 * The families in the order of the screen: first those where two could
 * be the same document (what changes what goes to the wiki), the most
 * first; then those with something else to decide; then the decided
 * ones. Inside each, as the base gave them (largest first).
 */
export function orderFamilies(groups: readonly SimilarGroup[]): SimilarGroup[] {
  const rank = (group: SimilarGroup) => (group.pendingSameDocument > 0 ? 0 : group.pending > 0 ? 1 : 2)
  return groups
    .map((group, index) => ({ group, index }))
    .sort(
      (a, b) =>
        rank(a.group) - rank(b.group) ||
        b.group.pendingSameDocument - a.group.pendingSameDocument ||
        a.index - b.index,
    )
    .map(({ group }) => group)
}

/**
 * The pairs of a family seen from its principal (`items` are the
 * principal's similar documents): only its members, those that could be
 * the same document first, then the rest waiting, then the decided ones.
 */
export function familyPairs(
  items: readonly SimilarDocument[],
  group: SimilarGroup,
  definitions: RelationDef[],
): SimilarDocument[] {
  const members = new Set(group.members.map((member) => member.doc.contentId))
  const rank = (item: SimilarDocument) =>
    item.state !== "proposed" ? 2 : makesOneDocument(item.relation, definitions) ? 0 : 1
  return items
    .filter((item) => members.has(item.other.contentId))
    .map((item, index) => ({ item, index }))
    .sort((a, b) => rank(a.item) - rank(b.item) || a.index - b.index)
    .map(({ item }) => item)
}

/**
 * What confirming a family as "same template" decides: every pair of its
 * principal with a member still proposed as "same template" is joined as
 * such, with the evidence the person saw. Pairs proposed as anything else
 * are left for one by one.
 */
export function templateConfirmations(items: readonly SimilarDocument[], group: SimilarGroup): PairDecision[] {
  return familyPairs(items, group, [])
    .filter((item) => item.state === "proposed" && item.relation === TEMPLATE_RELATION)
    .map((item) => ({
      contentId: item.contentId,
      otherContentId: item.other.contentId,
      decision: { kind: "join", relation: TEMPLATE_RELATION, wikiSource: null },
      evidenceHash: item.evidenceHash,
    }))
}

/** The relation a family is confirmed with in one click. */
export const TEMPLATE_RELATION = "same-template"

/** The pairs outside the families' principals, those that could be the same document first. */
export function orderOpenPairs(pairs: readonly OpenPair[], definitions: RelationDef[]): OpenPair[] {
  const rank = (pair: OpenPair) => (makesOneDocument(pair.similar.relation, definitions) ? 0 : 1)
  return pairs
    .map((pair, index) => ({ pair, index }))
    .sort((a, b) => rank(a.pair) - rank(b.pair) || a.index - b.index)
    .map(({ pair }) => pair)
}
