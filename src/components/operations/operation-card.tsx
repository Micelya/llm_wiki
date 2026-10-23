import { useEffect, useMemo, useState } from "react"
import { Check, ExternalLink, Undo2, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  cancelOperation,
  chooseOperationCandidate,
  confirmOperation,
  getOperationCard,
  undoOperation,
  type OperationCandidate,
  type OperationCard as Card,
  type OperationItem,
} from "@/commands/operations"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useOpenDocument, useOpenEntity, useOpenMatter } from "@/components/knowledge/shared"
import { vocabulary } from "@/vocabulary"

/** With more than this many things, the list folds and «Confirmar» asks again (4.3). */
const FOLDED_OVER = 10

function message(cause: unknown): string {
  return typeof cause === "string" ? cause : String((cause as Error)?.message ?? cause)
}

/**
 * The card of a change the chat proposed (4.2): what it will do, with the
 * verb and the count; each thing as the base keeps it, with a box to
 * untick it and a way to open it; what was not found; «¿Cuál?» when a name
 * matches several; the warnings the screen already gives; who proposed it.
 * Nothing is done until «Confirmar»; once done, «Deshacer». The words of
 * the profession come from the vocabulary.
 */
export function OperationCardView({ card: initial, onChange }: { card: Card; onChange?: (card: Card) => void }) {
  const dialog = useAppDialog()
  const openDocument = useOpenDocument()
  const openEntity = useOpenEntity()
  const openMatter = useOpenMatter()
  const words = vocabulary()
  const matterWord = words.knowledge.matter.toLowerCase()
  const [card, setCard] = useState<Card>(initial)
  const [unticked, setUnticked] = useState<Set<string>>(() => new Set(initial.items.filter((i) => i.status === "ok" && !i.checked && i.key).map((i) => i.key!)))
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [unfolded, setUnfolded] = useState(false)

  const update = (next: Card) => {
    setCard(next)
    onChange?.(next)
  }

  // A card that waits is shown as the base is now.
  useEffect(() => {
    if (initial.state !== "proposed" && initial.state !== "failed") return
    let cancelled = false
    getOperationCard(initial.id)
      .then((fresh) => {
        if (!cancelled) setCard(fresh)
      })
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [initial.id, initial.state])

  const waiting = card.state === "proposed" || card.state === "failed"
  const doable = card.items.filter((i) => i.status === "ok")
  const ticked = doable.filter((i) => i.required || (i.key !== null && !unticked.has(i.key)))
  const needsItems = !["caso_renombrar", "caso_decidir", "caso_cambiar_estado", "consulta_guardar", "caso_crear"].includes(card.operation)
  const canConfirm = waiting && !card.blocked && (!needsItems || ticked.length > 0)
  const shown = useMemo(() => (unfolded || card.items.length <= FOLDED_OVER ? card.items : card.items.slice(0, FOLDED_OVER)), [card.items, unfolded])

  const run = async (action: () => Promise<void>) => {
    setBusy(true)
    setProblem(null)
    setNotice(null)
    try {
      await action()
    } catch (cause) {
      setProblem(message(cause))
    } finally {
      setBusy(false)
    }
  }

  const confirm = () =>
    run(async () => {
      const chosen = ticked.map((i) => i.key).filter((k): k is string => k !== null)
      let second = false
      if (chosen.length > FOLDED_OVER) {
        second = await dialog.confirm({ title: "Confirmar", message: `${card.title}. ¿Seguro?`, confirmLabel: "Sí, confirmar" })
        if (!second) return
      }
      let result = await confirmOperation(card.id, chosen, card.fingerprint, second)
      if (result.outcome === "needs_second") {
        const sure = await dialog.confirm({ title: "Confirmar", message: `${result.card.title}. ¿Seguro?`, confirmLabel: "Sí, confirmar" })
        if (!sure) {
          setCard(result.card)
          return
        }
        result = await confirmOperation(card.id, chosen, result.card.fingerprint, true)
      }
      if (result.outcome === "changed") {
        setNotice("Algo cambió en la base desde que se armó la tarjeta: revisala y confirmá de nuevo.")
      }
      update(result.card)
    })

  const cancel = () =>
    run(async () => {
      update(await cancelOperation(card.id))
    })

  const undo = () =>
    run(async () => {
      let result = await undoOperation(card.id)
      if (result.outcome === "changed") {
        const sure = await dialog.confirm({
          title: "Deshacer",
          message: result.card.warnings[0] ?? "Algo cambió desde que se hizo. ¿Deshacer igual?",
          confirmLabel: "Deshacer igual",
          variant: "destructive",
        })
        if (!sure) return
        result = await undoOperation(card.id, true)
      }
      update(result.card)
    })

  const choose = (item: string, candidate: OperationCandidate) =>
    run(async () => {
      update(await chooseOperationCandidate(card.id, item, candidate.key))
    })

  const toggle = (item: OperationItem) => {
    if (!item.key || item.required) return
    setUnticked((previous) => {
      const next = new Set(previous)
      if (next.has(item.key!)) next.delete(item.key!)
      else next.add(item.key!)
      return next
    })
  }

  const open = (item: OperationItem) => {
    if (item.key?.startsWith("ent-")) openEntity(item.key)
    else if (item.locationId !== null) openDocument(item.locationId)
  }

  const engine = card.proposedBy.engine ? ` (${card.proposedBy.engine.split(":").slice(1).join(":") || card.proposedBy.engine})` : ""
  const target = card.target

  return (
    <div className="mt-2 rounded-lg border bg-card p-3 text-sm text-card-foreground" data-testid="operation-card">
      <div className="font-semibold">{card.title}</div>
      {card.request && <div className="mt-0.5 text-xs text-muted-foreground">Lo pediste: «{card.request}»</div>}

      {target?.status === "choose" && waiting && (
        <div className="mt-2">
          <div className="text-xs font-medium">¿Cuál {matterWord}? «{target.named}» coincide con varios:</div>
          <CandidateList candidates={target.candidates ?? []} busy={busy} onChoose={(c) => void choose("target", c)} />
        </div>
      )}

      {shown.length > 0 && (
        <ul className="mt-2 flex flex-col gap-1.5">
          {shown.map((item, index) => (
            <li key={`${item.named}-${item.key ?? index}`} className="flex flex-col gap-1">
              <ItemLine
                item={item}
                checked={item.status === "ok" && (item.required || (item.key !== null && !unticked.has(item.key)))}
                editable={waiting && item.status === "ok" && !item.required}
                onToggle={() => toggle(item)}
                onOpen={() => open(item)}
                matterWord={matterWord}
              />
              {item.status === "choose" && waiting && (
                <CandidateList candidates={item.candidates ?? []} busy={busy} onChoose={(c) => void choose(item.named, c)} />
              )}
              {item.detail && <SideDetail item={item} />}
            </li>
          ))}
        </ul>
      )}
      {card.items.length > FOLDED_OVER && !unfolded && (
        <button type="button" className="mt-1 text-xs text-primary hover:underline" onClick={() => setUnfolded(true)}>
          y {card.items.length - FOLDED_OVER} más, ver todos
        </button>
      )}

      {card.note && <div className="mt-2 whitespace-pre-wrap rounded bg-muted/60 px-2 py-1 text-xs">{card.note}</div>}

      {waiting && card.warnings.length > 0 && (
        <ul className="mt-2 list-disc pl-5 text-xs text-muted-foreground">
          {card.warnings.map((warning) => (
            <li key={warning}>{warning}</li>
          ))}
        </ul>
      )}
      {waiting && card.blocked && <div className="mt-2 text-xs text-amber-700 dark:text-amber-400">{card.blocked}</div>}
      {notice && <div className="mt-2 text-xs text-amber-700 dark:text-amber-400">{notice}</div>}
      {problem && <div className="mt-2 text-xs text-destructive">{problem}</div>}
      {card.result && !waiting && <div className="mt-2 text-xs font-medium">{card.result}</div>}
      {card.state === "failed" && card.result && <div className="mt-2 text-xs text-destructive">{card.result}</div>}

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {waiting && (
          <>
            <Button size="sm" disabled={busy || !canConfirm} onClick={() => void confirm()}>
              <Check /> Confirmar
            </Button>
            <Button size="sm" variant="outline" disabled={busy} onClick={() => void cancel()}>
              <X /> Cancelar
            </Button>
          </>
        )}
        {card.state === "applied" && card.undoable && (
          <Button size="sm" variant="outline" disabled={busy} onClick={() => void undo()}>
            <Undo2 /> Deshacer
          </Button>
        )}
        {card.state === "applied" && card.open && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => (card.open!.kind === "caso" ? openMatter(card.open!.id) : openEntity(card.open!.id))}
          >
            <ExternalLink /> {card.open.kind === "caso" ? `Abrir el ${matterWord}` : "Abrir la ficha"}
          </Button>
        )}
      </div>
      {waiting && card.undoNote && <div className="mt-2 text-xs text-muted-foreground">{card.undoNote}</div>}
      <div className="mt-2 text-[11px] text-muted-foreground">
        {card.proposedBy.caller === "chat" ? `Lo propuso el modelo${engine}.` : card.proposedBy.caller === "screen" ? "Desde la pantalla." : `Lo pidió ${card.proposedBy.caller.replace(/^program:/, "")}.`}
        {waiting ? " Todavía no se hizo nada." : ""}
      </div>
    </div>
  )
}

