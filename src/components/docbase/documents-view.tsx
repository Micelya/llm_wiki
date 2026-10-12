import { useEffect, useRef, useState } from "react"
import { ChevronLeft, ChevronRight, Copy } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  listDocuments,
  listOrigins,
  type DocumentFilter,
  type DocumentRow,
  type DocumentState,
} from "@/commands/docbase"
import { useDocbaseStore } from "@/stores/docbase-store"
import { DocumentDetailView } from "./document-detail"
import { DOCUMENT_STATES, DOCUMENT_STATE_ORDER, DocumentStateBadge } from "./document-state"
import { DocbasePage, EmptyNote, ErrorNote } from "./page"
import { formatCount, formatDate, formatSize } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

const PAGE_SIZE = 100

const SELECT_CLASS =
  "h-8 rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"

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
  const request: DocumentFilter = { ...filter, limit: PAGE_SIZE, offset: page * PAGE_SIZE }
  const documents = useDocbaseQuery(JSON.stringify(request), () => listDocuments(request))

  const rows = documents.data?.rows ?? []
  const total = documents.data?.total ?? 0
  const first = total === 0 ? 0 : page * PAGE_SIZE + 1
  const last = Math.min(total, (page + 1) * PAGE_SIZE)
  const noOrigins = origins.data !== null && origins.data.length === 0

  return (
    <DocbasePage
      title="Documentos"
      subtitle="Todo lo que hay en las carpetas incorporadas: qué documento es cada archivo, en qué estado está y dónde tiene copias."
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
          <label className="flex items-center gap-1.5 text-sm">
            <input
              type="checkbox"
              checked={filter.duplicatesOnly ?? false}
              onChange={(event) => setFilter({ ...filter, duplicatesOnly: event.target.checked })}
            />
            Solo con copias
          </label>
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

        {!documents.loading && rows.length === 0 ? (
          <EmptyNote>
            {noOrigins
              ? "Todavía no hay carpetas incorporadas. Agregá una en Orígenes."
              : "Ningún documento coincide con lo que buscás."}
          </EmptyNote>
        ) : (
          <div className="overflow-x-auto rounded-lg border">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="px-3 py-2 font-medium">Nombre</th>
                  <th className="px-3 py-2 font-medium">Dónde está</th>
                  <th className="px-3 py-2 font-medium">Etiquetas</th>
                  <th className="px-3 py-2 font-medium">Estado</th>
                  <th className="px-3 py-2 font-medium">Copias</th>
                  <th className="px-3 py-2 font-medium">Modificado</th>
                  <th className="px-3 py-2 text-right font-medium">Tamaño</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <DocumentTableRow key={row.locationId} row={row} onOpen={() => openDocument(row.locationId)} />
                ))}
              </tbody>
            </table>
          </div>
        )}

        {total > 0 && (
          <div className="flex items-center justify-between text-sm text-muted-foreground">
            <span>
              {formatCount(first)}–{formatCount(last)} de {formatCount(total)}
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

function DocumentTableRow({ row, onOpen }: { row: DocumentRow; onOpen: () => void }) {
  const labels = Object.entries(row.labels)
  return (
    <tr className="cursor-pointer border-t hover:bg-muted/40" onClick={onOpen}>
      <td className="max-w-xs px-3 py-2">
        <span className="font-medium text-primary hover:underline" title={row.name}>
          {row.name}
        </span>
      </td>
      <td className="max-w-xs px-3 py-2 text-muted-foreground">
        <span className="break-words" title={`${row.originName} › ${row.folder}`}>
          {row.originName}
          {row.folder && ` › ${row.folder.split("/").join(" › ")}`}
        </span>
        {row.originReachable === false && <span className="ml-1 text-xs">(origen sin conexión)</span>}
      </td>
      <td className="px-3 py-2">
        <div className="flex flex-wrap gap-1">
          {labels.map(([name, value]) => (
            <span key={name} className="rounded bg-muted px-1.5 py-0.5 text-xs" title={name}>
              <span className="text-muted-foreground">{name}:</span> {value}
            </span>
          ))}
        </div>
      </td>
      <td className="px-3 py-2">
        <DocumentStateBadge state={row.state} />
      </td>
      <td className="px-3 py-2">
        {row.copies > 1 && (
          <span
            className="inline-flex items-center gap-1 text-xs"
            title={`Este mismo documento está en ${row.copies} lugares`}
          >
            <Copy className="size-3" /> {row.copies}
          </span>
        )}
      </td>
      <td className="px-3 py-2 whitespace-nowrap text-muted-foreground">{formatDate(row.mtimeMs)}</td>
      <td className="px-3 py-2 text-right whitespace-nowrap text-muted-foreground">{formatSize(row.size)}</td>
    </tr>
  )
}
