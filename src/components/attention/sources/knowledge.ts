import {
  decideMatter,
  undoIdentityDecision,
  getKnowledgeAttention,
  joinMatter,
  knowledgeReadAgainFailed,
  leaveMatter,
  listIdentityDoubts,
  type DoubtView,
  type KnowledgeAttention,
} from "@/commands/docbase"
import type { AttentionCase, AttentionNotice } from "@/lib/attention/types"
import { entityTypeName, predicateName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatCount, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { decisionInWords, doubtReason, groupDoubts, reviewInWords } from "@/components/knowledge/knowledge"

/**
 * What the knowledge layer raises: readings that failed or lost many
 * quotes, identities in doubt, data in which the documents differ, the
 * matter the model proposed for a loose document, recent joins by an
 * identifier, pages kept by their notes, and how far the reading, the
 * resolution and the pages went. All of it is about the archive.
 */
export const KNOWLEDGE_SOURCE = "knowledge"

/** Cases listed when a line is opened; the rest are in its screen. */
export const KNOWLEDGE_CASES = 30

export interface KnowledgeFacts {
  attention: KnowledgeAttention
  doubts: DoubtView[] | null
}

export function knowledgeNotices(facts: KnowledgeFacts, words: Vocabulary = vocabulary()): AttentionNotice[] {
  const { attention } = facts
  const { status } = attention
  const terms = words.knowledge
  const out: AttentionNotice[] = []
  const notice = (patch: Omit<AttentionNotice, "source" | "id">): AttentionNotice => ({
    source: KNOWLEDGE_SOURCE,
    id: `${KNOWLEDGE_SOURCE}:${patch.topic}`,
    ...patch,
  })
  const cut = <T,>(list: T[]) => ({ list: list.slice(0, KNOWLEDGE_CASES), more: Math.max(0, list.length - KNOWLEDGE_CASES) })
  const openDocument = (locationId: number | null) =>
    locationId === null ? [] : [{ kind: "go" as const, label: "Abrir", target: { view: "document" as const, locationId } }]
  const who = terms.entities.toLowerCase()

  // ── Problems ────────────────────────────────────────────────────────
  if (attention.failed.length > 0) {
    const { list, more } = cut(attention.failed)
    out.push(
      notice({
        kind: "problem",
        topic: "reading-failed",
        priority: "normal",
        count: attention.failed.length,
        title: plural(
          attention.failed.length,
          "documento no se pudo leer para saber a quiénes nombra",
          "documentos no se pudieron leer para saber a quiénes nombran",
        ),
        explanation: `La respuesta del modelo no se pudo usar, o volvió vacía de un texto largo: sus ${who} no están en el registro ni en sus páginas.`,
        todo: "Mirá el motivo de cada uno. Se vuelve a leer cuando cambia su texto o la pregunta de lectura, o cuando pedís volver a leerlos.",
        resolvedWhen: "Desaparece cuando una lectura nueva del documento sale bien.",
        actions: [{ kind: "run", label: "Volver a leer", run: () => knowledgeReadAgainFailed() }],
        cases: list.map((item) => ({
          id: `failed:${item.document.contentId}`,
          title: item.document.name || `Documento ${item.document.contentId}`,
          detail: item.error ?? undefined,
          actions: openDocument(item.document.locationId),
        })),
        moreCases: more,
      }),
    )
  }
  if (attention.noisy.length > 0) {
    const { list, more } = cut(attention.noisy)
    out.push(
      notice({
        kind: "problem",
        topic: "reading-noisy",
        priority: "normal",
        count: attention.noisy.length,
        title: plural(
          attention.noisy.length,
          "documento perdió muchas citas al leerlo",
          "documentos perdieron muchas citas al leerlos",
        ),
        explanation:
          "Lo que el modelo dijo de ellos no estaba en su texto y se descartó. Suele pasar con escaneos mal reconocidos: faltan nombres y datos que el documento sí tiene.",
        todo: "Abrí el documento y compará su texto con el original. Si el escaneo se lee mal, reemplazalo por una copia mejor.",
        resolvedWhen: "Desaparece cuando el documento cambia y su lectura conserva sus citas.",
        actions: [],
        cases: list.map((item) => ({
          id: `noisy:${item.document.contentId}`,
          title: item.document.name || `Documento ${item.document.contentId}`,
          detail: `${plural(item.discarded, "cita descartada", "citas descartadas")}, ${formatCount(item.kept)} conservadas`,
          actions: openDocument(item.document.locationId),
        })),
        moreCases: more,
      }),
    )
  }
  if (attention.discardedSentences > 0) {
    out.push(
      notice({
        kind: "problem",
        topic: "synthesis-discarded",
        priority: "low",
        count: attention.discardedSentences,
        title: plural(
          attention.discardedSentences,
          "oración de las páginas se descartó por no remitir a una cita",
          "oraciones de las páginas se descartaron por no remitir a una cita",
        ),
        explanation:
          "Cada oración de lo que dicen los documentos tiene que remitir a una afirmación con su cita; las que no lo hacen no se escriben. Lo demás de la página sigue completo.",
        todo: "No hace falta hacer nada; si son muchas en una página, revisá sus afirmaciones.",
        resolvedWhen: "Cambia cuando la página se vuelve a escribir.",
        actions: [],
      }),
    )
  }

  // ── Decisions ───────────────────────────────────────────────────────
  const groups = groupDoubts(facts.doubts ?? [])
  if (groups.length > 0) {
    const { list, more } = cut(groups)
    out.push(
      notice({
        kind: "decision",
        topic: "identity-doubts",
        priority: "normal",
        count: groups.length,
        title: plural(groups.length, "identidad dudosa espera una decisión", "identidades dudosas esperan una decisión"),
        explanation:
          "La app no pudo saber si son la misma persona u organización, y ante la duda no las une: mientras tanto cada una tiene su propia página.",
        todo: "Mirá lo que dice cada documento de cada lado y decidí si son la misma o no.",
        resolvedWhen: "Desaparece cuando cada una se decide.",
        actions: [{ kind: "go", label: "Resolver", target: { view: "identity-doubts", group: list[0].id } }],
        cases: list.map((group) => ({
          id: `doubt:${group.id}`,
          title: group.sides.map((side) => `«${side.name}»`).join(" · "),
          detail: `${entityTypeName(group.kind, false, words)} · ${doubtReason(group.doubts[0].reason, group.doubts[0].detail)}`,
          actions: [{ kind: "go", label: "Resolver", target: { view: "identity-doubts", group: group.id } }],
        })),
        moreCases: more,
      }),
    )
  }
  if (attention.conflicts.length > 0) {
    const { list, more } = cut(attention.conflicts)
    out.push(
      notice({
        kind: "decision",
        topic: "conflicts",
        priority: "normal",
        count: attention.conflicts.length,
        title: plural(
          attention.conflicts.length,
          "dato en el que los documentos difieren",
          "datos en los que los documentos difieren",
        ),
        explanation:
          "Dos documentos dicen cosas distintas de lo mismo para el mismo momento. La página muestra todos los valores; la app no elige uno.",
        todo: "Si uno está mal, marcalo como incorrecto desde la entidad; si los dos valen, no hace falta hacer nada.",
        resolvedWhen: "Desaparece cuando se marca el valor incorrecto o un documento cambia.",
        actions: [{ kind: "go", label: `Ver ${who}`, target: { view: "entities", entityId: list[0].entityId } }],
        cases: list.map((item) => ({
          id: `conflict:${item.entityId}:${item.predicate}:${item.within ?? ""}`,
          title: `${item.entityName}: ${predicateName(item.predicate, false, words).toLowerCase()}${item.within ? ` (en ${item.within})` : ""}`,
          detail: item.values.map((value) => `«${value}»`).join(" / "),
          actions: [{ kind: "go", label: "Ver", target: { view: "entities", entityId: item.entityId } }],
        })),
        moreCases: more,
      }),
    )
  }
  if (attention.mattersToConfirm.length > 0) {
    const { list, more } = cut(attention.mattersToConfirm)
    const one = terms.matter.toLowerCase()
    out.push(
      notice({
        kind: "decision",
        topic: "matters-to-confirm",
        priority: "normal",
        count: attention.mattersToConfirm.length,
        title: plural(
          attention.mattersToConfirm.length,
          `${one} que propone la app para confirmar`,
          `${terms.matters.toLowerCase()} que propone la app para confirmar`,
        ),
        explanation: `La app juntó documentos distintos que comparten algo propio de un mismo ${one}: un número, un importe, una referencia o una relación entre ellos. Un documento con sus copias o versiones nunca es un ${one} por sí solo. Hasta que una persona lo confirme es una propuesta.`,
        todo: `Mirá sus documentos y por qué están juntos; confirmalo o descartalo.`,
        resolvedWhen: `Desaparece cuando se confirma o se descarta. Lo descartado no vuelve mientras la app encuentre los mismos documentos.`,
        actions: [{ kind: "go", label: `Ver ${terms.matters.toLowerCase()}`, target: { view: "matters", matterId: list[0].id } }],
        cases: list.map(
          (item): AttentionCase => ({
            id: `matter-to-confirm:${item.id}`,
            title: item.name,
            detail: `${plural(item.documents, "documento", "documentos")} · ${matterReasonsInWords(item.reasons)}`,
            actions: [
              { kind: "run", label: "Confirmar", run: () => decideMatter(item.id, "confirm") },
              { kind: "run", label: "Descartar", run: () => decideMatter(item.id, "discard") },
              { kind: "go", label: "Ver", target: { view: "matters", matterId: item.id } },
            ],
          }),
        ),
        moreCases: more,
      }),
    )
  }
  if (attention.proposedMatters.length > 0) {
    const { list, more } = cut(attention.proposedMatters)
    out.push(
      notice({
        kind: "decision",
        topic: "proposed-matters",
        priority: "normal",
        count: attention.proposedMatters.length,
        title: plural(
          attention.proposedMatters.length,
          `documento suelto que el modelo propone para un ${terms.matter.toLowerCase()}`,
          `documentos sueltos que el modelo propone para un ${terms.matter.toLowerCase()}`,
        ),
        explanation: `Ningún dato propio los pone en un ${terms.matter.toLowerCase()}, pero comparten las partes con uno y el modelo cree que son de él. Hasta que una persona lo confirme es solo una propuesta y no une a otros documentos.`,
        todo: "Confirmá o descartá cada propuesta.",
        resolvedWhen: "Desaparece cuando cada propuesta se confirma o se descarta.",
        actions: [{ kind: "go", label: `Ver ${terms.matters.toLowerCase()}`, target: { view: "matters", matterId: list[0].matterId } }],
        cases: list.map(
          (item): AttentionCase => ({
            id: `proposed:${item.document.contentId}:${item.matterId}`,
            title: `${item.document.name} → ${item.matterName}`,
            detail: item.reason ?? undefined,
            actions: [
              {
                kind: "run",
                label: "Confirmar",
                run: () => joinMatter(`content:${item.document.contentId}`, item.matterId, "propuesta del modelo confirmada"),
              },
              {
                kind: "run",
                label: "Descartar",
                run: () => leaveMatter(`content:${item.document.contentId}`, item.matterId, "propuesta del modelo descartada"),
              },
              { kind: "go", label: "Ver", target: { view: "matters", matterId: item.matterId } },
            ],
          }),
        ),
        moreCases: more,
      }),
    )
  }
  if (attention.identifierJoins.length > 0) {
    const { list, more } = cut(attention.identifierJoins)
    out.push(
      notice({
        kind: "decision",
        topic: "identifier-joins",
        priority: "low",
        count: attention.identifierJoins.length,
        title: plural(
          attention.identifierJoins.length,
          "unión reciente por identificador de nombres distintos",
          "uniones recientes por identificador de nombres distintos",
        ),
        explanation: `El mismo identificador (${strongIdentifiers(words)}) unió solo maneras distintas de escribir un nombre. Casi siempre es correcto; se muestran por si un identificador quedó atribuido a quien no es.`,
        todo: "Si alguna está mal, separá la mención desde la entidad.",
        resolvedWhen: "Desaparecen con el tiempo; separar una la deshace.",
        actions: [],
        cases: list.map((item) => ({
          id: `joined:${item.entityId}`,
          title: item.names.map((name) => `«${name}»`).join(" = "),
          detail: [item.identifiers.join(", "), plural(item.documents, "documento", "documentos")].filter(Boolean).join(" · "),
          actions: [{ kind: "go", label: "Ver", target: { view: "entities", entityId: item.entityId } }],
        })),
        moreCases: more,
      }),
    )
  }
  if (attention.decisionsToReview.length > 0) {
    const { list, more } = cut(attention.decisionsToReview)
    out.push(
      notice({
        kind: "decision",
        topic: "decisions-to-review",
        priority: "normal",
        count: attention.decisionsToReview.length,
        title: plural(
          attention.decisionsToReview.length,
          "decisión tuya ya no encuentra lo que decidía",
          "decisiones tuyas ya no encuentran lo que decidían",
        ),
        explanation:
          "Se volvió a leer un documento y lo que nombraba la decisión ya no está (o está más de una vez). La app no la aplica a ciegas ni la borra: queda para que la mires.",
        todo: "Abrí el documento y volvé a decidir sobre lo que dice ahora; después deshacé la decisión vieja.",
        resolvedWhen: "Desaparece cuando la decisión vuelve a encontrar lo que nombraba o cuando la deshacés.",
        actions: [],
        cases: list.map((item) => ({
          id: `review:${item.decision.id}`,
          title: `${decisionInWords(item.decision, { mentions: [] })}${item.document ? ` (en ${item.document.name})` : ""}`,
          detail: reviewInWords(item.reason),
          actions: [
            ...openDocument(item.document?.locationId ?? null),
            { kind: "run" as const, label: "Deshacer la decisión", run: () => undoIdentityDecision(item.decision.id) },
          ],
        })),
        moreCases: more,
      }),
    )
  }
  if (attention.keptPages.length > 0) {
    out.push(
      notice({
        kind: "decision",
        topic: "kept-pages",
        priority: "low",
        count: attention.keptPages.length,
        title: plural(
          attention.keptPages.length,
          "página se conserva por sus notas, sin documentos detrás",
          "páginas se conservan por sus notas, sin documentos detrás",
        ),
        explanation:
          "Su entidad dejó el registro (se unió con otra o ya no la nombra ningún documento), pero tiene notas tuyas, así que queda una versión mínima con ellas.",
        todo: "Abrila, pasá las notas a donde corresponda y borralas de ahí.",
        resolvedWhen: "Desaparece cuando sus notas quedan vacías o su entidad vuelve.",
        actions: [],
        cases: attention.keptPages.map((page) => ({
          id: `kept:${page.entityId}`,
          title: page.name ?? page.path,
          detail: page.mergedInto ? "se unió con otra entidad" : page.path,
          actions: [{ kind: "go", label: "Abrir", target: { view: "wiki-page", path: page.path } }],
        })),
      }),
    )
  }
  if (attention.leftWithDocuments.length > 0) {
    const { list, more } = cut(attention.leftWithDocuments)
    out.push(
      notice({
        kind: "decision",
        topic: "left-with-documents",
        priority: "low",
        count: attention.leftWithDocuments.length,
        title: plural(
          attention.leftWithDocuments.length,
          "entidad dejó el registro porque sus documentos se excluyeron",
          "entidades dejaron el registro porque sus documentos se excluyeron",
        ),
        explanation:
          "Solo las nombraban documentos que se dejaron afuera. Su lectura se conserva: si el documento vuelve, vuelven con el mismo identificador.",
        todo: "Si alguno no tenía que quedar afuera, volvé a incluirlo desde el documento.",
        resolvedWhen: "Desaparece cuando el documento vuelve a contar.",
        actions: [],
        cases: list.map((item) => ({
          id: `left:${item.entityId}`,
          title: `${item.name} (${entityTypeName(item.kind, false, words).toLowerCase()})`,
          detail: item.documents.map((d) => d.name).join(", "),
          actions: openDocument(item.documents[0]?.locationId ?? null),
        })),
        moreCases: more,
      }),
    )
  }

  // ── In progress ─────────────────────────────────────────────────────
  if (status.pending > 0) {
    out.push(
      notice({
        kind: "ongoing",
        topic: "reading",
        priority: "normal",
        count: status.pending,
        title: `Leyendo ${plural(status.pending, "documento", "documentos")} para saber a quiénes nombran`,
        explanation: `El modelo lee cada documento una vez y dice qué ${who} nombra, con qué rol y qué dice de cada uno, con la cita que lo respalda.`,
        todo: status.engine
          ? `Se leen solos mientras el ${words.archive.wiki.toLowerCase()} esté activo.`
          : "Elegí un modelo para que se lean.",
        resolvedWhen: "Desaparece cuando termina de leerlos.",
        actions: [],
        progress: { done: status.extracted, total: status.documents },
      }),
    )
  }
  if (status.toResolve > 0) {
    out.push(
      notice({
        kind: "ongoing",
        topic: "resolving",
        priority: "normal",
        count: status.toResolve,
        title: "Reconociendo quién es quién",
        explanation: `Se juntan los nombres de cada documento con los de los demás: por su identificador, por su nombre y, cuando hace falta, preguntándole al modelo.`,
        todo: "No hace falta hacer nada.",
        resolvedWhen: "Desaparece cuando termina.",
        actions: [],
      }),
    )
  }
  if (status.toProject > 0 || attention.synthesisPending > 0) {
    const waiting = attention.synthesisPending
    out.push(
      notice({
        kind: "ongoing",
        topic: "pages",
        priority: "normal",
        count: Math.max(status.toProject, waiting),
        title:
          waiting > 0
            ? `Escribiendo lo que dicen los documentos en ${plural(waiting, "página", "páginas")}`
            : `Rehaciendo las páginas de ${who}`,
        explanation:
          "Cada página lista todos los documentos de su entidad y, desde sus afirmaciones con cita, el modelo escribe lo que dicen. Mientras tanto la página ya está, con sus documentos.",
        todo: status.engine
          ? `Sigue sola mientras el ${words.archive.wiki.toLowerCase()} esté activo.`
          : "Elegí un modelo para que se escriban.",
        resolvedWhen: "Desaparece cuando todas las páginas están escritas.",
        actions: [],
        progress: waiting > 0 ? { done: Math.max(0, status.pages - waiting), total: status.pages } : undefined,
      }),
    )
  }
  return out
}

/** The kinds of identifier the vertical names, but "other": "DNI, CUIT o CUIL…". */
function strongIdentifiers(words: Vocabulary): string {
  return Object.entries(words.knowledge.identifiers)
    .filter(([id]) => id !== "otro")
    .map(([, name]) => name)
    .join(", ")
}

/** The source itself: asks the base again whenever it says something changed. */
export function useKnowledgeNotices(): AttentionNotice[] | null {
  const attention = useDocbaseQuery("attention:knowledge", getKnowledgeAttention)
  const doubts = useDocbaseQuery("attention:knowledge-doubts", listIdentityDoubts)
  if (attention.error) throw new Error(attention.error)
  if (!attention.data) return null
  return knowledgeNotices({ attention: attention.data, doubts: doubts.data })
}

/** Why the documents of a proposed matter are together, in a few words. */
export function matterReasonsInWords(reasons: Record<string, number>): string {
  const names: Record<string, string> = {
    persona: "decisión de una persona",
    documento: "relación entre documentos",
    asunto: "la misma referencia",
    numero: "el mismo número",
    importe: "el mismo importe",
  }
  const parts = Object.entries(reasons)
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([kind, n]) => `${names[kind] ?? kind} (${n})`)
  return parts.length > 0 ? `comparten ${parts.join(", ")}` : "sin motivo guardado"
}
