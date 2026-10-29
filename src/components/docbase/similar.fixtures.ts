/**
 * Invented similar documents for the tests of the screens (R-24: no
 * name, file or figure here comes from the test folder).
 */
import type { DocRef, RelationDef, SimilarDocument, SimilarGroup } from "@/commands/docbase"

export function docRef(contentId: number, name: string, patch: Partial<DocRef> = {}): DocRef {
  return {
    contentId,
    locationId: contentId * 10,
    originName: "Carpeta de ejemplo",
    relPath: `Contratos/${name}`,
    name,
    copies: 1,
    ...patch,
  }
}

export function similarDocument(patch: Partial<SimilarDocument> = {}): SimilarDocument {
  return {
    contentId: 1,
    other: docRef(2, "Contrato de servicios firmado.pdf"),
    relation: "signed-copy",
    subjectContentId: 2,
    certainty: "high",
    description: "El mismo texto, y uno de los dos tiene la firma verificable de una parte.",
    jaccard: 0.9,
    containment: 0.99,
    cover: 0.99,
    similarity: 0.925,
    suggestedSource: 2,
    evidence: {
      engine: "similar-1",
      rule: "party-signature",
      measures: { jaccard: 0.9, containment: 0.99, cover: 0.99, similarity: 0.925 },
      words: { a: 1200, b: 1340 },
      ownWords: { a: 1200, b: 1340 },
      recognized: { a: false, b: false },
      marks: { a: [], b: ["party-signature"] },
      changes: {
        matchedWords: 1190,
        hunks: 1,
        removedWords: 0,
        addedWords: 140,
        dataTokens: 0,
        wordingTokens: 140,
        items: { wording: 1 },
        longestHunk: 140,
        largestBlock: 140,
        repeated: [],
        swaps: {},
        examples: [],
      },
    },
    evidenceHash: "hash-1-2",
    computedAt: Date.UTC(2026, 9, 1),
    state: "proposed",
    chosenRelation: null,
    wikiSource: null,
    decidedAt: null,
    possiblySame: false,
    uncertain: false,
    ...patch,
  }
}

/** The relations as the catalog lists them (shape only; names come from the vocabulary). */
export const RELATIONS: RelationDef[] = [
  { id: "same-document", sameDocument: true, directed: false, measured: true, automatic: true, asksPerson: false },
  { id: "near-identical", sameDocument: true, directed: false, measured: true, automatic: false, asksPerson: true },
  { id: "signed-copy", sameDocument: true, directed: true, measured: true, automatic: false, asksPerson: true },
  { id: "preview", sameDocument: false, directed: true, measured: true, automatic: false, asksPerson: true },
  { id: "version", sameDocument: true, directed: false, measured: true, automatic: false, asksPerson: true },
  { id: "contained-in", sameDocument: false, directed: true, measured: true, automatic: false, asksPerson: true },
  { id: "same-template", sameDocument: false, directed: false, measured: true, automatic: false, asksPerson: false },
  { id: "similar", sameDocument: false, directed: false, measured: true, automatic: false, asksPerson: false },
  { id: "same-meaning", sameDocument: false, directed: false, measured: false, automatic: false, asksPerson: false },
  { id: "reply-to", sameDocument: false, directed: true, measured: false, automatic: false, asksPerson: false },
]

/**
 * A family whose first `pending` members are still undecided (the rest
 * were joined). Their relation is `relation` ("same-template" by
 * default); like the base, it counts as pending only what asks a person
 * (D-497), and in `pendingSameDocument` what of that makes one document.
 */
export function similarGroup(
  principal: DocRef,
  others: DocRef[],
  pending: number,
  flags: string[] = [],
  relation: string = "same-template",
): SimilarGroup {
  const definition = RELATIONS.find((r) => r.id === relation)
  const asks = definition?.asksPerson ?? false
  const oneDocument = definition?.sameDocument ?? false
  return {
    principal,
    members: [principal, ...others].map((doc, index) => ({
      doc,
      relation: index === 0 ? null : relation,
      cover: index === 0 ? null : 0.9,
      state: index === 0 ? null : index <= pending ? "proposed" : "joined",
      uncertain: false,
    })),
    flags,
    pending: asks ? pending : 0,
    pendingSameDocument: asks && oneDocument ? pending : 0,
    templateFields: relation === "same-template" ? 3 : null,
    possiblySame: 0,
    uncertain: 0,
  }
}
