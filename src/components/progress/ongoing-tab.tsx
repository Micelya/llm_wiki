import { useEffect, useState, type ReactNode } from "react"
import { AlertCircle, CheckCircle2, ChevronDown, ChevronRight, Clock, Loader2, PauseCircle } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  knowledgeReadAgainFailed,
  listOrigins,
  listProgressFolders,
  listStageItems,
  type Progress,
  type ProgressScope,
  type ProgressStage,
  type StageItem,
  type StageItems,
} from "@/commands/docbase"
import type { AttentionTarget } from "@/lib/attention/types"
import { modelErrorText, pageOf, stageDetails, stageFigures, stageShare, stageText, stateLabel } from "@/lib/progress/stages"
import { MISSING_ENGINE, recognitionFailureText } from "@/lib/text-recognition"
import { useAttentionViewStore } from "@/stores/attention-view-store"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { goToTarget } from "@/components/attention/navigate"
import {
  BackgroundReadingBadge,
  BackgroundReadingButton,
  useBackgroundReading,
} from "@/components/docbase/background-reading-panel"
import { EmptyNote, ErrorNote, StatusBadge, type Tone } from "@/components/docbase/page"
import { formatCount } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { useRecognize, useRecognitionEngine, type Recognize } from "@/components/docbase/recognition"
import { useProgressStore } from "@/stores/progress-store"
import { useProgress } from "./use-progress"

/** Things listed when a stage is opened; the rest are counted. */
export const STAGE_ITEMS = 50

/**
 * The tab «En curso» of the attention view (D-424): every stage of the
 * processing, done and to do, in the order they happen; each one opens
 * into its documents with a button to open each. Above, whether the
 * automatic reading is on, and the origin or folder it is narrowed to.
 * `extra` is what other parts of the app raise as in progress.
 */
export function OngoingTab({ extra }: { extra?: ReactNode }) {
  const scope = useAttentionViewStore((s) => s.scope)
  const setScope = useAttentionViewStore((s) => s.setScope)
  const { progress, error } = useProgress(scope)
  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-card px-4 py-3">
        <ReadingState progress={progress} />
        <ScopePicker scope={scope} onChange={setScope} />
      </div>
      {error && <ErrorNote>{error}</ErrorNote>}
      {progress ? <StageList progress={progress} scope={scope} /> : !error && <EmptyNote>Buscando…</EmptyNote>}
      {extra}
    </div>
  )
}

/** Whether the automatic reading is on, with the switch of Orígenes. */
function ReadingState({ progress }: { progress: Progress | null }) {
  const reading = useBackgroundReading()
  const on = reading.on ?? progress?.backgroundReading ?? false
  return (
    <div className="flex min-w-0 flex-col gap-1">
      <div className="flex items-center gap-2 text-sm font-medium">
        Lectura automática <BackgroundReadingBadge on={on} />
        {reading.on !== null && <BackgroundReadingButton reading={reading} />}
      </div>
      <p className="text-xs text-muted-foreground">
        {progress && !progress.engineChosen
          ? "Falta elegir un modelo en Ajustes: sin él, las etapas que usan el modelo no avanzan."
          : on
            ? "Las etapas que usan el modelo avanzan solas."
            : "Las etapas que usan el modelo esperan hasta que la actives."}
      </p>
      {reading.problem && <ErrorNote>{reading.problem}</ErrorNote>}
    </div>
  )
}

/** The origin, and the folder of it, the figures are narrowed to. */
export function ScopePicker({ scope, onChange }: { scope: ProgressScope; onChange: (scope: ProgressScope) => void }) {
  const origins = useDocbaseQuery("progress:origins", listOrigins)
  const originId = scope.originId ?? ""
  const folders = useDocbaseQuery(`progress:folders:${originId}`, () =>
    originId ? listProgressFolders(originId) : Promise.resolve([]),
  )
  const select = "h-8 max-w-64 rounded-md border bg-background px-2 text-sm"
  return (
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <label className="text-muted-foreground" htmlFor="progress-origin">
        Ver
      </label>
      <select
        id="progress-origin"
        className={select}
        value={originId}
        onChange={(event) => onChange(event.target.value ? { originId: event.target.value, folder: null } : {})}
      >
        <option value="">Todas las carpetas</option>
        {(origins.data ?? []).map((origin) => (
          <option key={origin.id} value={origin.id}>
            {origin.name}
          </option>
        ))}
      </select>
      {originId && (folders.data?.length ?? 0) > 0 && (
        <select
          aria-label="Carpeta"
          className={select}
          value={scope.folder ?? ""}
          onChange={(event) => onChange({ originId, folder: event.target.value || null })}
        >
          <option value="">Toda la carpeta</option>
          {(folders.data ?? []).map((folder) => {
            const depth = folder.folder.split("/").length - 1
            const name = folder.folder.split("/").pop() ?? folder.folder
            return (
              <option key={folder.folder} value={folder.folder}>
                {`${"  ".repeat(depth)}${name} (${formatCount(folder.files)})`}
              </option>
            )
          })}
        </select>
      )}
    </div>
  )
}

