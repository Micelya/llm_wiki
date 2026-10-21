import {
  getDocument,
  getHome,
  getSimilarPending,
  listDocuments,
  listProposedTypes,
  listSimilarGroups,
  rescanOrigins,
  type DocumentState,
  type Home,
  type ProposedType,
  type SimilarGroup,
  type SimilarPending,
} from "@/commands/docbase"
import { DEFAULT_DOCUMENT_FILTER } from "@/stores/docbase-store"
import type { AttentionCase, AttentionNotice } from "@/lib/attention/types"
import { documentTypeName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatCount, plural } from "@/components/docbase/format"
import { pendingCount } from "@/components/docbase/attention"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"

/**
 * What the document base raises: origins that cannot be read, files and
 * documents that could not be processed, similar documents that could be
 * the same one, document types the model proposed, and what it is still
 * reading. All of it is about the archive (the space), whatever project
 * is open.
 */
export const DOCBASE_SOURCE = "docbase"

/** Cases listed when a line of documents is opened; the rest are in Documentos. */
export const DOCUMENT_CASES = 20

export interface DocbaseFacts {
  home: Home
  similarPending: SimilarPending | null
  groups: SimilarGroup[] | null
  proposedTypes: ProposedType[] | null
}

export function docbaseNotices(facts: DocbaseFacts, words: Vocabulary = vocabulary()): AttentionNotice[] {
  const { home } = facts
  const out: AttentionNotice[] = []
  const count = (state: DocumentState) => home.health.byState[state] ?? 0
  const notice = (patch: Omit<AttentionNotice, "source" | "id"> & { id?: string }): AttentionNotice => ({
    source: DOCBASE_SOURCE,
    id: `${DOCBASE_SOURCE}:${patch.id ?? patch.topic}`,
    ...patch,
  })

  // ── Problems ────────────────────────────────────────────────────────
  const unreachable = home.origins.filter((origin) => origin.reachable === false)
  if (unreachable.length > 0) {
    out.push(
      notice({
        kind: "problem",
        topic: "origins",
        priority: "high",
        count: unreachable.length,
        title:
          unreachable.length === 1
            ? `No se puede leer «${unreachable[0].name}»`
            : `${formatCount(unreachable.length)} carpetas incorporadas no se pueden leer`,
        explanation:
          "La carpeta no está o aparece vacía. Lo que se sabía de sus archivos se conserva hasta que vuelva, pero no se ve lo nuevo.",
        todo: "Revisá que el disco o la unidad de red esté conectado y volvé a barrer.",
        resolvedWhen: "Desaparece cuando un barrido vuelve a leer la carpeta.",
        actions: [
          { kind: "run", label: "Volver a barrer", run: () => rescanOrigins() },
          { kind: "go", label: "Ver orígenes", target: { view: "origins" } },
        ],
        cases:
          unreachable.length > 1
            ? unreachable.map((origin) => ({ id: origin.id, title: origin.name, detail: origin.location }))
            : undefined,
      }),
    )
  }
  if (count("unreadable") > 0) {
    out.push(
      documentsNotice("unreadable", count("unreadable"), {
        kind: "problem",
        priority: "high",
        title: plural(count("unreadable"), "archivo no se pudo leer", "archivos no se pudieron leer"),
        explanation:
          "Sin leerlos no se sabe qué documento son: no se comparan con otros ni se sabe a quiénes nombran.",
        todo: "Suelen estar abiertos en otro programa o sin descargar del disco en la nube. Cerralos o descargalos; se vuelve a intentar en cada barrido.",
        resolvedWhen: "Desaparece cuando un barrido los lee.",
      }),
    )
  }
  if (count("error") > 0) {
    out.push(
      documentsNotice("error", count("error"), {
        kind: "problem",
        priority: "high",
        title: plural(count("error"), "documento falló al obtener su texto", "documentos fallaron al obtener su texto"),
        explanation: "El archivo se leyó, pero no se pudo extraer su texto: no se compara con otros ni se sabe a quiénes nombra.",
        todo: "Mirá el motivo de cada uno. Si el archivo está dañado o protegido, reemplazalo por una copia que se pueda abrir.",
        resolvedWhen: "Desaparece cuando el documento cambia y su texto se obtiene.",
        withReasons: true,
      }),
    )
  }
  if (count("too-large") > 0) {
    out.push(
      documentsNotice("too-large", count("too-large"), {
        kind: "problem",
        priority: "low",
        title: plural(count("too-large"), "archivo es demasiado grande", "archivos son demasiado grandes"),
        explanation: "Quedan en el inventario, pero no se leen ni se comparan con otros.",
        todo: "Si alguno importa, dividilo en partes más chicas en su carpeta.",
        resolvedWhen: "Desaparece cuando el archivo se achica o se quita.",
      }),
    )
  }
  if (count("link") > 0) {
    out.push(
      documentsNotice("link", count("link"), {
        kind: "problem",
        priority: "low",
        title: plural(
          count("link"),
          "documento de un disco en la nube quedó afuera",
          "documentos de un disco en la nube quedaron afuera",
        ),
        explanation:
          "En la carpeta solo hay un acceso directo, sin el contenido (por ejemplo, un documento nativo de Google): se detectan, pero no se pueden procesar.",
        todo: "Si alguno importa, descargalo como archivo común en la misma carpeta.",
        resolvedWhen: "Desaparece cuando el acceso directo se reemplaza por el archivo.",
      }),
    )
  }

  // ── Decisions ───────────────────────────────────────────────────────
  const similar = similarNotice(facts.similarPending, facts.groups)
  if (similar) out.push(similar)

  const proposed = facts.proposedTypes ?? []
  if (proposed.length > 0) {
    const documents = proposed.reduce((sum, type) => sum + type.documents, 0)
    out.push(
      notice({
        kind: "decision",
        topic: "document-types",
        priority: "normal",
        count: proposed.length,
        title: plural(
          proposed.length,
          "tipo de documento nuevo propuesto por el modelo",
          "tipos de documento nuevos propuestos por el modelo",
        ),
        explanation: `Abarcan ${plural(documents, "documento", "documentos")}. Hasta que una persona los confirme no entran a la lista de tipos, y el modelo no los puede elegir para otros documentos.`,
        todo: "Confirmalos, cambiales el nombre o rechazalos.",
        resolvedWhen: "Desaparece cuando cada tipo propuesto se confirma o se rechaza.",
        actions: [{ kind: "go", label: "Revisar tipos", target: { view: "document-types" } }],
        cases: proposed.map((type) => ({
          id: type.id,
          title: documentTypeName(type.id, type.name, words),
          detail: [plural(type.documents, "documento", "documentos"), type.description].filter(Boolean).join(" · "),
        })),
      }),
    )
  }

  // ── In progress ─────────────────────────────────────────────────────
  const reading = pendingCount(home.health)
  if (reading > 0) {
    out.push(
      notice({
        kind: "ongoing",
        topic: "reading",
        priority: "normal",
        count: reading,
        title: `Leyendo ${plural(reading, "archivo", "archivos")} de las carpetas incorporadas`,
        explanation: "La base identifica cada archivo, busca sus copias y obtiene su texto, una sola vez por documento.",
        todo: "No hace falta hacer nada.",
        resolvedWhen: "Desaparece cuando termina de leerlos.",
        actions: [],
      }),
    )
  }
  if (count("needs-recognition") > 0) {
    out.push(
      documentsNotice("needs-recognition", count("needs-recognition"), {
        kind: "ongoing",
        priority: "normal",
        title: plural(
          count("needs-recognition"),
          "documento tiene el texto en imagen",
          "documentos tienen el texto en imagen",
        ),
        explanation:
          "Son escaneos o fotos: su texto está en una imagen y hay que reconocerlo, una sola vez por documento.",
        todo: "Por ahora quedan sin leer: la base todavía no reconoce sola el texto de las imágenes.",
        resolvedWhen: "Desaparece cuando se reconoce su texto.",
      }),
    )
  }
  return out
}

