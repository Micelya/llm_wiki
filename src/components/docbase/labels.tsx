import { useState } from "react"
import { Check, Pencil, Plus, Trash2, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  addFixedRule,
  deleteGroup,
  deleteLabel,
  ensureLabel,
  listLabelGroups,
  listLabelRules,
  removeRule,
  renameGroup,
  renameLabel,
  setLevelsRule,
  type Label,
  type LabelGroup,
} from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { vocabulary } from "@/vocabulary"
import { parseConvention, plural } from "./format"
import { Panel } from "./page"
import { useDocbaseQuery } from "./use-docbase-query"

/** "Cliente: BGH", or just the name for a label that stands alone. */
export function labelText(label: Label): string {
  return label.group ? `${label.group}: ${label.name}` : label.name
}

/** What the user typed as a group and a name: "Cliente: BGH" or "urgente". */
export function parseLabelText(text: string): { group: string | null; name: string } {
  const at = text.indexOf(":")
  if (at <= 0) return { group: null, name: text.trim() }
  return { group: text.slice(0, at).trim(), name: text.slice(at + 1).trim() }
}

export function LabelChip({ label, onRemove }: { label: Label; onRemove?: () => void }) {
  return (
    <span className="inline-flex max-w-full items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs">
      <span className="size-2 shrink-0 rounded-full" style={{ backgroundColor: label.color }} />
      {label.group && <span className="shrink-0 text-muted-foreground">{label.group}:</span>}
      <span className="truncate">{label.name}</span>
      {onRemove && (
        <button className="shrink-0 text-muted-foreground hover:text-foreground" title="Quitar" onClick={onRemove}>
          <X className="size-3" />
        </button>
      )}
    </span>
  )
}

/**
 * Finds a label by typing, or creates it: "Cliente: BGH" makes the label
 * BGH in the group Cliente; a name without a colon stands alone.
 */
export function LabelPicker({
  exclude = [],
  onPick,
  onProblem,
}: {
  exclude?: number[]
  onPick: (label: Label) => void | Promise<void>
  onProblem: (problem: string | null) => void
}) {
  const groups = useDocbaseQuery("label-groups", listLabelGroups)
  const [text, setText] = useState("")
  const typed = text.trim().toLowerCase()
  const all = (groups.data ?? []).flatMap((group) => group.labels)
  const matches = all
    .filter((label) => !exclude.includes(label.id))
    .filter((label) => typed === "" || labelText(label).toLowerCase().includes(typed))
    .slice(0, 8)
  const exact = all.some((label) => labelText(label).toLowerCase() === typed)

  async function create() {
    const { group, name } = parseLabelText(text)
    if (!name) return
    try {
      const label = await ensureLabel(group, name)
      setText("")
      await onPick(label)
    } catch (cause) {
      onProblem(String(cause))
    }
  }

  return (
    <div className="flex flex-col gap-1.5">
      <Input
        className="h-7"
        value={text}
        placeholder="Buscar o crear: Cliente: BGH"
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && typed !== "" && !exact) void create()
        }}
      />
      <div className="flex flex-wrap gap-1">
        {matches.map((label) => (
          <button
            key={label.id}
            title="Usar esta etiqueta"
            onClick={() => {
              setText("")
              void onPick(label)
            }}
          >
            <LabelChip label={label} />
          </button>
        ))}
        {typed !== "" && !exact && (
          <button className="inline-flex items-center gap-1 rounded border border-dashed px-1.5 py-0.5 text-xs" onClick={create}>
            <Plus className="size-3" /> Crear «{text.trim()}»
          </button>
        )}
      </div>
    </div>
  )
}

/**
 * The label rules of one folder: what everything under it carries, and
 * what its subfolders stand for.
 */