function ItemLine({
  item,
  checked,
  editable,
  onToggle,
  onOpen,
  matterWord,
}: {
  item: OperationItem
  checked: boolean
  editable: boolean
  onToggle: () => void
  onOpen: () => void
  matterWord: string
}) {
  if (item.status === "not_found") {
    return <div className="text-xs text-muted-foreground">«{item.named}»: no lo encontré en la base</div>
  }
  if (item.status === "choose") {
    return <div className="text-xs">«{item.named}»: coincide con varios, ¿cuál?</div>
  }
  const openable = item.locationId !== null || item.key?.startsWith("ent-")
  return (
    <label className={`flex items-start gap-2 ${item.status === "ok" ? "" : "text-muted-foreground"}`}>
      <input type="checkbox" className="mt-0.5" checked={checked} disabled={!editable} onChange={onToggle} aria-label={item.name ?? item.named} />
      <span className="min-w-0 flex-1 break-words">
        {item.name ?? item.named}
        {item.place ? <span className="text-muted-foreground"> · {item.place}</span> : null}
        {item.date ? <span className="text-muted-foreground"> · {item.date}</span> : null}
        {item.reason ? <span className="text-muted-foreground"> · {item.reason.replace("{caso}", matterWord)}</span> : null}
      </span>
      {openable && (
        <button type="button" className="shrink-0 text-xs text-primary hover:underline" onClick={(event) => { event.preventDefault(); onOpen() }}>
          abrir
        </button>
      )}
    </label>
  )
}

