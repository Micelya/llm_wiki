import { useState, type FormEvent } from "react"
import { Plus, Search } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  chooseMatterDocument,
  createMatter,
  listDocuments,
  listMatters,
  type DocumentMatter,
  type DocumentRow,
  type MatterSummary,
} from "@/commands/docbase"
import { vocabulary } from "@/vocabulary"
import { ErrorNote } from "@/components/docbase/page"
import { mattersToJoin, pickableDocuments } from "./knowledge"
import { useAction, useOpenMatter } from "./shared"

/**
 * Making and editing a matter by hand (D-386): a name, and a search of
 * the documents (the one of Documentos) to put them in.
 */

/** How many documents a search shows. */
const SEARCH_LIMIT = 8

/** A one-line form for a name. */
export function NameForm({
  initial = "",
  placeholder,
  saveLabel,
  busy,
  onSave,
  onCancel,
}: {
  initial?: string
  placeholder: string
  saveLabel: string
  busy: boolean
  onSave: (name: string) => void
  onCancel: () => void
}) {
  const [name, setName] = useState(initial)
  function submit(event: FormEvent) {
    event.preventDefault()
    if (name.trim()) onSave(name.trim())
  }
  return (
    <form className="flex flex-wrap items-center gap-2" onSubmit={submit}>
      <Input
        autoFocus
        className="max-w-md flex-1"
        value={name}
        placeholder={placeholder}
        onChange={(event) => setName(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && onCancel()}
      />
      <Button type="submit" size="sm" disabled={busy || !name.trim()}>
        {saveLabel}
      </Button>
      <Button type="button" size="sm" variant="ghost" onClick={onCancel}>
        Cancelar
      </Button>
    </form>
  )
}

/** Looks for documents by name, path or text, and offers to put each in. */
export function DocumentPicker({
  already,
  busy,
  onPick,
}: {
  already: number[]
  busy: boolean
  onPick: (row: DocumentRow) => void
}) {
  const [query, setQuery] = useState("")
  const [found, setFound] = useState<DocumentRow[] | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [searching, setSearching] = useState(false)
  async function search(event: FormEvent) {
    event.preventDefault()
    if (!query.trim()) return
    setSearching(true)
    setProblem(null)
    try {
      const page = await listDocuments({ query: query.trim(), onePerDocument: true, sort: "name", limit: SEARCH_LIMIT * 2 })
      setFound(page.rows)
    } catch (cause) {
      setProblem(String((cause as Error)?.message ?? cause))
    } finally {
      setSearching(false)
    }
  }
  const rows = found ? pickableDocuments(found, already).slice(0, SEARCH_LIMIT) : []
  return (
    <div className="flex flex-col gap-2">
      <form className="flex items-center gap-2" onSubmit={search}>
        <Input
          className="max-w-md flex-1"
          value={query}
          placeholder="Buscar un documento por su nombre, su carpeta o su texto"
          onChange={(event) => setQuery(event.target.value)}
        />
        <Button type="submit" size="sm" variant="outline" disabled={searching || !query.trim()}>
          <Search /> Buscar
        </Button>
      </form>
      {problem && <ErrorNote>{problem}</ErrorNote>}
      {found && rows.length === 0 && <p className="text-sm text-muted-foreground">No hay otros documentos con esas palabras.</p>}
      {rows.length > 0 && (
        <ul className="flex flex-col divide-y rounded-md border text-sm">
          {rows.map((row) => (
            <li key={row.contentId} className="flex items-center gap-3 px-3 py-1.5">
              <span className="min-w-0 flex-1">
                <span className="break-words">{row.name}</span>
                <span className="block truncate text-xs text-muted-foreground">{row.folder || row.originName}</span>
              </span>
              <Button size="xs" variant="outline" disabled={busy} onClick={() => onPick(row)}>
                <Plus /> Agregar
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

/**
 * From the detail of a document: put it in a matter that exists, or
 * create one with it.
 */
export function AddToMatter({ locationId, current }: { locationId: number; current: DocumentMatter[] }) {
  const words = vocabulary()
  const name = words.knowledge.matter.toLowerCase()
  const action = useAction()
  const openMatter = useOpenMatter()
  const [mode, setMode] = useState<"closed" | "existing" | "new">("closed")
  const [matters, setMatters] = useState<MatterSummary[] | null>(null)
  const [query, setQuery] = useState("")
  const reference = `location:${locationId}`
  const currentIds = current.filter((m) => !m.proposed).map((m) => m.matterId)

  async function chooseExisting() {
    setMode("existing")
    await action.run(async () => setMatters(await listMatters()))
  }
  const options = matters ? mattersToJoin(matters, currentIds, query).slice(0, SEARCH_LIMIT) : []

  return (
    <div className="mb-3 flex flex-col gap-2 text-sm">
      {mode === "closed" && (
        <div className="flex flex-wrap gap-2">
          <Button size="xs" variant="outline" onClick={() => void chooseExisting()}>
            <Plus /> Agregar a un {name}
          </Button>
          <Button size="xs" variant="ghost" onClick={() => setMode("new")}>
            Nuevo {name} con este documento
          </Button>
        </div>
      )}
      {mode === "new" && (
        <NameForm
          placeholder={`Nombre del ${name}`}
          saveLabel={`Crear ${name}`}
          busy={action.busy}
          onCancel={() => setMode("closed")}
          onSave={(value) =>
            void action.run(async () => {
              const id = await createMatter(value, [reference])
              setMode("closed")
              openMatter(id)
            })
          }
        />
      )}
      {mode === "existing" && (
        <div className="flex flex-col gap-2">
          <div className="flex items-center gap-2">
            <Input
              autoFocus
              className="max-w-md flex-1"
              value={query}
              placeholder={`Buscar un ${name} por su nombre`}
              onChange={(event) => setQuery(event.target.value)}
            />
            <Button size="sm" variant="ghost" onClick={() => setMode("closed")}>
              Cerrar
            </Button>
          </div>
          {matters && options.length === 0 && <p className="text-muted-foreground">No hay otro {name} con ese nombre.</p>}
          {options.length > 0 && (
            <ul className="flex flex-col divide-y rounded-md border">
              {options.map((matter) => (
                <li key={matter.id} className="flex items-center gap-3 px-3 py-1.5">
                  <span className="min-w-0 flex-1 break-words">{matter.name}</span>
                  <Button
                    size="xs"
                    variant="outline"
                    disabled={action.busy}
                    onClick={() =>
                      void action.run(async () => {
                        await chooseMatterDocument(matter.id, reference, "in")
                        setMode("closed")
                      })
                    }
                  >
                    Agregar
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
      {action.problem && <ErrorNote>{action.problem}</ErrorNote>}
    </div>
  )
}