export function FolderRules({
  originId,
  folder,
  onProblem,
}: {
  originId: string
  folder: string
  onProblem: (problem: string | null) => void
}) {
  const rules = useDocbaseQuery(`label-rules:${originId}`, () => listLabelRules(originId))
  const here = (rules.data ?? []).filter((rule) => rule.folder === folder)
  const levels = here.find((rule) => rule.kind === "levels")
  const [levelsText, setLevelsText] = useState<string | null>(null)
  const suggestion = vocabulary().suggestedGroups.join(", ")

  async function run(action: () => Promise<void>) {
    onProblem(null)
    try {
      await action()
      rules.reload()
    } catch (cause) {
      onProblem(String(cause))
    }
  }

  return (
    <div className="flex flex-col gap-3 border-b bg-muted/30 px-4 py-3 text-sm">
      <div>
        <div className="text-xs font-medium text-muted-foreground">
          Todo lo que hay {folder ? "en esta carpeta" : "en este origen"} lleva estas etiquetas
        </div>
        <div className="mt-1.5 flex flex-wrap gap-1">
          {here
            .filter((rule) => rule.kind === "fixed" && rule.label)
            .map((rule) => (
              <LabelChip key={rule.id} label={rule.label!} onRemove={() => run(() => removeRule(rule.id))} />
            ))}
        </div>
        <div className="mt-1.5 max-w-sm">
          <LabelPicker
            exclude={here.flatMap((rule) => (rule.label ? [rule.label.id] : []))}
            onPick={(label) => run(() => addFixedRule(originId, folder, label.id))}
            onProblem={onProblem}
          />
        </div>
      </div>
      <div>
        <div className="text-xs font-medium text-muted-foreground">
          Cada subcarpeta de acá es un… (de afuera hacia adentro, si hay más de un nivel)
        </div>
        <div className="mt-1.5 flex max-w-sm items-center gap-1.5">
          <Input
            className="h-7"
            value={levelsText ?? levels?.groups.join(", ") ?? ""}
            placeholder={suggestion}
            onChange={(event) => setLevelsText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && levelsText !== null) {
                void run(() => setLevelsRule(originId, folder, parseConvention(levelsText))).then(() =>
                  setLevelsText(null),
                )
              }
            }}
          />
          <Button
            variant="outline"
            size="sm"
            disabled={levelsText === null}
            onClick={() =>
              run(() => setLevelsRule(originId, folder, parseConvention(levelsText ?? ""))).then(() =>
                setLevelsText(null),
              )
            }
          >
            Guardar
          </Button>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          Por ejemplo «{suggestion}»: cada subcarpeta pasa a ser una etiqueta de ese grupo. Conviene solo donde las
          subcarpetas siguen el mismo criterio. Dejalo vacío para quitar la regla.
        </p>
      </div>
    </div>
  )
}

/** Every group and label of the space, to rename or delete them. */
export function LabelsPanel({ onProblem }: { onProblem: (problem: string | null) => void }) {
  const groups = useDocbaseQuery("label-groups", listLabelGroups)
  const dialog = useAppDialog()
  const data = groups.data ?? []
  if (data.length === 0) return null

  async function run(action: () => Promise<void>) {
    onProblem(null)
    try {
      await action()
      groups.reload()
    } catch (cause) {
      onProblem(String(cause))
    }
  }

  async function removeGroup(group: LabelGroup) {
    if (group.id === null) return
    const confirmed = await dialog.confirm({
      title: `Borrar el grupo "${group.name}"`,
      message: `Se borran sus ${plural(group.labels.length, "etiqueta", "etiquetas")}, se quitan de todos los archivos y se quitan las reglas de carpeta que lo usan. Los archivos no se tocan.`,
      confirmLabel: "Borrar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (confirmed) await run(() => deleteGroup(group.id!))
  }

  return (
    <Panel title="Grupos y etiquetas">
      <div className="flex flex-col gap-3">
        {data.map((group) => (
          <div key={group.id ?? "loose"}>
            <div className="flex items-center gap-1 text-sm font-medium">
              {group.id === null ? (
                <span className="text-muted-foreground">Sin grupo</span>
              ) : (
                <Renamable
                  text={group.name ?? ""}
                  onRename={(name) => run(() => renameGroup(group.id!, name))}
                  onDelete={() => removeGroup(group)}
                />
              )}
            </div>
            <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1">
              {group.labels.map((label) => (
                <span key={label.id} className="flex items-center gap-1 text-sm">
                  <span className="size-2 rounded-full" style={{ backgroundColor: label.color }} />
                  <Renamable
                    text={label.name}
                    note={String(label.files)}
                    onRename={(name) => run(() => renameLabel(label.id, name))}
                    onDelete={() => run(() => deleteLabel(label.id))}
                  />
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>
    </Panel>
  )
}

function Renamable({
  text,
  note,
  onRename,
  onDelete,
}: {
  text: string
  note?: string
  onRename: (name: string) => void | Promise<void>
  onDelete: () => void | Promise<void>
}) {
  const [editing, setEditing] = useState<string | null>(null)
  if (editing === null) {
    return (
      <span className="group/item flex items-center gap-1">
        <span>{text}</span>
        {note && <span className="text-xs text-muted-foreground">{note}</span>}
        <button
          className="text-muted-foreground opacity-0 group-hover/item:opacity-100 hover:text-foreground"
          title="Renombrar"
          onClick={() => setEditing(text)}
        >
          <Pencil className="size-3" />
        </button>
        <button
          className="text-muted-foreground opacity-0 group-hover/item:opacity-100 hover:text-destructive"
          title="Borrar"
          onClick={() => void onDelete()}
        >
          <Trash2 className="size-3" />
        </button>
      </span>
    )
  }
  const save = () => {
    void onRename(editing)
    setEditing(null)
  }
  return (
    <span className="flex items-center gap-1">
      <Input
        className="h-6 w-40"
        value={editing}
        autoFocus
        onChange={(event) => setEditing(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") save()
          if (event.key === "Escape") setEditing(null)
        }}
      />
      <button title="Guardar" onClick={save}>
        <Check className="size-3.5" />
      </button>
      <button title="Cancelar" onClick={() => setEditing(null)}>
        <X className="size-3.5" />
      </button>
    </span>
  )
}
