import { Fragment, useEffect, useRef, useState, type MouseEvent } from "react"
import { ChevronDown, ChevronLeft, ChevronRight, Copy } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { LongName } from "@/components/ui/long-name"
import { Table, Td, Th } from "@/components/ui/table"
import {
  documentCopies,
  listDocuments,
  listDocumentTypes,
  listLabelGroups,
  listOrigins,
  listSimilarGroups,
  type DocumentFilter,
  type DocumentRow,
  type DocumentState,
  type LocationRef,
} from "@/commands/docbase"
import { useDocbaseStore } from "@/stores/docbase-store"
import { DocumentDetailView } from "./document-detail"
import { DOCUMENT_STATES, DOCUMENT_STATE_ORDER, DocumentStateBadge } from "./document-state"
import { LabelChip } from "./labels"
import { DocbasePage, EmptyNote, ErrorNote } from "./page"
import { formatCount, formatDate, formatSize } from "./format"
import { groupLabel } from "./similar"
import { FamilyTemplateLine } from "./template-panel"
import { rowTypeLabel } from "./document-types"
import { documentTypeName, vocabulary } from "@/vocabulary"
import { useDocbaseQuery } from "./use-docbase-query"

const PAGE_SIZE = 100

// Where the table is narrower than 64rem (a window under about 1360 px),
// where a document is, when it changed, its size and its copies go under
// its name instead of in columns of their own: eight columns leave a long
// file name a strip of a few letters, or a horizontal bar. Wider, each one
// has its column.
const WIDE_ONLY = "@max-[64rem]:hidden"
const NARROW_ONLY = "hidden @max-[64rem]:block"

// A filter is never wider than its place nor than 18rem: a long option
// (the name of an origin or of a family's document) is cut inside it
// instead of pushing the screen.
const SELECT_CLASS =
  "h-8 max-w-[min(100%,18rem)] min-w-0 rounded-lg border border-border-strong bg-card px-2 text-sm outline-none focus-visible:border-link focus-visible:ring-3 focus-visible:ring-ring/30"

export function DocumentsView() {
  const openLocationId = useDocbaseStore((s) => s.openLocationId)
  const closeDocument = useDocbaseStore((s) => s.closeDocument)
  if (openLocationId !== null) {
    return <DocumentDetailView key={openLocationId} locationId={openLocationId} onBack={closeDocument} />
  }
  return <DocumentList />
}