/** A line about the documents in one state, with them one by one when it is opened. */
function documentsNotice(
  state: DocumentState,
  total: number,
  text: Pick<AttentionNotice, "kind" | "priority" | "title" | "explanation" | "todo" | "resolvedWhen"> & {
    withReasons?: boolean
  },
): AttentionNotice {
  const { withReasons, ...rest } = text
  return {
    source: DOCBASE_SOURCE,
    id: `${DOCBASE_SOURCE}:${state}`,
    topic: state,
    count: total,
    ...rest,
    actions: [
      {
        kind: "go",
        label: "Ver la lista",
        target: { view: "documents", filter: { ...DEFAULT_DOCUMENT_FILTER, state } },
      },
    ],
    moreCases: Math.max(0, total - DOCUMENT_CASES),
    loadCases: () => loadDocumentCases(state, withReasons ?? false),
  }
}

async function loadDocumentCases(state: DocumentState, withReasons: boolean): Promise<AttentionCase[]> {
  const page = await listDocuments({ state, limit: DOCUMENT_CASES, sort: "folder" })
  const reasons = withReasons
    ? await Promise.all(page.rows.map((row) => getDocument(row.locationId).then((d) => d?.error ?? null).catch(() => null)))
    : []
  return page.rows.map((row, index) => ({
    id: String(row.locationId),
    title: row.name,
    detail: [reasons[index], `${row.originName} › ${row.folder || "(raíz)"}`].filter(Boolean).join(" · "),
    actions: [{ kind: "go", label: "Abrir", target: { view: "document", locationId: row.locationId } }],
  }))
}

