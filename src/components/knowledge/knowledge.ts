import type {
  ClaimView,
  DocumentMatter,
  DoubtView,
  EntityDetail,
  EntityMention,
  IdentityDecisionView,
} from "@/commands/docbase"
import { entityTypeName, roleName, type Vocabulary, vocabulary } from "@/vocabulary"
import { plural } from "@/components/docbase/format"

/**
 * What the screens of the knowledge layer compute from what the base
 * gives: the doubts grouped into decisions, the documents of an entity by
 * matter, and the words for the reasons and decisions. Pure functions,
 * tested on their own.
 */

/** One side of a doubtful pair: an entity, or a group of mentions not resolved yet. */
export interface DoubtSide {
  /** What a decision refers to: the group key the base gave. */
  key: string
  entityId: string | null
  name: string
}

/**
 * Doubts that are one decision for a person: the pairs that share an
 * entity, joined in chain (the same person in doubt with three others is
 * one look at four sides). It is named by the side most in doubt.
 */
export interface DoubtGroup {
  id: string
  kind: string
  sides: DoubtSide[]
  doubts: DoubtView[]
}

function sideOf(doubt: DoubtView, which: "a" | "b"): DoubtSide {
  const entityId = which === "a" ? doubt.entityA : doubt.entityB
  const name = (which === "a" ? doubt.nameA : doubt.nameB) ?? "(sin nombre)"
  return { key: which === "a" ? doubt.a : doubt.b, entityId, name }
}

/** The doubts as decisions: connected by the entities they share. */
export function groupDoubts(doubts: DoubtView[]): DoubtGroup[] {
  const parent = new Map<string, string>()
  const find = (x: string): string => {
    let root = x
    while (parent.get(root) !== root) root = parent.get(root) ?? root
    let at = x
    while (parent.get(at) !== root) {
      const next = parent.get(at) ?? root
      parent.set(at, root)
      at = next
    }
    return root
  }
  const node = (doubt: DoubtView, which: "a" | "b") => {
    const id = (which === "a" ? doubt.entityA : doubt.entityB) ?? (which === "a" ? doubt.a : doubt.b)
    if (!parent.has(id)) parent.set(id, id)
    return id
  }
  for (const doubt of doubts) {
    const a = find(node(doubt, "a"))
    const b = find(node(doubt, "b"))
    if (a !== b) parent.set(a < b ? b : a, a < b ? a : b)
  }
  const groups = new Map<string, DoubtGroup>()
  for (const doubt of doubts) {
    const root = find(node(doubt, "a"))
    let group = groups.get(root)
    if (!group) {
      group = { id: root, kind: doubt.kind, sides: [], doubts: [] }
      groups.set(root, group)
    }
    group.doubts.push(doubt)
    for (const side of [sideOf(doubt, "a"), sideOf(doubt, "b")]) {
      const same = (s: DoubtSide) => (side.entityId ? s.entityId === side.entityId : s.key === side.key)
      if (!group.sides.some(same)) group.sides.push(side)
    }
  }
  // The side in most doubts first; larger groups first.
  const inDoubts = (group: DoubtGroup, side: DoubtSide) =>
    group.doubts.filter((d) => [sideOf(d, "a"), sideOf(d, "b")].some((s) => (side.entityId ? s.entityId === side.entityId : s.key === side.key)))
      .length
  for (const group of groups.values()) {
    group.sides.sort((x, y) => inDoubts(group, y) - inDoubts(group, x) || x.name.localeCompare(y.name, "es"))
  }
  return [...groups.values()].sort(
    (x, y) => y.sides.length - x.sides.length || x.sides[0].name.localeCompare(y.sides[0].name, "es"),
  )
}

