import { Fragment, useState } from "react"
import { ChevronDown, ChevronRight, LayoutTemplate, Pencil } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getSimilarTemplate, type FieldChoice, type SimilarGroup, type Template } from "@/commands/docbase"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { cn } from "@/lib/utils"
import { useTemplatesStore } from "@/stores/templates-store"
import { useWikiStore } from "@/stores/wiki-store"
import { formatCount, plural } from "./format"
import { StatusBadge } from "./page"
import {
  fieldName,
  fieldNote,
  fieldRows,
  madeWithLine,
  nameSourceText,
  placeLabel,
  possiblySameNames,
  rowIndexOf,
  templateSummary,
} from "./template"
import { FieldEditor } from "./template-field-editor"
import { useDocbaseQuery } from "./use-docbase-query"

const SELECT_CLASS =
  "h-7 max-w-full rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"

/**
 * The template a document was made with, at the head of its «Copias y
 * versiones»: one line that says it ("Hecho con la plantilla «Recibo»,
 * como 2 documentos más"), and the template itself when asked. The
 * documents made with one template are different documents: nothing to
 * decide (D-497). Nothing when it shares none.
 */
export function DocumentTemplate({ locationId, onOpen }: { locationId: number; onOpen: (locationId: number) => void }) {
  const template = useDocbaseQuery(`similar-template:${locationId}`, () => getSimilarTemplate(locationId))
  const [open, setOpen] = useState(false)
  const openInSection = useOpenTemplate()
  const generateFrom = useGenerateFromTemplate()
  const words = vocabulary()
  const data = template.data
  if (!data) return null
  const others = data.documents.length - 1
  const column = data.documents.findIndex((d) => d.doc.contentId === data.asked)
  return (
    <div className="mb-4 rounded-md border bg-muted/20 px-4 py-3 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p>
          {madeWithLine(data, data.documents[0]?.doc.name ?? "", others, words)}{" "}
          <span className="text-muted-foreground">{templateSummary(data)}</span>
        </p>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" aria-expanded={open} onClick={() => setOpen(!open)}>
            <LayoutTemplate /> {open ? "Ocultar la plantilla" : "Ver la plantilla"}
          </Button>
          {data.documents[0] && (
            <Button size="sm" variant="ghost" onClick={() => openInSection(data.documents[0].doc.locationId)}>
              Abrir en {words.templates.name}
            </Button>
          )}
          {data.documents[0] && (
            <Button size="sm" variant="ghost" onClick={() => generateFrom(data.documents[0].doc.locationId)}>
              Generar uno nuevo con esta {words.templates.one}
            </Button>
          )}
        </div>
      </div>
      {open && (
        <div className="mt-3">
          <TemplatePanel template={data} initialColumn={column >= 0 ? column : null} onOpen={onOpen} />
        </div>
      )}
    </div>
  )
}

/** Opens a template in the section «Plantillas», by the location of its principal. */
export function useOpenTemplate(): (principalLocationId: number) => void {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const openTemplate = useTemplatesStore((s) => s.openTemplate)
  return (principalLocationId) => {
    openTemplate(principalLocationId)
    setActiveView("templates")
  }
}

/** Opens, in the section «Plantillas», the screen to generate a document from a template, by its principal. */
export function useGenerateFromTemplate(): (principalLocationId: number) => void {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const generateFrom = useTemplatesStore((s) => s.generateFrom)
  return (principalLocationId) => {
    generateFrom(principalLocationId)
    setActiveView("templates")
  }
}

/**
 * In Documentos, filtered by a family: the template the family shares,
 * when it shares one, and the template itself when asked.
 */
export function FamilyTemplateLine({ group, onOpen }: { group: SimilarGroup; onOpen: (locationId: number) => void }) {
  const [open, setOpen] = useState(false)
  if (group.templateFields === null) return null
  return (
    <div className="rounded-md border bg-muted/20 px-4 py-3 text-sm">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p>
          Los documentos de esta familia comparten una plantilla:{" "}
          {group.templateFields === 1 ? "cambia un campo" : `cambian ${formatCount(group.templateFields)} campos`}.
          {group.possiblySame > 0 && (
            <span className="text-muted-foreground">
              {" "}
              En {plural(group.possiblySame, "par no cambia", "pares no cambia")} ningún dato: posiblemente sean el mismo
              documento.
            </span>
          )}
        </p>
        <Button size="sm" variant="outline" aria-expanded={open} onClick={() => setOpen(!open)}>
          <LayoutTemplate /> {open ? "Ocultar la plantilla" : "Ver la plantilla"}
        </Button>
      </div>
      {open && (
        <div className="mt-3">
          <FamilyTemplate principalLocationId={group.principal.locationId} onOpen={onOpen} />
        </div>
      )}
    </div>
  )
}