/**
 * Similar documents that could be the same document: one decision per
 * family (it is decided from its principal) and one per pair outside
 * families. "Same template", "contained in" and plain "similar" are not
 * escalated: they are two different documents.
 */
export function similarNotice(pending: SimilarPending | null, groups: SimilarGroup[] | null): AttentionNotice | null {
  if (!pending || pending.sameDocumentPairs === 0) return null
  const families = (groups ?? []).filter((group) => group.pendingSameDocument > 0)
  const inFamilies = families.reduce((sum, group) => sum + group.pendingSameDocument, 0)
  const loose = Math.max(0, pending.sameDocumentPairs - inFamilies)
  const decisions = families.length + loose
  const cases: AttentionCase[] = families.map((group) => ({
    id: `family:${group.principal.contentId}`,
    title: `«${group.principal.name}»`,
    detail: `${plural(group.pendingSameDocument, "documento podría ser el mismo", "documentos podrían ser el mismo")} · familia de ${formatCount(group.members.length)}`,
    actions: [{ kind: "go", label: "Resolver", target: { view: "similar", principal: group.principal.contentId } }],
  }))
  if (loose > 0) {
    cases.push({
      id: "loose",
      title: plural(loose, "par fuera de las familias", "pares fuera de las familias"),
      actions: [{ kind: "go", label: "Ver", target: { view: "similar" } }],
    })
  }
  return {
    source: DOCBASE_SOURCE,
    id: `${DOCBASE_SOURCE}:similar`,
    kind: "decision",
    topic: "similar",
    priority: "normal",
    count: decisions,
    title: plural(
      pending.sameDocumentPairs,
      "par de documentos podría ser el mismo documento",
      "pares de documentos podrían ser el mismo documento",
    ),
    explanation:
      "Prácticamente el mismo, un ejemplar firmado o una versión: si se unen, cuenta uno solo, el que elijas, y no se repite lo que dicen.",
    todo: "Unilos con la relación que corresponda, elegí otra o descartalos.",
    resolvedWhen: "Desaparece cuando cada par se une o se descarta.",
    actions: [{ kind: "go", label: "Resolver parecidos", target: { view: "similar", principal: families[0]?.principal.contentId ?? null } }],
    cases,
  }
}

/** The source itself: asks the base again whenever it says something changed. */
export function useDocbaseNotices(): AttentionNotice[] | null {
  const home = useDocbaseQuery("attention:home", getHome)
  const pending = useDocbaseQuery("attention:similar-pending", getSimilarPending)
  const groups = useDocbaseQuery("attention:similar-groups", listSimilarGroups)
  const types = useDocbaseQuery("attention:proposed-types", listProposedTypes)
  if (home.error) throw new Error(home.error)
  if (!home.data) return null
  return docbaseNotices({
    home: home.data,
    similarPending: pending.data,
    groups: groups.data,
    proposedTypes: types.data,
  })
}
