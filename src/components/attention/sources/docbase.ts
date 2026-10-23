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
import { listNewClips, markClipsSeen, openWorkFolder, type NewFiles } from "@/commands/work-folder"
import { deleteCoreLeftReadings, listCoreLeftReadings, type CoreLeftReading } from "@/commands/micelya-core"
import { DEFAULT_DOCUMENT_FILTER } from "@/stores/docbase-store"
import type { AttentionCase, AttentionNotice } from "@/lib/attention/types"
import { documentTypeName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatAgo, formatCount, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"

/**
 * What the document base raises: origins that cannot be read, files and
 * documents that could not be processed, similar documents that could be
 * the same one, document types the model proposed, and the web clips saved
 * since a person last looked at them (D-450). All of it is about
 * the archive (the space), whatever project is open. How far it went
 * reading is in the stages of «En curso» (`progress`, D-424).
 */
export const DOCBASE_SOURCE = "docbase"

/** Cases listed when a line of documents is opened; the rest are in Documentos. */
export const DOCUMENT_CASES = 20

export interface DocbaseFacts {
  home: Home
  similarPending: SimilarPending | null
  groups: SimilarGroup[] | null
  proposedTypes: ProposedType[] | null
  /** The clips of «Recortes web» a person has not seen yet. */
  newClips?: NewFiles | null
  /** The readings of this computer that Micelya Core refused to delete (P-294). */
  coreLeft?: CoreLeftReading[] | null
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
  const coreLeft = coreLeftNotice(facts.coreLeft)
  if (coreLeft) out.push(coreLeft)

  // ── Decisions ───────────────────────────────────────────────────────
  const similar = similarNotice(facts.similarPending, facts.groups, words)
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

  const clips = clipsNotice(facts.newClips)
  if (clips) out.push(clips)

  // What is in progress (reading the files, their text, the text of the
  // images) is in the stages of «En curso» (D-424), not here.
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
 * The documents that could be one same document (D-497): a signed copy, a
 * version, one inside the other, a delivery receipt, a signing
 * certificate, and two of one template where no datum changes (P-218).
 * One decision per family (it is decided from its principal) and one per
 * pair outside families. Two documents made with one template, or that
 * only share some text, are different documents and are not escalated.
 */
export function similarNotice(
  pending: SimilarPending | null,
  groups: SimilarGroup[] | null,
  words: Vocabulary = vocabulary(),
): AttentionNotice | null {
  const possiblySame = (groups ?? []).reduce((sum, group) => sum + group.possiblySame, 0)
  if (!pending || pending.pairs + possiblySame === 0) return null
  const families = (groups ?? []).filter((group) => group.pending > 0 || group.possiblySame > 0)
  const inFamilies = families.reduce((sum, group) => sum + group.pending, 0)
  const loose = Math.max(0, pending.pairs - inFamilies)
  const decisions = families.length + loose
  const cases: AttentionCase[] = families.map((group) => ({
    id: `family:${group.principal.contentId}`,
    title: `«${group.principal.name}»`,
    detail: [
      group.pending > 0 ? plural(group.pending, "par por decidir", "pares por decidir") : null,
      group.possiblySame > 0
        ? plural(group.possiblySame, "par de la misma plantilla no cambia en ningún dato", "pares de la misma plantilla no cambian en ningún dato")
        : null,
      `familia de ${formatCount(group.members.length)}`,
    ]
      .filter(Boolean)
      .join(" · "),
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
      pending.pairs + possiblySame,
      "par de documentos podría ser el mismo documento",
      "pares de documentos podrían ser el mismo documento",
    ),
    explanation:
      "Un ejemplar firmado, otra versión, uno dentro de otro, un acuse de recibo, una constancia de firma o dos de la misma plantilla en los que no cambia ningún dato. Si son el mismo documento y se unen, cuenta uno solo, el que elijas, y no se repite lo que dicen. Los hechos con la misma plantilla y otros datos son documentos distintos y no piden nada.",
    todo: "Unilos con la relación que corresponda, elegí otra o descartalos.",
    resolvedWhen: "Desaparece cuando cada par se une o se descarta.",
    actions: [
      {
        kind: "go",
        label: `Resolver en «${words.copies.name}»`,
        target: { view: "similar", principal: families[0]?.principal.contentId ?? null },
      },
    ],
    cases,
  }
}

/**
 * The web clips saved with Micelya Clipper since a person last looked at
 * them: they are already documents of the base, in «Recortes web» of the
 * work folder; the line is there so someone sees what came in.
 */
export function clipsNotice(clips: NewFiles | null | undefined): AttentionNotice | null {
  if (!clips || clips.total === 0) return null
  return {
    source: DOCBASE_SOURCE,
    id: `${DOCBASE_SOURCE}:clips`,
    kind: "decision",
    topic: "clips",
    priority: "normal",
    count: clips.total,
    title: plural(clips.total, "recorte web nuevo", "recortes web nuevos"),
    explanation:
      "Se guardaron con Micelya Clipper en «Recortes web», en la carpeta de trabajo, y la base los lee como cualquier documento.",
    todo: "Miralos. Si alguno no sirve, borralo de la carpeta.",
    resolvedWhen: "Desaparece cuando tocás «Ya los vi»; vuelve con el próximo recorte.",
    actions: [
      { kind: "run", label: "Ya los vi", run: () => markClipsSeen() },
      { kind: "run", label: "Abrir la carpeta", run: () => openWorkFolder("clips") },
    ],
    cases: clips.files.map((file) => ({
      id: String(file.locationId),
      title: file.name.replace(/\.md$/i, ""),
      detail: `guardado ${formatAgo(file.firstSeenAt)}`,
      actions: [{ kind: "go", label: "Abrir", target: { view: "document", locationId: file.locationId } }],
    })),
    moreCases: Math.max(0, clips.total - clips.files.length),
  }
}

/**
 * The readings this computer asked for in Micelya Core that Core refused
 * to delete for something that is not a cut, such as a permission the
 * credential lacks (P-294): Core keeps their text until they are deleted.
 * The app tries again by itself every few minutes.
 */
export function coreLeftNotice(left: CoreLeftReading[] | null | undefined, now: number = Date.now()): AttentionNotice | null {
  if (!left || left.length === 0) return null
  return {
    source: DOCBASE_SOURCE,
    id: `${DOCBASE_SOURCE}:core-readings`,
    kind: "problem",
    topic: "core-readings",
    priority: "high",
    count: left.length,
    title: plural(left.length, "lectura no se pudo borrar en Micelya Core", "lecturas no se pudieron borrar en Micelya Core"),
    explanation:
      "Micelya Core sigue guardando esas lecturas, con el texto de sus páginas, hasta que se borren. Solo se borran las que pidió esta computadora.",
    todo: "Mirá el motivo de cada una. Si es un permiso, revisá con quien administra Micelya Core que la credencial de Ajustes → Conexiones pueda borrar lecturas. La app lo vuelve a intentar sola cada 10 minutos.",
    resolvedWhen: "Desaparece cuando se borran.",
    actions: [
      { kind: "run", label: "Reintentar ahora", run: () => deleteCoreLeftReadings() },
      { kind: "go", label: "Ver conexiones", target: { view: "settings" } },
    ],
    cases: left.map((reading) => ({
      id: reading.id,
      title: reading.name,
      detail: [reading.refused, `pedida ${formatAgo(reading.askedAt, now)}`].filter(Boolean).join(" · "),
    })),
  }
}

/** The source itself: asks the base again whenever it says something changed. */
export function useDocbaseNotices(): AttentionNotice[] | null {
  const home = useDocbaseQuery("attention:home", getHome)
  const pending = useDocbaseQuery("attention:similar-pending", getSimilarPending)
  const groups = useDocbaseQuery("attention:similar-groups", listSimilarGroups)
  const types = useDocbaseQuery("attention:proposed-types", listProposedTypes)
  const clips = useDocbaseQuery("attention:new-clips", listNewClips)
  const coreLeft = useDocbaseQuery("attention:core-left", listCoreLeftReadings)
  if (home.error) throw new Error(home.error)
  if (!home.data) return null
  return docbaseNotices({
    home: home.data,
    similarPending: pending.data,
    groups: groups.data,
    proposedTypes: types.data,
    newClips: clips.data,
    coreLeft: coreLeft.data,
  })
}