/** The template of a family, from the similar documents screen, by its principal. */
export function FamilyTemplate({ principalLocationId, onOpen }: { principalLocationId: number; onOpen: (locationId: number) => void }) {
  const template = useDocbaseQuery(`similar-template:${principalLocationId}`, () => getSimilarTemplate(principalLocationId))
  if (template.loading && !template.data) return <p className="text-xs text-muted-foreground">Armando la plantilla…</p>
  if (!template.data) return <p className="text-xs text-muted-foreground">Esta familia todavía no tiene su plantilla.</p>
  return <TemplatePanel template={template.data} initialColumn={null} onOpen={onOpen} />
}

/**
 * Whether a person confirmed every document of a template, before the
 * documents of one template stopped waiting for a decision (D-497). A
 * template no one confirmed says nothing: there is nothing to confirm.
 */
export function TemplateStateBadge({ state, words = vocabulary() }: { state: Template["state"]; words?: Vocabulary }) {
  return state === "confirmed" ? <StatusBadge tone="ok">{words.templates.confirmed}</StatusBadge> : null
}

function capitalized(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1)
}

/**
 * A template: what its documents say alike, with the fields that change
 * marked in their place, and a table with the value of each document in
 * each field. The documents made with it are different documents (D-497).
 */
export function TemplatePanel({
  template,
  initialColumn,
  onOpen,
  heading = true,
  onChooseField,
  busy = false,
  words = vocabulary(),
}: {
  template: Template
  /** The document whose values the text shows at first; `null`: the names of the fields. */
  initialColumn: number | null
  onOpen: (locationId: number) => void
  /** Its own title and state; the section «Plantillas» shows them above. */
  heading?: boolean
  /**
   * When given, a person can rename each field, say what it is and fix it
   * (the section «Plantillas»); `false` from it keeps the editor open.
   */
  onChooseField?: (position: number, choice: FieldChoice) => Promise<boolean | void> | boolean | void
  busy?: boolean
  words?: Vocabulary
}) {
  const [column, setColumn] = useState<number | null>(initialColumn)
  const [selected, setSelected] = useState<number | null>(null)
  const [editing, setEditing] = useState<number | null>(null)
  const [showText, setShowText] = useState(true)
  const rows = fieldRows(template)
  const same = possiblySameNames(template)
  const formRows = rows.filter((row) => row.field.formName !== null)

  return (
    <div className="flex flex-col gap-3" data-template={template.principal}>
      {heading && (
        <div className="flex flex-wrap items-center gap-2">
          <h3 className="font-medium">{capitalized(words.templates.one)}</h3>
          <TemplateStateBadge state={template.state} words={words} />
        </div>
      )}
      <p className="text-xs text-muted-foreground">
        Lo que todos los documentos dicen igual es la plantilla; lo resaltado cambia de un documento a otro. La armó la
        app comparando los textos, sin modelo; el modelo solo les pone nombre a los campos que el texto no nombra. Los
        documentos hechos con ella son documentos distintos: no hay nada que decidir.
      </p>

      {same.map(([a, b]) => (
        <div
          key={`${a}|${b}`}
          className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-800 dark:text-amber-300"
        >
          En «{a}» y «{b}» no cambia ningún dato (a lo sumo unas palabras): posiblemente sean el mismo documento y no
          la misma plantilla con otros datos. Revisalos en «{words.copies.name}»: si son el mismo, unilos como tales.
        </div>
      ))}

      <div>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <button
            className="flex items-center gap-1 text-left text-xs font-semibold tracking-wide text-muted-foreground uppercase"
            onClick={() => setShowText(!showText)}
          >
            {showText ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
            El texto de la plantilla
          </button>
          {showText && (
            <label className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              En los campos, mostrar
              <select
                className={SELECT_CLASS}
                value={column === null ? "" : String(column)}
                onChange={(event) => setColumn(event.target.value === "" ? null : Number(event.target.value))}
              >
                <option value="">sus nombres</option>
                {template.documents.map((d, index) => (
                  <option key={d.doc.contentId} value={index}>
                    los datos de «{d.doc.name}»
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
        {showText && (
          <div className="mt-2 max-h-96 overflow-y-auto rounded-md border bg-background px-4 py-3 font-serif text-[15px] leading-relaxed break-words whitespace-pre-wrap">
            {template.segments.map((segment, index) =>
              "text" in segment ? (
                <span key={index}>{segment.text}</span>
              ) : (
                <button
                  key={index}
                  type="button"
                  title={fieldName(template.fields[segment.field], words)}
                  onClick={() => setSelected(rowIndexOf(template, segment.field))}
                  className={cn(
                    "mx-0.5 rounded px-1 font-sans text-[13px] font-medium",
                    selected === rowIndexOf(template, segment.field)
                      ? "bg-primary text-primary-foreground"
                      : "bg-amber-500/20 text-amber-900 hover:bg-amber-500/30 dark:text-amber-200",
                  )}
                >
                  {placeLabel(template, segment.field, column, words)}
                </button>
              ),
            )}
            {formRows.length > 0 && (
              <p className="mt-3 font-sans text-xs text-muted-foreground">
                Además, {plural(formRows.length, "campo del formulario cambia", "campos del formulario cambian")}: están
                en la tabla.
              </p>
            )}
          </div>
        )}
      </div>

      <div>
        <h4 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          Los campos, documento por documento ({formatCount(rows.length)})
        </h4>
        {rows.length === 0 ? (
          <p className="mt-2 text-sm text-muted-foreground">Ningún campo cambia entre estos documentos.</p>
        ) : (
          <div className="mt-2 max-h-[32rem] overflow-auto rounded-md border">
            <table className="w-full border-collapse text-sm">
              <thead className="sticky top-0 bg-muted">
                <tr>
                  <th className="px-2 py-1.5 text-left text-xs font-medium text-muted-foreground">Campo</th>
                  {template.documents.map((d, index) => (
                    <th key={d.doc.contentId} className="min-w-40 px-2 py-1.5 text-left align-top text-xs font-medium">
                      <button
                        className="text-left break-words text-primary hover:underline"
                        title={`${d.doc.originName} › ${d.doc.relPath}`}
                        onClick={() => onOpen(d.doc.locationId)}
                      >
                        {d.doc.name}
                      </button>
                      <div className="font-normal text-muted-foreground">
                        {index === 0 ? "principal" : d.state === "joined" ? "confirmado" : "hecho con ella"}
                      </div>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <Fragment key={row.index}>
                    <tr
                      className={cn("border-t align-top", selected === row.index && "bg-primary/10")}
                      onClick={() => setSelected(row.index)}
                    >
                      <th className="px-2 py-1.5 text-left font-medium whitespace-nowrap">
                        <span className="flex items-center gap-1">
                          {fieldName(row.field, words)}
                          {onChooseField && (
                            <Button
                              size="icon-xs"
                              variant="ghost"
                              title="Cambiar el nombre, qué es o si es fijo"
                              onClick={(event) => {
                                event.stopPropagation()
                                setEditing(editing === row.index ? null : row.index)
                              }}
                            >
                              <Pencil />
                            </Button>
                          )}
                        </span>
                        {nameSourceText(row.field) && (
                          <div className="text-xs font-normal text-muted-foreground">{nameSourceText(row.field)}</div>
                        )}
                        {onChooseField && row.field.nameSource === null && (
                          <div className="text-xs font-normal text-muted-foreground">sin nombre todavía: lo nombra el modelo</div>
                        )}
                        {row.field.formName && row.field.name !== row.field.formName && (
                          <div className="text-xs font-normal text-muted-foreground">
                            del formulario: «{row.field.formName}»
                          </div>
                        )}
                        {fieldNote(row.field, words) && (
                          <div className="text-xs font-normal text-muted-foreground">{fieldNote(row.field, words)}</div>
                        )}
                        {row.places > 1 && (
                          <div className="text-xs font-normal text-muted-foreground">
                            {plural(row.places, "lugar", "lugares")} del texto
                          </div>
                        )}
                      </th>
                      {row.field.values.map((value, index) => (
                        <td key={index} className="px-2 py-1.5 break-words" title={value || undefined}>
                          {value ? <span className="line-clamp-4">{value}</span> : <span className="text-muted-foreground">—</span>}
                        </td>
                      ))}
                    </tr>
                    {onChooseField && editing === row.index && (
                      <tr className="border-t bg-muted/30">
                        <td colSpan={template.documents.length + 1} className="px-3 py-2">
                          <FieldEditor
                            field={row.field}
                            whats={template.whats}
                            busy={busy}
                            words={words}
                            onCancel={() => setEditing(null)}
                            onSave={(choice) => {
                              void Promise.resolve(onChooseField(row.index, choice)).then((saved) => {
                                if (saved !== false) setEditing(null)
                              })
                            }}
                          />
                        </td>
                      </tr>
                    )}
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}
