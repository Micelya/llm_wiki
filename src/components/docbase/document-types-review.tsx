import { useState } from "react"
import { Check, Pencil, Undo2, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  decideType,
  listDecidedTypes,
  listDocuments,
  listProposedTypes,
  type DecidedType,
  type ProposedType,
  type TypeDecision,
} from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { DEFAULT_DOCUMENT_FILTER, useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { documentTypeName, vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, Panel, StatusBadge } from "./page"
import { formatDate, plural } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

/** Documents of a proposed type shown as examples. */
export const TYPE_EXAMPLES = 5

/**
 * The document types the model proposed, to confirm, rename or reject
 * them (R-22). A confirmed type joins the closed list the model chooses
 * from, with the name it was given; a rejected one leaves its documents
 * without a type, to be asked again telling the model not to propose it.
 * Every decision can be undone.
 */
export function DocumentTypesReview() {
  const proposed = useDocbaseQuery("types-review:proposed", listProposedTypes)
  const decided = useDocbaseQuery("types-review:decided", listDecidedTypes)
  const [problem, setProblem] = useState<string | null>(null)

  async function decide(typeId: string, decision: TypeDecision) {
    setProblem(null)
    try {
      await decideType(typeId, decision)
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      proposed.reload()
      decided.reload()
    }
  }

  const waiting = proposed.data ?? []
  return (
    <DocbasePage
      title="Tipos de documento propuestos"
      subtitle={proposed.data ? plural(waiting.length, "tipo espera una decisión", "tipos esperan una decisión") : undefined}
    >
      <div className="mx-auto flex max-w-4xl flex-col gap-6">
        <p className="text-sm text-muted-foreground">
          Cuando ningún tipo de la lista corresponde, el modelo propone uno nuevo. Hasta que una persona lo confirme no
          entra a la lista de tipos, y el modelo no lo puede elegir para otros documentos. Confirmalo con el nombre que
          prefieras, o rechazalo: sus documentos vuelven a preguntarse, diciéndole al modelo que no lo proponga.
        </p>
        {problem && <ErrorNote>{problem}</ErrorNote>}
        {proposed.error && <ErrorNote>{proposed.error}</ErrorNote>}
        {proposed.data && waiting.length === 0 && (
          <Panel>
            <EmptyNote>El modelo no propuso ningún tipo nuevo que espere una decisión.</EmptyNote>
          </Panel>
        )}
        {waiting.map((type) => (
          <ProposedTypeCard key={type.id} type={type} onDecide={decide} />
        ))}
        {(decided.data ?? []).length > 0 && (
          <Panel title="Ya decididos">
            <ul className="flex flex-col divide-y">
              {(decided.data ?? []).map((type) => (
                <DecidedRow key={type.id} type={type} onUndo={() => decide(type.id, { kind: "undo" })} />
              ))}
            </ul>
          </Panel>
        )}
      </div>
    </DocbasePage>
  )
}

function ProposedTypeCard({
  type,
  onDecide,
}: {
  type: ProposedType
  onDecide: (typeId: string, decision: TypeDecision) => Promise<void>
}) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const name = documentTypeName(type.id, type.name, words)
  const [renaming, setRenaming] = useState(false)
  const [newName, setNewName] = useState(name)
  const [busy, setBusy] = useState(false)
  const examples = useDocbaseQuery(`types-review:examples:${type.id}`, () =>
    listDocuments({ docType: type.id, limit: TYPE_EXAMPLES, sort: "name" }),
  )
  const setFilter = useDocbaseStore((s) => s.setFilter)
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const setActiveView = useWikiStore((s) => s.setActiveView)

  async function run(decision: TypeDecision) {
    setBusy(true)
    try {
      await onDecide(type.id, decision)
    } finally {
      setBusy(false)
    }
  }

  async function reject() {
    const confirmed = await dialog.confirm({
      title: `Rechazar «${name}»`,
      message:
        `${plural(type.documents, "documento pierde", "documentos pierden")} este tipo y vuelven a preguntarse al modelo, ` +
        "diciéndole que no lo proponga. Hasta entonces quedan sin tipo. Se puede deshacer.",
      confirmLabel: "Rechazar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (confirmed) await run({ kind: "reject" })
  }

  return (
    <Panel
      title={
        <>
          {name} <StatusBadge tone="busy">propuesto</StatusBadge>
        </>
      }
      count={plural(type.documents, "documento", "documentos")}
    >
      <div className="flex flex-col gap-3 text-sm">
        {type.description && <p className="font-serif italic">{type.description}</p>}
        <div>
          <p className="text-xs font-medium text-muted-foreground">Algunos de sus documentos</p>
          <ul className="mt-1 flex flex-col gap-0.5">
            {(examples.data?.rows ?? []).map((row) => (
              <li key={row.locationId}>
                <button
                  className="text-left text-primary hover:underline"
                  onClick={() => {
                    setActiveView("documents")
                    openDocument(row.locationId)
                  }}
                >
                  {row.name}
                </button>
                <span className="text-xs text-muted-foreground"> · {row.originName} › {row.folder || "(raíz)"}</span>
              </li>
            ))}
          </ul>
          {type.documents > TYPE_EXAMPLES && (
            <button
              className="mt-1 text-xs text-primary hover:underline"
              onClick={() => {
                setFilter({ ...DEFAULT_DOCUMENT_FILTER, docType: type.id })
                setActiveView("documents")
              }}
            >
              Ver los {type.documents}
            </button>
          )}
        </div>
        {renaming ? (
          <form
            className="flex flex-wrap items-center gap-2"
            onSubmit={(event) => {
              event.preventDefault()
              if (newName.trim()) void run({ kind: "confirm", name: newName.trim(), description: type.description })
            }}
          >
            <Input className="max-w-xs" value={newName} autoFocus onChange={(event) => setNewName(event.target.value)} />
            <Button size="sm" type="submit" disabled={busy || !newName.trim()}>
              <Check /> Confirmar con este nombre
            </Button>
            <Button size="sm" variant="ghost" type="button" onClick={() => setRenaming(false)}>
              Cancelar
            </Button>
          </form>
        ) : (
          <div className="flex flex-wrap gap-2">
            <Button size="sm" disabled={busy} onClick={() => void run({ kind: "confirm", name, description: type.description })}>
              <Check /> Confirmar «{name}»
            </Button>
            <Button size="sm" variant="outline" disabled={busy} onClick={() => setRenaming(true)}>
              <Pencil /> Cambiar el nombre
            </Button>
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => void reject()}>
              <X /> Rechazar
            </Button>
          </div>
        )}
      </div>
    </Panel>
  )
}

function DecidedRow({ type, onUndo }: { type: DecidedType; onUndo: () => Promise<void> }) {
  const [busy, setBusy] = useState(false)
  const name = type.name ?? documentTypeName(type.id)
  return (
    <li className="flex items-center gap-3 py-2 text-sm">
      <span className="min-w-0 flex-1">
        <span className="font-medium">{name}</span>
        <span className="text-xs text-muted-foreground">
          {" · "}
          {type.decision === "confirmed"
            ? `confirmado el ${formatDate(type.decidedAt)}, ${plural(type.documents, "documento", "documentos")}`
            : `rechazado el ${formatDate(type.decidedAt)}`}
        </span>
      </span>
      <Button
        size="sm"
        variant="ghost"
        disabled={busy}
        onClick={() => {
          setBusy(true)
          void onUndo().finally(() => setBusy(false))
        }}
      >
        <Undo2 /> Deshacer
      </Button>
    </li>
  )
}