/** Why the app could not tell whether two are the same, in words. */
export function doubtReason(reason: string, detail: string | null = null): string {
  const said = detail ? ` (${detail})` : ""
  switch (reason) {
    case "identificador-parecido":
      return `Los nombres coinciden y los identificadores difieren por poco${said}: puede ser un dígito mal leído en un escaneo.`
    case "identificador-otro-nombre":
      return "Tienen el mismo identificador, pero sus nombres no se parecen: el identificador puede estar atribuido a quien no es."
    case "identificador-otra-forma":
    case "otra-forma":
    case "modelo-misma-otra-forma":
      return "Tienen el mismo nombre con distinta forma societaria: suelen ser organizaciones distintas."
    case "mismo-nombre-otro-identificador":
    case "otro-identificador":
    case "modelo-misma-otro-identificador":
      return "Tienen el mismo nombre y otro identificador: dos con el mismo nombre, o un identificador mal leído."
    case "sin-forma-varias-sociedades":
      return "El nombre aparece sin forma societaria y hay varias sociedades con ese nombre."
    case "modelo-dudosa":
      return "El modelo no pudo decidir con lo que dicen los documentos." + (detail ? ` Dijo: «${detail}».` : "")
    case "modelo-misma-nombre-distinto":
      return (
        "El modelo cree que son la misma, pero los nombres difieren en una letra y nada más los une: puede ser un error de escritura o dos personas distintas con el mismo apellido." +
        (detail ? ` Dijo: «${detail}».` : "")
      )
    case "separadas-por-una-persona":
    case "modelo-misma-separadas-por-una-persona":
      return "El modelo dice que son la misma, pero una persona ya decidió que no."
    default:
      return reason.startsWith("modelo-misma")
        ? "El modelo dice que son la misma, pero algo de los documentos lo impide."
        : "La app no pudo decidir si son la misma."
  }
}

/** What joined a mention to its entity, in words. */
export function resolvedByName(resolvedBy: string | null): string | null {
  switch (resolvedBy) {
    case "persona":
      return "unida por una persona"
    case "identificador":
      return "unida por su identificador"
    case "nombre":
      return "unida por el nombre"
    case "modelo":
      return "unida por el modelo"
    default:
      return null
  }
}

/** The documents of an entity, each with its roles, under its matter. */
export interface EntityDocument {
  contentId: number
  locationId: number | null
  name: string
  roles: string[]
  mentions: EntityMention[]
  proposed: boolean
}

export interface MatterGroup {
  /** None for the documents of no matter. */
  matterId: string | null
  name: string
  pagePath: string | null
  documents: EntityDocument[]
}

/**
 * The documents of an entity grouped by matter, the largest matter first
 * and the documents of no matter last. A document the model proposed for a
 * matter is listed there, marked as a proposal.
 */
export function documentsByMatter(
  mentions: EntityMention[],
  matters: DocumentMatter[],
  withoutMatter = vocabulary().knowledge.withoutMatter,
): MatterGroup[] {
  const documents = new Map<number, EntityDocument>()
  for (const mention of mentions) {
    let doc = documents.get(mention.contentId)
    if (!doc) {
      doc = {
        contentId: mention.contentId,
        locationId: mention.locationId,
        name: mention.document,
        roles: [],
        mentions: [],
        proposed: false,
      }
      documents.set(mention.contentId, doc)
    }
    doc.mentions.push(mention)
    if (!doc.roles.includes(mention.role)) doc.roles.push(mention.role)
  }
  const groups = new Map<string, MatterGroup>()
  const placed = new Set<number>()
  for (const matter of matters) {
    const doc = documents.get(matter.contentId)
    if (!doc) continue
    let group = groups.get(matter.matterId)
    if (!group) {
      group = { matterId: matter.matterId, name: matter.name, pagePath: matter.pagePath, documents: [] }
      groups.set(matter.matterId, group)
    }
    group.documents.push(matter.proposed ? { ...doc, proposed: true } : doc)
    if (!matter.proposed) placed.add(doc.contentId)
  }
  const byName = (x: EntityDocument, y: EntityDocument) => x.name.localeCompare(y.name, "es")
  const out = [...groups.values()]
  for (const group of out) group.documents.sort(byName)
  out.sort((x, y) => y.documents.length - x.documents.length || x.name.localeCompare(y.name, "es"))
  const loose = [...documents.values()].filter((doc) => !placed.has(doc.contentId)).sort(byName)
  if (loose.length > 0) out.push({ matterId: null, name: withoutMatter, pagePath: null, documents: loose })
  return out
}