/** Every stage, in the order they happen. */
export function StageList({ progress, scope }: { progress: Progress; scope: ProgressScope }) {
  return (
    <ul className="flex flex-col divide-y rounded-lg border bg-card">
      {progress.stages.map((stage) => (
        <StageLine
          key={stage.id}
          stage={stage}
          scope={scope}
          engineChosen={progress.engineChosen}
          backgroundReading={progress.backgroundReading}
        />
      ))}
    </ul>
  )
}

const MARKS: Record<ProgressStage["state"], { icon: typeof Clock; className: string }> = {
  done: { icon: CheckCircle2, className: "text-emerald-600 dark:text-emerald-400" },
  running: { icon: Loader2, className: "animate-spin text-blue-600 dark:text-blue-400" },
  waiting: { icon: Clock, className: "text-muted-foreground" },
  paused: { icon: PauseCircle, className: "text-amber-600 dark:text-amber-400" },
  error: { icon: AlertCircle, className: "text-destructive" },
}

const TONES: Record<ProgressStage["state"], Tone> = {
  done: "ok",
  running: "busy",
  waiting: "neutral",
  paused: "warning",
  error: "error",
}

/** One stage: how it stands and its figures; opened, its documents one by one. */
export function StageLine({
  stage,
  scope,
  engineChosen,
  backgroundReading = true,
  initialOpen = false,
  initialItems = null,
}: {
  stage: ProgressStage
  scope: ProgressScope
  engineChosen: boolean
  backgroundReading?: boolean
  initialOpen?: boolean
  initialItems?: StageItems | null
}) {
  const words = vocabulary()
  // The scans have their own engine and their own buttons (P-242).
  const scans = stage.id === "recognition"
  const recognitionEngine = useRecognitionEngine()
  const recognize = useRecognize()
  const [open, setOpen] = useState(initialOpen)
  const [items, setItems] = useState<StageItems | null>(initialItems)
  const [problem, setProblem] = useState<string | null>(null)
  const text = stageText(stage.id, words)
  const mark = MARKS[stage.state]
  const Icon = mark.icon
  const share = stageShare(stage)
  const details = stageDetails(stage, words)
  // The list is asked again when the figures of the stage change.
  const figuresKey = `${stage.done}/${stage.pending}/${stage.failed}/${stage.out}`
  const scopeKey = `${scope.originId ?? ""}/${scope.folder ?? ""}`

  useEffect(() => {
    if (!open) return
    let current = true
    listStageItems(stage.id, scope.originId ? scope : null, STAGE_ITEMS)
      .then((found) => {
        if (current) {
          setItems(found)
          setProblem(null)
        }
      })
      .catch((cause) => current && setProblem(String(cause)))
    return () => {
      current = false
    }
    // The scope and the figures are compared by their keys, not by object.
  }, [open, stage.id, figuresKey, scopeKey])

  return (
    <li className="px-4 py-3" data-stage={stage.id} data-state={stage.state}>
      <div className="flex items-start gap-3">
        <Icon className={`mt-0.5 size-4 shrink-0 ${mark.className}`} aria-hidden />
        <div className="min-w-0 flex-1">
          <button
            className="flex w-full items-start gap-1 text-left"
            onClick={() => setOpen(!open)}
            aria-expanded={open}
          >
            <span className="min-w-0 flex-1">
              <span className="text-sm font-medium">{text.title}</span>
              <span className="ml-2 align-middle">
                <StatusBadge tone={TONES[stage.state]}>{stateLabel(stage)}</StatusBadge>
              </span>
            </span>
            {open ? (
              <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
            ) : (
              <ChevronRight className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
            )}
          </button>
          <p className="mt-0.5 text-xs text-muted-foreground">{text.explanation}</p>
          <div className="mt-2 flex items-center gap-2">
            {share !== null && (
              <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
                <div
                  className={`h-full rounded-full transition-all ${stage.state === "error" ? "bg-destructive" : "bg-primary"}`}
                  style={{ width: `${share * 100}%` }}
                />
              </div>
            )}
            <span className={`shrink-0 text-xs ${share === null ? "text-muted-foreground" : ""}`}>{stageFigures(stage)}</span>
          </div>
          {details.length > 0 && <p className="mt-1 text-xs text-muted-foreground">{details.join(" · ")}</p>}
          {stage.current && (
            <p className="mt-1 text-xs">
              Ahora: «{stage.current.name}»{stage.current.place && <span className="text-muted-foreground"> · {stage.current.place}</span>}
              {pageOf(stage.current) && <span className="text-muted-foreground"> · {pageOf(stage.current)}</span>}
            </p>
          )}
          {stage.state === "paused" && (
            <p className="mt-1 text-xs text-amber-700 dark:text-amber-400">
              {scans
                ? !recognitionEngine.id
                  ? MISSING_ENGINE
                  : "En pausa: la lectura automática está apagada. Podés reconocerlos a pedido, todos o de a uno."
                : engineChosen
                  ? "En pausa: sigue cuando actives la lectura automática."
                  : "En pausa: falta elegir un modelo en Ajustes."}
            </p>
          )}
          {scans && stage.state === "running" && !backgroundReading && (
            <p className="mt-1 text-xs text-muted-foreground">
              La lectura automática está en pausa: se reconocen solo los que pediste.
            </p>
          )}
          {stage.error && <ModelErrorNote stage={stage} />}
        </div>
        {stage.id === "reading" && stage.failed > 0 && <ReadAgainButton onProblem={setProblem} />}
        {scans && <RecognizeButtons stage={stage} recognize={recognize} engine={recognitionEngine.id} />}
      </div>
      {(problem || (scans && recognize.problem)) && (
        <div className="mt-2 ml-7">
          <ErrorNote>{problem ?? recognize.problem}</ErrorNote>
        </div>
      )}
      {open && (
        <div className="mt-2 ml-7 rounded-md border bg-muted/20">
          {items ? (
            <StageItemsList stage={stage} items={items} words={words} recognize={scans && recognitionEngine.id ? recognize : null} />
          ) : (
            <p className="flex items-center gap-2 px-3 py-2 text-xs text-muted-foreground">
              <Loader2 className="size-3 animate-spin" /> Buscando…
            </p>
          )}
        </div>
      )}
    </li>
  )
}