function CandidateList({ candidates, busy, onChoose }: { candidates: OperationCandidate[]; busy: boolean; onChoose: (candidate: OperationCandidate) => void }) {
  return (
    <ul className="ml-5 flex flex-col gap-1">
      {candidates.map((candidate) => (
        <li key={candidate.key}>
          <button
            type="button"
            disabled={busy}
            className="w-full rounded border px-2 py-1 text-left text-xs hover:bg-muted disabled:opacity-50"
            onClick={() => onChoose(candidate)}
          >
            {candidate.name}
            {candidate.place ? <span className="text-muted-foreground"> · {candidate.place}</span> : null}
            {candidate.date ? <span className="text-muted-foreground"> · {candidate.date}</span> : null}
          </button>
        </li>
      ))}
    </ul>
  )
}

/** One side of a decision about records: what the screen shows when deciding (D-224). */
function SideDetail({ item }: { item: OperationItem }) {
  const side = item.detail!
  return (
    <div className="ml-6 rounded bg-muted/50 px-2 py-1 text-xs">
      {side.aliases.length > 0 && <div>Se escribe también: {side.aliases.join(", ")}</div>}
      {side.identifiers.length > 0 && <div>{side.identifiers.join(", ")}</div>}
      <div className="text-muted-foreground">
        {side.documentCount === 1 ? "1 documento" : `${side.documentCount} documentos`}
        {side.documents.length > 0 ? `: ${side.documents.slice(0, 4).join(", ")}${side.documentCount > 4 ? "…" : ""}` : ""}
      </div>
    </div>
  )
}