/** "parte, firmante" with the words of the vertical. */
export function rolesInWords(roles: string[], words: Vocabulary = vocabulary()): string {
  return roles.map((role) => roleName(role, words)).join(", ")
}

/** A decision about an entity, in words, for its list of decisions. */
export function decisionInWords(decision: IdentityDecisionView, entity: Pick<EntityDetail, "mentions">): string {
  const documentOf = (key: string | null): string | null => {
    const content = key?.match(/^(?:cluster|evidence):(\d+):/)?.[1]
    if (!content) return null
    return entity.mentions.find((m) => m.contentId === Number(content))?.document ?? null
  }
  const nameOf = (key: string | null): string | null => key?.match(/^cluster:\d+:[^:]+:(.+)$/)?.[1] ?? null
  switch (decision.kind) {
    case "join": {
      const [a, b] = [nameOf(decision.a), nameOf(decision.b)]
      return a && b ? `Son la misma: «${a}» y «${b}»` : "Son la misma"
    }
    case "separate": {
      const [a, b] = [nameOf(decision.a), nameOf(decision.b)]
      const where = documentOf(decision.a)
      return a && b ? `No son la misma: «${a}»${where ? ` (en ${where})` : ""} y «${b}»` : "No son la misma"
    }
    case "confirm":
      return "Un dato confirmado"
    case "wrong": {
      const where = documentOf(decision.a)
      if (decision.a.startsWith("claim:")) return "Un dato marcado como incorrecto"
      if (decision.a.startsWith("evidence:")) return `Un dato marcado como incorrecto${where ? ` en ${where}` : ""}`
      return `Una mención que no es una entidad${where ? ` (en ${where})` : ""}`
    }
    default:
      return decision.kind
  }
}

/** "3 personas, 2 organizaciones" for a count by type. */
export function countByKind(byKind: Record<string, number>, words: Vocabulary = vocabulary()): string {
  return Object.entries(byKind)
    .filter(([, n]) => n > 0)
    .map(([kind, n]) => plural(n, entityTypeName(kind, false, words).toLowerCase(), entityTypeName(kind, true, words).toLowerCase()))
    .join(", ")
}

/**
 * How sure the record is of a claim, in words. Every claim has its quote,
 * checked in the text of its document; that does not make the reading
 * right, so the screens say "con cita comprobada", never "verificado".
 */
export function certaintyInWords(claim: Pick<ClaimView, "certainty" | "reasons">): {
  label: string
  tone: "plain" | "doubt" | "person"
  why: string
} {
  const why = claim.reasons
    .map((reason) => {
      switch (reason) {
        case "conflict":
          return "los documentos difieren"
        case "identity":
          return "no es seguro de quién se habla"
        case "attribution":
          return "la cita nombra a otra persona u organización"
        case "states":
          return "los documentos dicen estados distintos del importe"
        default:
          return null
      }
    })
    .filter((text): text is string => text !== null)
    .join("; ")
  switch (claim.certainty) {
    case "confirmed":
      return { label: "confirmado por una persona", tone: "person", why: "" }
    case "corrected":
      return { label: "corregido por una persona", tone: "person", why: "" }
    case "doubtful":
      return { label: "sin confirmar", tone: "doubt", why }
    default:
      return { label: "con cita comprobada", tone: "plain", why: "" }
  }
}

/** Why a decision of a person waits for a review, in words. */
export function reviewInWords(reason: string): string {
  return reason === "varias"
    ? "Al volver a leer el documento, lo que decidiste coincide con más de una cosa: la app no elige."
    : "Al volver a leer el documento, ya no aparece lo que decidiste: la decisión no se está aplicando."
}