function DocumentList() {
  const filter = useDocbaseStore((s) => s.filter)
  const setFilter = useDocbaseStore((s) => s.setFilter)
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const [typed, setTyped] = useState(filter.query ?? "")
  const [page, setPage] = useState(0)
  const sentQuery = useRef(filter.query ?? "")

  // Typing searches after a short pause, not on every key.
  useEffect(() => {
    if (typed === (filter.query ?? "")) return
    const timer = setTimeout(() => {
      sentQuery.current = typed
      setFilter({ ...filter, query: typed })
    }, 250)
    return () => clearTimeout(timer)
  }, [typed, filter, setFilter])

  // A filter set from another screen replaces what was typed here; the
  // echo of what this box just sent must not undo newer keystrokes.
  useEffect(() => {
    const query = filter.query ?? ""
    if (query !== sentQuery.current) {
      sentQuery.current = query
      setTyped(query)
    }
    setPage(0)
  }, [filter])

  const origins = useDocbaseQuery("origins", listOrigins)
  const labelGroups = useDocbaseQuery("label-groups", listLabelGroups)
  const similarGroups = useDocbaseQuery("similar-groups", listSimilarGroups)
  const documentTypes = useDocbaseQuery("document-types", listDocumentTypes)
  const words = vocabulary()
  const request: DocumentFilter = { ...filter, onePerDocument: true, limit: PAGE_SIZE, offset: page * PAGE_SIZE }
  const documents = useDocbaseQuery(JSON.stringify(request), () => listDocuments(request))

  const rows = documents.data?.rows ?? []
  const total = documents.data?.total ?? 0
  const first = total === 0 ? 0 : page * PAGE_SIZE + 1
  const last = Math.min(total, (page + 1) * PAGE_SIZE)
  const noOrigins = origins.data !== null && origins.data.length === 0

  return (
    <DocbasePage
      title="Documentos"
      subtitle="Todo lo que hay en las carpetas incorporadas, una fila por documento: en qué estado está y dónde tiene copias."
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="max-w-xs"
            title="Cada palabra tiene que estar en el nombre, en la carpeta o en el texto del documento. No distingue tildes ni mayúsculas."
            placeholder="Buscar por nombre, carpeta o texto…"
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
          />
          <select
            className={SELECT_CLASS}
            value={filter.originId ?? ""}
            onChange={(event) => setFilter({ ...filter, originId: event.target.value })}
          >
            <option value="">Todos los orígenes</option>
            {(origins.data ?? []).map((origin) => (
              <option key={origin.id} value={origin.id}>
                {origin.name}
              </option>
            ))}
          </select>
          <select
            className={SELECT_CLASS}
            value={filter.state ?? ""}
            onChange={(event) => setFilter({ ...filter, state: event.target.value as DocumentState | "" })}
          >
            <option value="">Todos los estados</option>
            {DOCUMENT_STATE_ORDER.map((state) => (
              <option key={state} value={state}>
                {DOCUMENT_STATES[state].label}
              </option>
            ))}
          </select>
          {(labelGroups.data ?? []).length > 0 && (
            <select
              className={SELECT_CLASS}
              value={filter.labelId ?? ""}
              onChange={(event) =>
                setFilter({ ...filter, labelId: event.target.value ? Number(event.target.value) : null })
              }
            >
              <option value="">Todas las etiquetas</option>
              {(labelGroups.data ?? []).map((group) => (
                <optgroup key={group.id ?? "loose"} label={group.name ?? "Sin grupo"}>
                  {group.labels.map((label) => (
                    <option key={label.id} value={label.id}>
                      {label.name} ({label.files})
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          )}
          <label className="flex items-center gap-1.5 text-sm">
            <input
              type="checkbox"
              checked={filter.duplicatesOnly ?? false}
              onChange={(event) => setFilter({ ...filter, duplicatesOnly: event.target.checked })}
            />
            Solo con copias
          </label>
          <label
            className="flex items-center gap-1.5 text-sm"
            title="Documentos que pueden ser el mismo que otro (un ejemplar firmado, una versión, uno dentro de otro…) y esperan que alguien los una o los descarte"
          >
            <input
              type="checkbox"
              checked={filter.similarPending ?? false}
              onChange={(event) => setFilter({ ...filter, similarPending: event.target.checked })}
            />
            Con una copia o versión por decidir
          </label>
          {((similarGroups.data ?? []).length > 0 || filter.similarGroup != null) && (
            <select
              className={`${SELECT_CLASS} w-64`}
              title="Familias de documentos con el mismo texto de base, por su documento principal"
              value={filter.similarGroup ?? ""}
              onChange={(event) =>
                setFilter({ ...filter, similarGroup: event.target.value ? Number(event.target.value) : null })
              }
            >
              <option value="">Todas las familias</option>
              {filter.similarGroup != null &&
                similarGroups.data !== null &&
                !similarGroups.data.some((group) => group.principal.contentId === filter.similarGroup) && (
                  <option value={filter.similarGroup}>Familia que ya no existe</option>
                )}
              {(similarGroups.data ?? []).map((group) => (
                <option key={group.principal.contentId} value={group.principal.contentId}>
                  {groupLabel(group)}
                </option>
              ))}
            </select>
          )}
          <select
            className={SELECT_CLASS}
            title="Qué tipo de documento es cada uno: lo dicen sus marcas o lo propone el modelo"
            value={filter.docType ?? ""}
            onChange={(event) => setFilter({ ...filter, docType: event.target.value || null })}
          >
            <option value="">Todos los tipos</option>
            {(documentTypes.data ?? []).map((docType) => (
              <option key={docType.id} value={docType.id}>
                {documentTypeName(docType.id, docType.name, words)}
              </option>
            ))}
            <option value="none">Sin tipo todavía</option>
          </select>
          <select
            className={`${SELECT_CLASS} ml-auto`}
            value={filter.sort ?? "recent"}
            onChange={(event) => setFilter({ ...filter, sort: event.target.value as DocumentFilter["sort"] })}
          >
            <option value="recent">Modificados primero</option>
            <option value="name">Por nombre</option>
            <option value="folder">Por carpeta</option>
          </select>
        </div>

        {documents.error && <ErrorNote>{documents.error}</ErrorNote>}

        {filter.similarGroup != null &&
          (similarGroups.data ?? [])
            .filter((group) => group.principal.contentId === filter.similarGroup)
            .map((group) => <FamilyTemplateLine key={group.principal.contentId} group={group} onOpen={openDocument} />)}

        {!documents.loading && rows.length === 0 ? (
          <EmptyNote>
            {noOrigins
              ? "Todavía no hay carpetas incorporadas. Agregá una en Orígenes."
              : "Ningún documento coincide con lo que buscás."}
          </EmptyNote>
        ) : (
          <Table>
            <thead>
              <tr>
                <Th>Nombre</Th>
                <Th className={WIDE_ONLY}>Dónde está</Th>
                <Th>Tipo</Th>
                <Th>Etiquetas</Th>
                <Th>Estado</Th>
                <Th className={WIDE_ONLY}>Copias</Th>
                <Th className={WIDE_ONLY}>Modificado</Th>
                <Th align="right" className={WIDE_ONLY}>
                  Tamaño
                </Th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <DocumentTableRow key={row.locationId} row={row} onOpen={openDocument} />
              ))}
            </tbody>
          </Table>
        )}

        {total > 0 && (
          <div className="flex items-center justify-between text-sm text-muted-foreground">
            <span>
              {formatCount(first)}–{formatCount(last)} de {formatCount(total)} {total === 1 ? "documento" : "documentos"}
            </span>
            <div className="flex items-center gap-1">
              <Button variant="outline" size="icon-sm" disabled={page === 0} onClick={() => setPage(page - 1)}>
                <ChevronLeft />
              </Button>
              <Button variant="outline" size="icon-sm" disabled={last >= total} onClick={() => setPage(page + 1)}>
                <ChevronRight />
              </Button>
            </div>
          </div>
        )}
      </div>
    </DocbasePage>
  )
}

function DocumentTableRow({ row, onOpen }: { row: DocumentRow; onOpen: (locationId: number) => void }) {
  const [copies, setCopies] = useState<LocationRef[] | null>(null)
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const toggleCopies = (event: MouseEvent) => {
    event.stopPropagation()
    if (open) {
      setOpen(false)
      return
    }
    setOpen(true)
    setError(null)
    documentCopies(row.locationId)
      .then(setCopies)
      .catch((err) => setError(String(err)))
  }

  const where = row.originName + (row.folder ? ` › ${row.folder.split("/").join(" › ")}` : "")

  return (
    <Fragment>
      <tr className="cursor-pointer" onClick={() => onOpen(row.locationId)}>
        <Td>
          <LongName className="font-semibold text-link hover:underline" title={row.name}>
            {row.name}
          </LongName>
          {/* In a narrow window, where it is, when it changed and its copies go under its name. */}
          <div className={`${NARROW_ONLY} mt-0.5 text-[0.78125rem] text-muted-foreground`}>
            <LongName title={`${row.originName} › ${row.folder}`}>{where}</LongName>
            {row.originReachable === false && " (origen sin conexión)"}
            <span className="tabular-nums">
              {" · "}
              {formatDate(row.mtimeMs)} · {formatSize(row.size)}
            </span>
          </div>
          {row.copies > 1 && (
            <div className={`${NARROW_ONLY} mt-0.5`}>
              <CopiesButton copies={row.copies} open={open} onClick={toggleCopies} words />
            </div>
          )}
        </Td>
        <Td muted className={WIDE_ONLY}>
          <LongName title={`${row.originName} › ${row.folder}`}>{where}</LongName>
          {row.originReachable === false && <span className="ml-1 text-xs">(origen sin conexión)</span>}
        </Td>
        <Td muted>{rowTypeLabel(row) ?? "—"}</Td>
        <Td>
          <div className="flex flex-wrap gap-1">
            {row.labels.map((label) => (
              <LabelChip key={label.id} label={label} />
            ))}
          </div>
        </Td>
        <Td>
          <DocumentStateBadge state={row.state} />
        </Td>
        <Td className={WIDE_ONLY}>
          {row.copies > 1 && <CopiesButton copies={row.copies} open={open} onClick={toggleCopies} />}
        </Td>
        <Td muted nowrap className={`${WIDE_ONLY} tabular-nums`}>
          {formatDate(row.mtimeMs)}
        </Td>
        <Td muted nowrap align="right" className={WIDE_ONLY}>
          {formatSize(row.size)}
        </Td>
      </tr>
      {open && (
        <tr>
          <td colSpan={8} className="border-b bg-panel-sub px-6 py-2">
            {error && <ErrorNote>{error}</ErrorNote>}
            {!error && copies === null && <span className="text-xs text-muted-foreground">Buscando las copias…</span>}
            {copies !== null && (
              <ul className="flex flex-col gap-1 text-xs">
                {copies.map((copy) => (
                  <li key={copy.locationId}>
                    <button
                      type="button"
                      className="text-left hover:underline"
                      title={copy.path}
                      onClick={() => onOpen(copy.locationId)}
                    >
                      <span className="text-link">{copy.relPath.split("/").pop()}</span>
                      <span className="text-muted-foreground">
                        {" "}— {copy.originName} › {copy.relPath.split("/").slice(0, -1).join(" › ") || "(raíz)"} ·{" "}
                        {formatDate(copy.mtimeMs)}
                        {!copy.sameFile && " · otro archivo con el mismo texto"}
                        {copy.originReachable === false && " · origen sin conexión"}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </td>
        </tr>
      )}
    </Fragment>
  )
}

/** The count of the files with this same document, that shows where they are. */
function CopiesButton({
  copies,
  open,
  onClick,
  words = false,
}: {
  copies: number
  open: boolean
  onClick: (event: MouseEvent) => void
  /** «en 3 archivos» instead of the bare count, under the name. */
  words?: boolean
}) {
  return (
    <button
      type="button"
      className="inline-flex items-center gap-1 rounded px-1 text-xs tabular-nums hover:bg-hover"
      title={open ? "Ocultar las copias" : `Este mismo documento está en ${copies} archivos: ver dónde`}
      aria-expanded={open}
      onClick={onClick}
    >
      <Copy className="size-3" /> {words ? `en ${formatCount(copies)} archivos` : copies}
      <ChevronDown className={`size-3 transition-transform ${open ? "rotate-180" : ""}`} />
    </button>
  )
}
