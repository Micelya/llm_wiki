import { useState } from "react"
import { AlertCircle, CheckCircle2, Clock, HelpCircle, Loader2, PauseCircle, RotateCcw, UserRound } from "lucide-react"
import { Button } from "@/components/ui/button"
import { processView, reprocess, type ProcessView } from "@/commands/docbase"
import { backLine, statusLine, stepGroups, stepLine, type StepMark } from "@/lib/progress/process"
import { useAttentionViewStore } from "@/stores/attention-view-store"
import { EmptyNote, ErrorNote, StatusBadge, type Tone } from "@/components/docbase/page"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"

const MARKS: Record<StepMark, { icon: typeof Clock; className: string }> = {
  complete: { icon: CheckCircle2, className: "text-ok" },
  working: { icon: Loader2, className: "animate-spin text-info" },
  pending: { icon: Clock, className: "text-muted-foreground" },
  aside: { icon: PauseCircle, className: "text-warn" },
  person: { icon: UserRound, className: "text-warn" },
  unknown: { icon: HelpCircle, className: "text-muted-foreground" },
}

const STATE_TONES: Record<string, Tone> = {
  Procesando: "busy",
  Detenido: "ok",
  "Esperando a una persona": "warning",
  Esperando: "neutral",
  Empezando: "neutral",
}

/**
 * The process in «En curso» (A3 of 2 nov): above, how the loop stands and
 * why it went back the last time; below, the 19 steps in two groups, «Cada
 * archivo» and «Cada documento», each complete, working, with what is
 * pending or set aside (by a failure, a person or a type of low confidence,
 * with a link to Atención). The documents are counted with the one rule of
 * Inicio. There is no line of the barrier: there is no barrier.
 */
export function ProcessSteps() {
  const query = useDocbaseQuery("progress:process", processView)
  if (query.error) return <ErrorNote>{query.error}</ErrorNote>
  if (!query.data) return <EmptyNote>Buscando…</EmptyNote>
  return <ProcessStepsView view={query.data} onAgain={query.reload} />
}

export function ProcessStepsView({ view, onAgain }: { view: ProcessView; onAgain?: () => void }) {
  const status = statusLine(view)
  const back = backLine(view)
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-start justify-between gap-3 rounded-lg border bg-card px-4 py-3">
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex flex-wrap items-center gap-2 text-sm">
            <StatusBadge tone={STATE_TONES[status.state] ?? "neutral"}>{status.state}</StatusBadge>
            {status.detail && <span className="min-w-0 text-muted-foreground">{status.detail}</span>}
          </div>
          {back && <p className="text-xs text-muted-foreground">{back}</p>}
        </div>
        <ProcessAgainButton onDone={onAgain} />
      </div>
      {stepGroups(view).map((group) => (
        <section key={group.title} className="rounded-lg border bg-card">
          <header className="flex flex-wrap items-baseline justify-between gap-2 border-b px-4 py-2">
            <h3 className="text-sm font-medium">{group.title}</h3>
            <span className="text-xs text-muted-foreground">{group.counts}</span>
          </header>
          <ol className="flex flex-col divide-y">
            {group.steps.map((step) => {
              const line = stepLine(step, view)
              const mark = MARKS[line.mark]
              const Icon = mark.icon
              return (
                <li key={step.about.key} className="flex items-center gap-3 px-4 py-2 text-sm" data-step={step.about.key}>
                  <Icon className={`size-4 shrink-0 ${mark.className}`} />
                  <span className="w-6 shrink-0 text-right text-xs text-muted-foreground">{step.about.number}</span>
                  <span className="min-w-0 flex-1 truncate" title={step.about.name}>
                    {step.about.name}
                  </span>
                  <span className="shrink-0 text-xs text-muted-foreground">{line.text}</span>
                  {line.attention && <AttentionLink />}
                </li>
              )
            })}
          </ol>
        </section>
      ))}
    </div>
  )
}

/** To Atención, where each failure and each decision waits with its reason. */
function AttentionLink() {
  const setTab = useAttentionViewStore((s) => s.setTab)
  return (
    <button className="shrink-0 text-xs text-info underline-offset-2 hover:underline" onClick={() => setTab("problem")}>
      <AlertCircle className="mr-1 inline size-3" />
      ver en Atención
    </button>
  )
}

/**
 * «Volver a procesar» todo: the inventory of every folder is left
 * incomplete and the process takes it in its order. Nothing is asked to the
 * AI again unless something changed; reading again with the AI is «Leer de
 * nuevo», with its cost.
 */
export function ProcessAgainButton({ onDone }: { onDone?: () => void }) {
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  return (
    <div className="flex flex-col items-end gap-1">
      <Button
        size="sm"
        variant="outline"
        disabled={busy}
        title="Vuelve a mirar cada carpeta y la huella de cada archivo. Lo que no cambió no se rehace ni se le pregunta de nuevo a la IA."
        onClick={async () => {
          setBusy(true)
          setProblem(null)
          try {
            await reprocess({})
            onDone?.()
          } catch (cause) {
            setProblem(String(cause))
          } finally {
            setBusy(false)
          }
        }}
      >
        <RotateCcw className="mr-1 size-4" />
        Volver a procesar todo
      </Button>
      {problem && <ErrorNote>{problem}</ErrorNote>}
    </div>
  )
}
