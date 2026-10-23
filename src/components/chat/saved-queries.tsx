import { useState } from "react"
import { Bookmark, ChevronDown, ChevronRight, Trash2, X } from "lucide-react"
import { deleteSavedQuery, listSavedQueries, type SavedQuery } from "@/commands/docbase"
import { formatAgo } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { WikiReader } from "@/components/editor/wiki-reader"
import { openWikiPath } from "@/lib/open-wiki-path"
import { useAppDialog } from "@/stores/app-dialog-store"

/**
 * «Consultas guardadas»: the questions of the chat a person saved, with
 * their answers, kept in the base (D-413). Each one opens to read it, and
 * what it cites opens the screen of what it names.
 */
export function SavedQueriesList() {
  const queries = useDocbaseQuery("saved-queries", listSavedQueries)
  const appDialog = useAppDialog()
  const [open, setOpen] = useState(false)
  const [shown, setShown] = useState<SavedQuery | null>(null)
  const list = queries.data ?? []

  async function remove(query: SavedQuery) {
    const confirmed = await appDialog.confirm({
      title: "Borrar la consulta guardada",
      message: `¿Borrar «${query.title}»? No se puede deshacer.`,
      confirmLabel: "Borrar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (!confirmed) return
    await deleteSavedQuery(query.id)
    if (shown?.id === query.id) setShown(null)
    queries.reload()
  }

  return (
    <div className="border-t">
      <button
        type="button"
        className="flex w-full items-center gap-1.5 px-3 py-2 text-xs font-medium text-muted-foreground hover:text-foreground"
        onClick={() => {
          // The agent saves queries too, without a change of the base on the screen.
          if (!open) queries.reload()
          setOpen(!open)
        }}
      >
        {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        <Bookmark className="h-3 w-3" />
        Consultas guardadas
        {list.length > 0 && <span className="ml-auto">{list.length}</span>}
      </button>
      {open && (
        <div className="max-h-56 overflow-y-auto pb-1">
          {list.length === 0 ? (
            <p className="px-3 py-2 text-[11px] text-muted-foreground">
              Todavía no guardaste ninguna. «Guardar consulta», debajo de una respuesta, la guarda acá.
            </p>
          ) : (
            list.map((query) => (
              <div
                key={query.id}
                className="group mx-1 my-0.5 flex cursor-pointer items-start gap-1 rounded-md px-2 py-1 text-xs hover:bg-accent"
                onClick={() => setShown(query)}
              >
                <span className="line-clamp-2 flex-1">{query.title}</span>
                <button
                  type="button"
                  title="Borrar"
                  className="invisible shrink-0 rounded p-0.5 text-muted-foreground hover:text-destructive group-hover:visible"
                  onClick={(event) => {
                    event.stopPropagation()
                    void remove(query)
                  }}
                >
                  <Trash2 className="h-3 w-3" />
                </button>
              </div>
            ))
          )}
        </div>
      )}
      {shown && <SavedQueryDialog query={shown} onClose={() => setShown(null)} onDelete={() => void remove(shown)} />}
    </div>
  )
}

function SavedQueryDialog({ query, onClose, onDelete }: { query: SavedQuery; onClose: () => void; onDelete: () => void }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-6" onClick={onClose}>
      <div
        className="flex max-h-full w-full max-w-3xl flex-col rounded-lg border bg-background shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="flex items-start justify-between gap-3 border-b px-5 py-3">
          <div className="min-w-0">
            <h2 className="font-serif text-lg break-words">{query.title}</h2>
            <p className="text-xs text-muted-foreground">Guardada {formatAgo(query.createdAt)}</p>
          </div>
          <div className="flex shrink-0 gap-1">
            <button type="button" title="Borrar" className="rounded p-1 text-muted-foreground hover:text-destructive" onClick={onDelete}>
              <Trash2 className="h-4 w-4" />
            </button>
            <button type="button" title="Cerrar" className="rounded p-1 text-muted-foreground hover:text-foreground" onClick={onClose}>
              <X className="h-4 w-4" />
            </button>
          </div>
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4 text-sm">
          {query.question.trim() && (
            <p className="mb-3 rounded-md bg-muted/50 px-3 py-2">
              <span className="font-medium">Pregunta: </span>
              {query.question}
            </p>
          )}
          <WikiReader body={query.answer} />
          {query.refs.length > 0 && (
            <div className="mt-4 border-t pt-2">
              <p className="mb-1 text-xs font-medium text-muted-foreground">Lo que cita</p>
              <ul className="flex flex-wrap gap-1.5">
                {query.refs.map((ref) => (
                  <li key={ref}>
                    <button
                      type="button"
                      className="rounded border px-2 py-0.5 text-xs hover:bg-accent"
                      onClick={() => {
                        void openWikiPath(ref).then((opened) => opened && onClose())
                      }}
                    >
                      {refLabel(ref)}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

/** A reference as a person reads it: the name of its page, without folders or extension. */
export function refLabel(ref: string): string {
  const name = ref.replace(/\\/g, "/").split("/").pop() ?? ref
  return name.replace(/\.md$/i, "").replace(/-/g, " ")
}
