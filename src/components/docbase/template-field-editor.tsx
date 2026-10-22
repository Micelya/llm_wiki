import { useState, type FormEvent } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import type { FieldChoice, TemplateField } from "@/commands/docbase"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { automaticFieldName, choiceOf, decided, distinctValues, whatName } from "./template"

const SELECT_CLASS =
  "h-8 max-w-full rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"

/** The choice of a fixed value that is none of the documents'. */
const OTHER = "\u0000otro"

/**
 * What a person decides about a field of a template (D-488): its name,
 * what it is and whether it is fixed, with one of the values of the
 * documents or another one. Kept apart from the template, so it survives
 * every time the template is built again; a name a person writes is never
 * overwritten by the program nor by the model.
 */
export function FieldEditor({
  field,
  whats,
  busy,
  onSave,
  onCancel,
  words = vocabulary(),
}: {
  field: TemplateField
  /** What a field can be. */
  whats: string[]
  busy: boolean
  onSave: (choice: FieldChoice) => void
  onCancel: () => void
  words?: Vocabulary
}) {
  const start = choiceOf(field)
  const values = distinctValues(field)
  const [name, setName] = useState(start.name ?? "")
  const [what, setWhat] = useState(start.what ?? "")
  const [fixed, setFixed] = useState(start.fixed)
  const [fixedValue, setFixedValue] = useState(start.fixedValue ?? values[0] ?? "")
  const [other, setOther] = useState(start.fixed && !!start.fixedValue && !values.includes(start.fixedValue))
  const automaticName = field.nameSource && field.nameSource !== "person" && field.name ? field.name : automaticFieldName(field, words)
  const automaticWhat = field.whatSource && field.whatSource !== "person" && field.what ? whatName(field.what, words) : null

  function submit(event: FormEvent) {
    event.preventDefault()
    onSave({
      name: name.trim() || null,
      what: what || null,
      fixed,
      fixedValue: fixed ? fixedValue : null,
    })
  }

  return (
    <form className="flex flex-col gap-3 py-1 text-sm" onSubmit={submit} onClick={(event) => event.stopPropagation()}>
      <div className="flex flex-wrap items-end gap-3">
        <label className="flex min-w-56 flex-1 flex-col gap-1">
          <span className="text-xs text-muted-foreground">Nombre</span>
          <Input
            autoFocus
            value={name}
            placeholder={automaticName}
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => event.key === "Escape" && onCancel()}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Qué es</span>
          <select className={SELECT_CLASS} value={what} onChange={(event) => setWhat(event.target.value)}>
            <option value="">{automaticWhat ? `Automático: ${automaticWhat}` : "Sin decir"}</option>
            {whats.map((id) => (
              <option key={id} value={id}>
                {whatName(id, words)}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="flex flex-col gap-2">
        <label className="flex items-center gap-2">
          <input type="checkbox" checked={fixed} onChange={(event) => setFixed(event.target.checked)} />
          Fijo: no se pide al generar un documento y se escribe siempre igual
        </label>
        {fixed && (
          <div className="flex flex-wrap items-center gap-2 pl-6">
            <select
              className={SELECT_CLASS}
              value={other ? OTHER : fixedValue}
              onChange={(event) => {
                const chosen = event.target.value
                setOther(chosen === OTHER)
                if (chosen !== OTHER) setFixedValue(chosen)
              }}
            >
              {values.map((value) => (
                <option key={value} value={value}>
                  {value.length > 80 ? `${value.slice(0, 80)}…` : value}
                </option>
              ))}
              <option value="">(nada)</option>
              <option value={OTHER}>Otro…</option>
            </select>
            {other && (
              <Input
                className="max-w-md flex-1"
                value={fixedValue}
                placeholder="Lo que se escribe siempre"
                onChange={(event) => setFixedValue(event.target.value)}
              />
            )}
          </div>
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        <Button type="submit" size="sm" disabled={busy}>
          Guardar
        </Button>
        <Button type="button" size="sm" variant="ghost" onClick={onCancel}>
          Cancelar
        </Button>
        {decided(field) && (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={busy}
            onClick={() => onSave({ name: null, what: null, fixed: false, fixedValue: null })}
          >
            Volver a lo automático
          </Button>
        )}
      </div>
    </form>
  )
}
