import { useState } from "react"
import { Button } from "@/components/ui/button"
import { getKnowledgeNotes, setKnowledgeNotes } from "@/commands/docbase"
import { ErrorNote } from "@/components/docbase/page"
import { formatAgo } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { Section, useAction } from "./shared"

/**
 * The notes of a person on a person or organization, a matter or a
 * document (D-407): plain text kept in the base, apart from what the app
 * writes, which no pass of the app changes. Saved when the person says
 * so; the same on the three screens. Empty, they go.
 */
export function NotesEditor({ objectId, id = "notas" }: { objectId: string; id?: string }) {
  const notes = useDocbaseQuery(`notes:${objectId}`, () => getKnowledgeNotes(objectId))
  const action = useAction()
  // What the person is writing, while it differs from what is saved; the
  // base changing meanwhile does not touch it.
  const [draft, setDraft] = useState<string | null>(null)
  const saved = notes.data?.text ?? ""
  const changed = draft !== null && draft !== saved

  function save() {
    const text = draft ?? ""
    void action.run(async () => {
      await setKnowledgeNotes(objectId, text)
      setDraft(null)
      notes.reload()
    })
  }

  return (
    <Section id={id} title="Notas">
      {(notes.error || action.problem) && (
        <div className="mb-2">
          <ErrorNote>{notes.error ?? action.problem}</ErrorNote>
        </div>
      )}
      <textarea
        aria-label="Notas"
        className="min-h-28 w-full rounded-[10px] border border-border-strong bg-card px-3.5 py-3 text-[0.9375rem] leading-relaxed placeholder:text-muted-foreground focus:border-link focus:ring-3 focus:ring-ring/30 focus:outline-none"
        value={draft ?? saved}
        placeholder="Lo que sabés y los documentos no dicen. La app no lo cambia."
        disabled={notes.loading && !notes.data}
        onChange={(event) => setDraft(event.target.value)}
      />
      <div className="mt-2.5 flex items-center gap-2.5">
        <Button size="sm" disabled={!changed || action.busy} onClick={save}>
          Guardar
        </Button>
        {changed && (
          <Button size="sm" variant="ghost" disabled={action.busy} onClick={() => setDraft(null)}>
            Descartar los cambios
          </Button>
        )}
        <span className="text-[0.8125rem] text-muted-foreground">
          {changed ? "Sin guardar" : notes.data ? `Guardadas ${formatAgo(notes.data.updatedAt)}` : ""}
        </span>
      </div>
    </Section>
  )
}
