import { useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { FolderPlus, RefreshCw, Trash2, Pencil, Check, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { useAppDialog } from "@/stores/app-dialog-store"
import {
  addOrigin,
  getGeneralProject,
  listOrigins,
  previewFolder,
  removeOrigin,
  rescanOrigins,
  setOriginConvention,
  type FolderPreview,
  type OriginSummary,
} from "@/commands/docbase"
import { GeneralProjectPanel } from "./general-project-panel"
import { DocbasePage, EmptyNote, ErrorNote, Panel, StatusBadge } from "./page"
import { formatAgo, parseConvention, plural } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"
import { vocabulary } from "@/vocabulary"

interface Draft {
  folder: string
  name: string
  convention: string
  preview: FolderPreview
}

export function OriginsView() {
  const origins = useDocbaseQuery("origins", listOrigins)
  const dialog = useAppDialog()
  const [draft, setDraft] = useState<Draft | null>(null)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  async function pickFolder() {
    setProblem(null)
    const selected = await open({ directory: true, multiple: false, title: "Elegí la carpeta a incorporar" })
    if (typeof selected !== "string") return
    try {
      const preview = await previewFolder(selected)
      setDraft({ folder: selected, name: preview.name, convention: "", preview })
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  async function confirmDraft() {
    if (!draft) return
    // With the wiki active, a new folder goes to the model by itself.
    const general = await getGeneralProject().catch(() => null)
    if (general?.wikiActive) {
      const confirmed = await dialog.confirm({
        title: `Incorporar "${draft.name}"`,
        message:
          `El ${vocabulary().archive.wiki.toLowerCase()} está activo: todos los documentos de esta carpeta se van a incorporar al wiki solos, ` +
          "y eso usa el modelo configurado. Si es una carpeta grande, conviene pausar el wiki antes.",
        confirmLabel: "Incorporar",
        cancelLabel: "Cancelar",
      })
      if (!confirmed) return
    }
    setBusy(true)
    setProblem(null)
    try {
      await addOrigin(draft.name, draft.folder, parseConvention(draft.convention))
      setDraft(null)
      origins.reload()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  async function handleRemove(origin: OriginSummary) {
    const confirmed = await dialog.confirm({
      title: `Quitar "${origin.name}"`,
      message:
        "La base deja de vigilar esta carpeta y olvida lo que sabía de sus archivos. La carpeta y sus archivos no se tocan.",
      confirmLabel: "Quitar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (!confirmed) return
    try {
      await removeOrigin(origin.id)
      origins.reload()
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  async function handleRescan() {
    setProblem(null)
    try {
      await rescanOrigins()
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  const list = origins.data ?? []

  return (
    <DocbasePage
      title="Orígenes"
      subtitle="Las carpetas que forman tu base documental. Solo se leen: nunca se modifican, se mueven ni se copian."
      actions={
        <>
          <Button variant="outline" onClick={handleRescan} disabled={list.length === 0}>
            <RefreshCw /> Volver a barrer
          </Button>
          <Button onClick={pickFolder}>
            <FolderPlus /> Agregar carpeta
          </Button>
        </>
      }
    >
      <div className="mx-auto flex max-w-4xl flex-col gap-4">
        {(problem || origins.error) && <ErrorNote>{problem ?? origins.error}</ErrorNote>}

        {draft && (
          <NewOriginForm
            draft={draft}
            busy={busy}
            onChange={setDraft}
            onCancel={() => setDraft(null)}
            onConfirm={confirmDraft}
          />
        )}

        {!origins.loading && list.length === 0 && !draft && (
          <Panel>
            <EmptyNote>
              Todavía no hay carpetas incorporadas. Agregá la primera para que la base empiece a inventariar
              tus documentos.
            </EmptyNote>
          </Panel>
        )}

        <GeneralProjectPanel />

        {list.map((origin) => (
          <OriginCard
            key={origin.id}
            origin={origin}
            onRemove={() => handleRemove(origin)}
            onSaved={origins.reload}
            onProblem={setProblem}
          />
        ))}
      </div>
    </DocbasePage>
  )
}

function NewOriginForm({
  draft,
  busy,
  onChange,
  onCancel,
  onConfirm,
}: {
  draft: Draft
  busy: boolean
  onChange: (draft: Draft) => void
  onCancel: () => void
  onConfirm: () => void
}) {
  const levels = parseConvention(draft.convention)
  return (
    <Panel title="Nueva carpeta">
      <div className="flex flex-col gap-4">
        <div>
          <div className="text-xs font-medium text-muted-foreground">Carpeta</div>
          <div className="mt-1 font-mono text-sm break-all">{draft.folder}</div>
        </div>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-medium text-muted-foreground">Nombre</span>
          <Input value={draft.name} onChange={(event) => onChange({ ...draft, name: event.target.value })} />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-medium text-muted-foreground">
            ¿Cómo está ordenada? Nombrá cada nivel de carpetas, de afuera hacia adentro (opcional)
          </span>
          <Input
            value={draft.convention}
            placeholder="cliente, caso"
            onChange={(event) => onChange({ ...draft, convention: event.target.value })}
          />
          <span className="text-xs text-muted-foreground">
            Cada nivel pasa a ser una etiqueta de los documentos. Se puede cambiar después, y el valor de un
            documento mal guardado se puede corregir.
          </span>
        </label>
        {draft.preview.examples.length > 0 && (
          <ConventionExamples examples={draft.preview.examples} levels={levels} />
        )}
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onCancel} disabled={busy}>
            Cancelar
          </Button>
          <Button onClick={onConfirm} disabled={busy || draft.name.trim().length === 0}>
            Incorporar
          </Button>
        </div>
      </div>
    </Panel>
  )
}

/** Shows real folders of the origin next to the label each level would get. */
function ConventionExamples({ examples, levels }: { examples: string[][]; levels: string[] }) {
  return (
    <div className="rounded-md bg-muted/50 px-3 py-2">
      <div className="text-xs font-medium text-muted-foreground">Así quedarían algunas de sus carpetas</div>
      <ul className="mt-1.5 flex flex-col gap-1 text-sm">
        {examples.map((path) => (
          <li key={path.join("/")} className="flex flex-wrap items-center gap-x-1.5 gap-y-1">
            {path.map((folder, index) => (
              <span key={index} className="flex items-center gap-1.5">
                {index > 0 && <span className="text-muted-foreground">›</span>}
                {levels[index] && <span className="text-xs text-muted-foreground">{levels[index]}:</span>}
                <span className={levels[index] ? "font-medium" : "text-muted-foreground"}>{folder}</span>
              </span>
            ))}
          </li>
        ))}
      </ul>
    </div>
  )
}

function OriginCard({
  origin,
  onRemove,
  onSaved,
  onProblem,
}: {
  origin: OriginSummary
  onRemove: () => void
  onSaved: () => void
  onProblem: (problem: string | null) => void
}) {
  const [editing, setEditing] = useState<string | null>(null)

  async function saveConvention() {
    if (editing === null) return
    try {
      await setOriginConvention(origin.id, parseConvention(editing))
      setEditing(null)
      onSaved()
    } catch (cause) {
      onProblem(String(cause))
    }
  }

  return (
    <Panel
      title={origin.name}
      action={
        <Button variant="ghost" size="icon-sm" onClick={onRemove} title="Quitar de la base">
          <Trash2 />
        </Button>
      }
    >
      <div className="flex flex-col gap-3">
        <div className="font-mono text-xs break-all text-muted-foreground">{origin.location}</div>
        <div className="flex flex-wrap items-center gap-2 text-sm">
          <ReachBadge origin={origin} />
          <span>{plural(origin.files, "archivo", "archivos")}</span>
          {origin.pending > 0 && (
            <StatusBadge tone="busy" title="La base los está leyendo de a poco, en segundo plano">
              {plural(origin.pending, "pendiente de leer", "pendientes de leer")}
            </StatusBadge>
          )}
          {origin.errors > 0 && (
            <StatusBadge tone="error" title="Se vuelve a intentar en cada barrido">
              {plural(origin.errors, "no se pudo leer", "no se pudieron leer")}
            </StatusBadge>
          )}
          {origin.links > 0 && (
            <StatusBadge
              tone="neutral"
              title="Documentos propios de un disco en la nube: en la carpeta solo hay un acceso directo, sin contenido"
            >
              {plural(origin.links, "detectado, no procesable", "detectados, no procesables")}
            </StatusBadge>
          )}
          {origin.lastScanAt !== null && (
            <span className="text-xs text-muted-foreground">Último barrido: {formatAgo(origin.lastScanAt)}</span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2 text-sm">
          <span className="text-xs font-medium text-muted-foreground">Niveles de carpeta:</span>
          {editing === null ? (
            <>
              <span>{origin.convention.length > 0 ? origin.convention.join(" › ") : "sin definir"}</span>
              <Button
                variant="ghost"
                size="icon-xs"
                title="Cambiar"
                onClick={() => setEditing(origin.convention.join(", "))}
              >
                <Pencil />
              </Button>
            </>
          ) : (
            <>
              <Input
                className="h-7 max-w-xs"
                value={editing}
                placeholder="cliente, caso"
                autoFocus
                onChange={(event) => setEditing(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void saveConvention()
                  if (event.key === "Escape") setEditing(null)
                }}
              />
              <Button variant="ghost" size="icon-xs" title="Guardar" onClick={saveConvention}>
                <Check />
              </Button>
              <Button variant="ghost" size="icon-xs" title="Cancelar" onClick={() => setEditing(null)}>
                <X />
              </Button>
            </>
          )}
        </div>
      </div>
    </Panel>
  )
}

function ReachBadge({ origin }: { origin: OriginSummary }) {
  if (origin.reachable === null) {
    return <StatusBadge tone="busy">Primer barrido en curso</StatusBadge>
  }
  if (origin.reachable) {
    return <StatusBadge tone="ok">Conectado</StatusBadge>
  }
  return (
    <StatusBadge
      tone="warning"
      title="La carpeta no está o aparece vacía. Se conserva lo que se sabía de sus archivos hasta que vuelva."
    >
      No se puede leer
    </StatusBadge>
  )
}
