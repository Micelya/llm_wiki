import { useEffect, useState } from "react"
import { FileText, Search, X } from "lucide-react"
import type { ChatDocument } from "@/lib/chat-documents"

/** How long the search waits for the person to stop typing. */
export const DOCUMENT_SEARCH_DELAY_MS = 200

export type SearchDocuments = (query: string, attached: ChatDocument[]) => Promise<ChatDocument[]>

/**
 * The documents of the base the search finds for `query` (P-243), asked
 * once the person stops typing; `null` while there is nothing to search
 * (`query` is null) or before the first answer.
 */
export function useDocumentSearch(query: string | null, search: SearchDocuments, attached: ChatDocument[]): ChatDocument[] | null {
  const [found, setFound] = useState<ChatDocument[] | null>(null)
  const attachedKey = attached.map((document) => document.locationId).join(",")
  useEffect(() => {
    if (query === null) {
      setFound(null)
      return
    }
    let current = true
    const timer = setTimeout(() => {
      search(query, attached)
        .then((documents) => current && setFound(documents))
        .catch(() => current && setFound([]))
    }, DOCUMENT_SEARCH_DELAY_MS)
    return () => {
      current = false
      clearTimeout(timer)
    }
    // The documents attached are compared by their key, not by object.
  }, [query, search, attachedKey])
  return found
}

/** The documents found, each with its name and where it is. */
export function DocumentOptions({
  options,
  activeIndex = -1,
  onPick,
}: {
  options: ChatDocument[]
  activeIndex?: number
  onPick: (document: ChatDocument) => void
}) {
  return (
    <div className="max-h-64 overflow-y-auto" role="listbox" aria-label="Documentos de la base">
      {options.map((document, index) => (
        <button
          key={document.locationId}
          type="button"
          role="option"
          aria-selected={index === activeIndex}
          onMouseDown={(event) => {
            event.preventDefault()
            onPick(document)
          }}
          className={`flex w-full items-start gap-2 rounded px-2 py-1.5 text-left text-xs ${index === activeIndex ? "bg-accent" : "hover:bg-accent/60"}`}
        >
          <FileText className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          <span className="min-w-0 flex-1">
            <span className="block truncate font-medium">{document.name}</span>
            <span className="block truncate text-muted-foreground">{document.place}</span>
          </span>
        </button>
      ))}
    </div>
  )
}

/**
 * The search box to attach a document of the base to the message: by
 * words of its name, its folder or its text; empty, the most recent.
 */
export function DocumentPicker({
  query,
  options,
  onQuery,
  onPick,
  onClose,
}: {
  query: string
  /** `null` while the first answer is coming. */
  options: ChatDocument[] | null
  onQuery: (query: string) => void
  onPick: (document: ChatDocument) => void
  onClose: () => void
}) {
  return (
    <div className="absolute bottom-[4.75rem] left-4 right-4 z-30 max-w-xl rounded-md border border-border bg-popover p-1.5 text-popover-foreground shadow-lg">
      <div className="px-2 pb-1 text-[11px] font-medium text-muted-foreground">
        Adjuntar un documento de la base: el chat lee su texto y lo cita.
      </div>
      <div className="mb-1 flex items-center gap-2 rounded border border-border/70 px-2">
        <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        <input
          autoFocus
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault()
              onClose()
            } else if (event.key === "Enter" && options && options.length > 0) {
              event.preventDefault()
              onPick(options[0])
            }
          }}
          placeholder="Buscá por el nombre, la carpeta o lo que dice"
          aria-label="Buscar un documento de la base"
          className="h-7 min-w-0 flex-1 bg-transparent text-xs focus:outline-none"
        />
        <button type="button" onClick={onClose} className="rounded p-0.5 text-muted-foreground hover:text-foreground" title="Cerrar">
          <X className="h-3 w-3" />
        </button>
      </div>
      {options === null ? (
        <p className="px-2 py-1.5 text-xs text-muted-foreground">Buscando…</p>
      ) : options.length === 0 ? (
        <p className="px-2 py-1.5 text-xs text-muted-foreground">
          {query.trim() ? "Ningún documento con texto coincide con lo que escribiste." : "Todavía no hay documentos con texto en la base."}
        </p>
      ) : (
        <DocumentOptions options={options} onPick={onPick} />
      )}
    </div>
  )
}

/** A document attached: its name, where it is on hover, and how to take it out. */
export function DocumentChip({ document, onRemove }: { document: ChatDocument; onRemove?: () => void }) {
  return (
    <span
      className="group mt-1 inline-flex h-7 max-w-[18rem] items-center gap-1.5 rounded-md border border-blue-500/25 bg-blue-500/10 px-2 text-xs font-medium text-blue-700 dark:text-blue-300"
      title={`${document.name} · ${document.place}`}
    >
      <FileText className="h-3 w-3 shrink-0" />
      <span className="truncate">{document.name}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="rounded p-0.5 opacity-70 hover:bg-blue-500/15 group-hover:opacity-100"
          title={`Quitar «${document.name}» del mensaje`}
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </span>
  )
}