/** The documents whose reading failed, read again (the action of «Problemas»). */
function ReadAgainButton({ onProblem }: { onProblem: (problem: string | null) => void }) {
  const refresh = useProgressStore((s) => s.refresh)
  const [busy, setBusy] = useState(false)
  async function run() {
    setBusy(true)
    onProblem(null)
    try {
      await knowledgeReadAgainFailed()
      refresh()
    } catch (cause) {
      onProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <Button size="sm" variant="outline" className="shrink-0" disabled={busy} onClick={() => void run()}>
      {busy && <Loader2 className="animate-spin" />}
      Volver a leer los que fallaron
    </Button>
  )
}

/**
 * The scans: all the ones waiting recognized now, with the automatic
 * reading on or off, and the failed ones tried again (P-242).
 */
function RecognizeButtons({ stage, recognize, engine }: { stage: ProgressStage; recognize: Recognize; engine: string | null }) {
  if (stage.pending === 0 && stage.failed === 0) return null
  const disabled = !engine || recognize.busy
  return (
    <div className="flex shrink-0 flex-col items-end gap-1">
      {stage.pending > 0 && stage.state !== "running" && (
        <Button size="sm" variant="outline" disabled={disabled} onClick={() => void recognize.run(null)}>
          {recognize.busy && <Loader2 className="animate-spin" />}
          Reconocer todos
        </Button>
      )}
      {stage.failed > 0 && (
        <Button size="sm" variant="outline" disabled={disabled} onClick={() => void recognize.run(null, true)}>
          Reintentar los que fallaron
        </Button>
      )}
    </div>
  )
}

function ModelErrorNote({ stage }: { stage: ProgressStage }) {
  if (!stage.error) return null
  const { what, when } = modelErrorText(stage.error)
  return (
    <div className="mt-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs">
      <p className="font-medium text-destructive">Último error del modelo: {what}</p>
      <p className="mt-0.5 text-muted-foreground">{when}</p>
      {stage.error.message && (
        <details className="mt-1 text-muted-foreground">
          <summary className="cursor-pointer">Lo que respondió</summary>
          <p className="mt-1 break-words">{stage.error.message}</p>
        </details>
      )}
    </div>
  )
}

const OUT_REASONS: Record<string, string> = {
  "too-large": "demasiado grande",
  unsupported: "un formato sin texto",
  link: "un acceso directo de un disco en la nube",
}

/** The things of a stage, each with how it stands in it and a button to open it. */
export function StageItemsList({
  stage,
  items,
  words,
  recognize = null,
}: {
  stage: ProgressStage
  items: StageItems
  words: Vocabulary
  /** For the scans: a button to recognize each one waiting or failed. */
  recognize?: Recognize | null
}) {
  if (items.items.length === 0) return <EmptyNote>No hay nada en esta etapa.</EmptyNote>
  const more = Math.max(0, items.total - items.items.length)
  return (
    <>
      <ul className="flex flex-col divide-y">
        {items.items.map((item, index) => (
          <li key={`${item.locationId ?? item.pagePath ?? item.originId ?? item.contentId}:${index}`} className="flex items-start gap-3 px-3 py-2 text-sm">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="min-w-0 break-words">{item.name}</span>
                <ItemBadge stage={stage} item={item} />
              </div>
              <div className="text-xs break-words text-muted-foreground">
                {[itemDetail(stage, item, words), item.place].filter(Boolean).join(" · ")}
              </div>
            </div>
            {recognize && item.contentId !== null && (item.status === "pending" || item.status === "failed") && (
              <Button
                size="xs"
                variant="outline"
                className="shrink-0"
                disabled={recognize.busy}
                onClick={() => void recognize.run([item.contentId as number])}
              >
                {item.status === "failed" ? "Reintentar" : "Reconocer"}
              </Button>
            )}
            <ItemAction item={item} />
          </li>
        ))}
      </ul>
      {more > 0 && (
        <p className="border-t px-3 py-2 text-xs text-muted-foreground">
          Y {formatCount(more)} más{stage.unit === "files" ? ": están todos en Documentos." : "."}
        </p>
      )}
    </>
  )
}

function ItemBadge({ stage, item }: { stage: ProgressStage; item: StageItem }) {
  switch (item.status) {
    case "failed":
      return <StatusBadge tone="error">falló</StatusBadge>
    case "pending":
      return <StatusBadge tone={stage.state === "paused" ? "neutral" : "busy"}>{stage.state === "paused" ? "espera" : "falta"}</StatusBadge>
    case "done":
      return <StatusBadge tone="ok">hecho</StatusBadge>
    default:
      return <StatusBadge tone="neutral">queda afuera</StatusBadge>
  }
}

/** Its reason, why it is out, or its matter. */
export function itemDetail(stage: ProgressStage, item: StageItem, words: Vocabulary = vocabulary()): string | null {
  if (item.status === "out") return item.detail ? `Es ${OUT_REASONS[item.detail] ?? item.detail}` : null
  if (stage.id === "matters" && item.status === "done") {
    return item.detail ? `${words.knowledge.matter}: ${item.detail}` : words.knowledge.withoutMatter
  }
  if (stage.id === "origins" && item.status === "failed") return "No se puede leer: revisá que el disco esté conectado."
  if (stage.id === "recognition") {
    if (item.status === "failed") return item.detail ? recognitionFailureText(item.detail) : null
    if (item.status === "pending" && item.pages) return item.pages === 1 ? "1 página" : `${formatCount(item.pages)} páginas`
  }
  return item.detail
}

/** Where its button leads: the document, the page or the folders. */
export function itemTarget(item: StageItem): AttentionTarget | null {
  if (item.locationId !== null) return { view: "document", locationId: item.locationId }
  if (item.pagePath) return { view: "wiki-page", path: item.pagePath }
  if (item.originId) return { view: "origins" }
  return null
}

function ItemAction({ item }: { item: StageItem }) {
  const target = itemTarget(item)
  if (!target) return null
  return (
    <Button size="xs" variant="outline" className="shrink-0" onClick={() => goToTarget(target)}>
      {target.view === "origins" ? "Ver carpetas" : "Abrir"}
    </Button>
  )
}
