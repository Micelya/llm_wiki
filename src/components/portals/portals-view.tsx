import { useEffect, useRef, useState, type ReactNode } from "react"
import { Eye, EyeOff, FolderOpen, Hand, ListChecks, Loader2, RefreshCw, Square, Download, Play } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  PJN,
  fetchRecord,
  listFollowed,
  listJurisdictions,
  openPortalFolder,
  portalGoOn,
  setPortalManual,
  stopPortal,
  updateAllRecords,
  type FollowedRecord,
  type Jurisdiction,
  type PortalStatus,
} from "@/commands/portals"
import { EmptyNote, ErrorNote, Panel, StatusBadge, type Tone } from "@/components/docbase/page"
import { formatAgo, formatCount, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { vocabulary } from "@/vocabulary"
import { portalWindow } from "./portal-window"
import { downloadingAlone, progressFigures, showsPortal } from "./portal-state"
import { publishPortalStatus, usePortalStatus } from "./use-portal-status"
import { usePortalStore } from "@/stores/portal-store"

const STATES: Record<string, { tone: Tone; label: string }> = {
  working: { tone: "busy", label: "Trabajando" },
  "needs-person": { tone: "warning", label: "Te necesita" },
  done: { tone: "ok", label: "Listo" },
  stopped: { tone: "neutral", label: "Detenido" },
  error: { tone: "error", label: "No se pudo" },
}

/** The «Portales» page: the form, the records followed and the portal's window. */
export function PortalsView() {
  const terms = vocabulary().portals
  const status = usePortalStatus()
  const followed = useDocbaseQuery("portals-followed", () => listFollowed(PJN))
  const [problem, setProblem] = useState<string | null>(null)

  // While the page is on screen the end of a walk is seen here; out of
  // it, it is told in the foot of the window and in Atención.
  const setOpen = usePortalStore((s) => s.setOpen)
  useEffect(() => {
    setOpen(true)
    return () => setOpen(false)
  }, [setOpen])

  // When a walk ends, what is followed changed.
  const lastState = useRef<string | null>(null)
  const reloadFollowed = followed.reload
  useEffect(() => {
    const state = status?.progress.state ?? null
    if (state !== lastState.current && state && state !== "working" && state !== "needs-person") reloadFollowed()
    lastState.current = state
  }, [status, reloadFollowed])

  async function run(action: () => Promise<PortalStatus | void>) {
    setProblem(null)
    try {
      const answer = await action()
      if (answer) publishPortalStatus(answer)
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  // «Ver el portal» while the app downloads alone; forgotten when the walk
  // in course changes.
  const [asked, setAsked] = useState(false)
  const running = status?.running ?? null
  useEffect(() => {
    setAsked(false)
  }, [running])
  const shown = showsPortal(status, asked)

  const busy = Boolean(status?.running)
  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-start justify-between gap-4 border-b bg-background px-10 pt-7 pb-5">
        <div className="min-w-0">
          <h1 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">{terms.name}</h1>
          <p className="mt-1 font-ui text-sm font-medium text-muted-foreground">{terms.description}</p>
        </div>
        <Button variant="outline" onClick={() => run(() => openPortalFolder(PJN))}>
          <FolderOpen /> Abrir la carpeta
        </Button>
      </header>
      <div className="flex min-h-0 flex-1">
        <aside className="w-[360px] shrink-0 space-y-4 overflow-y-auto border-r bg-panel-sub p-4">
          {problem && <ErrorNote>{problem}</ErrorNote>}
          <FetchForm portalName={followed.data?.name} busy={busy} onFetch={(j, n, y) => run(() => fetchRecord(PJN, j, n, y))} />
          {status && status.progress.state && shown && <StatusPanel status={status} onRun={run} />}
          <Panel
            title={`${terms.records} seguidos`}
            count={followed.data?.records.length ?? 0}
            action={
              <Button
                size="sm"
                variant="outline"
                disabled={!followed.data?.records.length}
                onClick={() => run(() => updateAllRecords(PJN))}
              >
                <RefreshCw /> Actualizar todos
              </Button>
            }
          >
            {followed.error && <ErrorNote>{followed.error}</ErrorNote>}
            {followed.data && followed.data.records.length === 0 && (
              <EmptyNote>Todavía no trajiste ninguno. Cada uno que traigas queda acá, para actualizarlo cuando quieras.</EmptyNote>
            )}
            <ul className="divide-y">
              {followed.data?.records.map((record) => (
                <FollowedItem
                  key={record.folder}
                  record={record}
                  running={status?.running === record.reference}
                  onUpdate={() => run(() => fetchRecord(PJN, record.jurisdiction, String(record.number), String(record.year)))}
                  onReviewAll={() =>
                    run(() => fetchRecord(PJN, record.jurisdiction, String(record.number), String(record.year), true))
                  }
                  onFolder={() => run(() => openPortalFolder(PJN, folderName(record)))}
                  onFetchLinked={(j, n, y) => run(() => fetchRecord(PJN, j, n, y))}
                />
              ))}
            </ul>
            {followed.data && (
              <p className="mt-3 break-all text-xs text-muted-foreground">Se guardan en {followed.data.folder}</p>
            )}
          </Panel>
        </aside>
        <section className="flex min-w-0 flex-1 flex-col">
          <LockBar status={status} onRun={run} asked={asked} onHidePortal={() => setAsked(false)} />
          {shown || !status ? (
            <PortalArea onProblem={setProblem} />
          ) : (
            <DownloadProgress status={status} onShowPortal={() => setAsked(true)} onRun={run} />
          )}
        </section>
      </div>
    </div>
  )
}

function folderName(record: FollowedRecord): string {
  return `${record.jurisdiction} ${record.number}-${record.year}`
}

function FetchForm({
  portalName,
  busy,
  onFetch,
}: {
  portalName: string | undefined
  busy: boolean
  onFetch: (jurisdiction: string, number: string, year: string) => void
}) {
  const terms = vocabulary().portals
  const [jurisdictions, setJurisdictions] = useState<Jurisdiction[]>([])
  const [jurisdiction, setJurisdiction] = useState("")
  const [number, setNumber] = useState("")
  const [year, setYear] = useState("")
  useEffect(() => {
    listJurisdictions(PJN).then(setJurisdictions).catch(() => setJurisdictions([]))
  }, [])
  const ready = jurisdiction !== "" && /^\d+$/.test(number.trim()) && /^\d{4}$/.test(year.trim())
  return (
    <Panel title={`Traer un ${terms.record.toLowerCase()}`}>
      <form
        className="space-y-3"
        onSubmit={(event) => {
          event.preventDefault()
          if (!ready) return
          onFetch(jurisdiction, number.trim(), year.trim())
          setNumber("")
          setYear("")
        }}
      >
        {portalName && <p className="text-xs text-muted-foreground">{portalName}</p>}
        <label className="block text-sm">
          <span className="mb-1 block font-ui font-semibold">Jurisdicción</span>
          <select
            className="h-8 w-full rounded-lg border bg-card px-2 text-sm"
            value={jurisdiction}
            onChange={(event) => setJurisdiction(event.target.value)}
          >
            <option value="">Elegí la jurisdicción…</option>
            {jurisdictions.map((j) => (
              <option key={j.code} value={j.code}>
                {j.code} · {j.name}
              </option>
            ))}
          </select>
        </label>
        <div className="flex gap-2">
          <label className="block flex-1 text-sm">
            <span className="mb-1 block font-ui font-semibold">Número</span>
            <Input inputMode="numeric" value={number} onChange={(event) => setNumber(event.target.value)} />
          </label>
          <label className="block w-24 text-sm">
            <span className="mb-1 block font-ui font-semibold">Año</span>
            <Input inputMode="numeric" maxLength={4} value={year} onChange={(event) => setYear(event.target.value)} />
          </label>
        </div>
        <Button type="submit" className="w-full" disabled={!ready}>
          <Download /> {busy ? "Traer después del actual" : "Traer"}
        </Button>
        <p className="text-xs text-muted-foreground">
          La app completa el formulario del portal y baja cada documento nuevo. Si el portal pide resolver el desafío,
          resolvelo en la ventana de la derecha.
        </p>
      </form>
    </Panel>
  )
}

function StatusPanel({ status, onRun }: { status: PortalStatus; onRun: (action: () => Promise<PortalStatus | void>) => void }) {
  const { progress } = status
  const shown = STATES[progress.state] ?? { tone: "neutral" as Tone, label: progress.state }
  const counts = [
    progress.page ? `página ${progress.page}` : null,
    progress.seen ? plural(progress.seen, "actuación vista", "actuaciones vistas") : null,
    progress.withDocument ? plural(progress.withDocument, "con documento", "con documento") : null,
    progress.downloaded ? plural(progress.downloaded, "bajado ahora", "bajados ahora") : null,
    progress.already ? `${progress.already} ya estaban` : null,
    progress.failed ? `${progress.failed} no se pudieron bajar` : null,
  ].filter(Boolean)
  return (
    <Panel title={progress.reference ?? "El portal"} action={<StatusBadge tone={shown.tone}>{shown.label}</StatusBadge>}>
      <div className="space-y-2 text-sm">
        <p>{progress.message}</p>
        {counts.length > 0 && <p className="text-xs text-muted-foreground">{counts.join(" · ")}</p>}
        {status.queue.length > 0 && <p className="text-xs text-muted-foreground">Después: {status.queue.join(", ")}</p>}
        {progress.warnings.length > 0 && (
          <ul className="list-disc space-y-1 pl-4 text-xs text-warn">
            {progress.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        )}
        <div className="flex flex-wrap gap-2">
          {progress.state === "needs-person" && (
            <Button size="sm" onClick={() => onRun(() => portalGoOn())}>
              <Play /> Ya lo resolví, seguir
            </Button>
          )}
          {status.running && (
            <Button size="sm" variant="outline" onClick={() => onRun(() => stopPortal())}>
              <Square /> Detener
            </Button>
          )}
        </div>
      </div>
    </Panel>
  )
}

function FollowedItem({
  record,
  running,
  onUpdate,
  onReviewAll,
  onFolder,
  onFetchLinked,
}: {
  record: FollowedRecord
  running: boolean
  onUpdate: () => void
  onReviewAll: () => void
  onFolder: () => void
  onFetchLinked: (jurisdiction: string, number: string, year: string) => void
}) {
  const knowledge = vocabulary().knowledge
  const updated = record.updatedAt ? Date.parse(record.updatedAt) : NaN
  return (
    <li className="space-y-1.5 py-3 text-sm">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="font-ui font-bold text-strong">{record.reference}</p>
          {record.title && <p className="text-xs text-muted-foreground">{record.title}</p>}
        </div>
        {record.matterId && (
          <StatusBadge tone="ok">
            {knowledge.matter} {knowledge.matterStates.confirmed.toLowerCase()}
          </StatusBadge>
        )}
      </div>
      {(record.office || record.situation) && (
        <p className="text-xs text-muted-foreground">{[record.office, record.situation].filter(Boolean).join(" · ")}</p>
      )}
      <p className="text-xs text-muted-foreground">
        {Number.isNaN(updated) ? "Todavía sin traer" : `Actualizado ${formatAgo(updated)}`} ·{" "}
        {plural(record.withDocument, "documento", "documentos")} ({record.downloaded + record.already} en la base
        {record.failed > 0 ? `, ${record.failed} por reintentar` : ""})
      </p>
      {record.lastResult && <p className="text-xs">{record.lastResult}</p>}
      <div className="flex flex-wrap gap-2">
        <Button
          size="xs"
          variant="outline"
          disabled={running}
          onClick={onUpdate}
          title="Trae lo nuevo: recorre las páginas hasta la primera que ya estaba entera"
        >
          <RefreshCw /> {running ? "Actualizando…" : "Actualizar"}
        </Button>
        {!Number.isNaN(updated) && (
          <Button
            size="xs"
            variant="ghost"
            disabled={running}
            onClick={onReviewAll}
            title="Recorre todas las páginas del expediente, también las que ya estaban, y trae lo que falte"
          >
            <ListChecks /> Revisar todo el expediente
          </Button>
        )}
        <Button size="xs" variant="ghost" onClick={onFolder}>
          <FolderOpen /> Carpeta
        </Button>
      </div>
      {record.linked.length > 0 && (
        <div className="space-y-1 pt-1">
          <p className="text-xs font-medium">Vinculados</p>
          {record.linked.map((link) => (
            <div key={link.reference} className="flex items-center justify-between gap-2 text-xs">
              <span className="min-w-0 truncate">
                {link.reference}
                {link.detail ? ` · ${link.detail}` : ""}
              </span>
              {link.followed ? (
                <span className="text-muted-foreground">seguido</span>
              ) : (
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() => onFetchLinked(link.jurisdiction, String(link.number), String(link.year))}
                >
                  Traer
                </Button>
              )}
            </div>
          ))}
        </div>
      )}
    </li>
  )
}

/** Says who has the portal's window now, and lets the person take it. */
function LockBar({
  status,
  onRun,
  asked,
  onHidePortal,
}: {
  status: PortalStatus | null
  onRun: (action: () => Promise<PortalStatus | void>) => void
  asked: boolean
  onHidePortal: () => void
}) {
  const progress = status?.progress
  if (downloadingAlone(status)) {
    return asked ? (
      <Bar tone="busy" text="Estás viendo el portal mientras Micelya baja: no hace falta tocar nada.">
        <Button size="xs" variant="outline" onClick={onHidePortal}>
          <EyeOff /> Volver al avance
        </Button>
      </Bar>
    ) : (
      <Bar
        tone="busy"
        text="Micelya está bajando los documentos con la ventana del portal oculta. Podés ir a otras pantallas: sigue sola y avisa al terminar."
      />
    )
  }
  if (status?.manual) {
    return (
      <Bar tone="neutral" text="La ventana es tuya: Micelya no la toca hasta que se la devuelvas.">
        <Button size="xs" variant="outline" onClick={() => onRun(() => setPortalManual(PJN, false))}>
          Devolver a Micelya
        </Button>
      </Bar>
    )
  }
  if (status?.running && progress?.busy) {
    return (
      <Bar
        tone="busy"
        text="El portal está procesando la consulta: Micelya espera sin tocar nada. La ventana queda libre por si querés tocar «Cancelar» en el portal."
      />
    )
  }
  if (status?.running && progress?.state === "needs-person") {
    const text =
      progress.person === "login"
        ? "Te toca: el portal pide iniciar sesión. Hacelo en la ventana; Micelya sigue sola cuando la página cambie."
        : "Te toca: resolvé el desafío en el recuadro resaltado de la ventana del portal y después tocá «Consultar». Micelya sigue sola cuando la página cambie."
    return (
      <Bar tone="warning" text={text}>
        <Button size="xs" onClick={() => onRun(() => portalGoOn())}>
          Ya lo resolví, seguir
        </Button>
      </Bar>
    )
  }
  if (status?.running) {
    return <Bar tone="busy" text="Micelya está usando la ventana del portal: no hace falta tocar nada." />
  }
  return (
    <Bar tone="neutral" text="Micelya maneja esta ventana y te la pasa sola cuando el portal pide un desafío o iniciar sesión.">
      <Button size="xs" variant="ghost" onClick={() => onRun(() => setPortalManual(PJN, true))}>
        <Hand /> Usar la ventana yo
      </Button>
    </Bar>
  )
}

/**
 * The walk in course, with room, while the portal's window is hidden: the
 * page, the entries seen, the documents downloaded and what is left, the
 * document in course and what did not add up.
 */
function DownloadProgress({
  status,
  onShowPortal,
  onRun,
}: {
  status: PortalStatus
  onShowPortal: () => void
  onRun: (action: () => Promise<PortalStatus | void>) => void
}) {
  const { progress } = status
  const terms = vocabulary().portals
  return (
    <div className="min-h-0 flex-1 overflow-y-auto bg-panel-sub p-6" data-testid="portal-progress">
      <div className="relative mx-auto max-w-2xl space-y-5 overflow-hidden rounded-xl border bg-card p-6 shadow-card">
        <span aria-hidden className="absolute inset-x-0 top-0 h-[3px] bg-grad-vibrante" />
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="font-ui text-xs font-bold tracking-[0.08em] text-muted-foreground uppercase">Bajando el {terms.record.toLowerCase()}</p>
            <h2 className="font-ui text-xl font-bold text-strong">{progress.reference ?? status.running}</h2>
          </div>
          <StatusBadge tone="busy" icon={Loader2} spin>
            Trabajando
          </StatusBadge>
        </div>
        {/* «Bajando …» is said below, with room for the name. */}
        {!(progress.current && progress.message === `Bajando ${progress.current}`) && <p className="text-sm">{progress.message}</p>}
        <dl className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {progressFigures(status).map((figure) => (
            <div key={figure.label} className="rounded-[10px] border bg-panel-sub px-3.5 py-3">
              <dt className="text-[0.8125rem] text-muted-foreground">{figure.label}</dt>
              <dd className="font-ui text-[1.625rem] leading-[1.1] font-bold text-strong tabular-nums">{formatCount(figure.value)}</dd>
            </div>
          ))}
        </dl>
        {progress.current && (
          <div className="text-sm">
            <p className="text-xs text-muted-foreground">Ahora</p>
            <p className="break-all font-mono text-xs">{progress.current}</p>
          </div>
        )}
        {status.queue.length > 0 && <p className="text-xs text-muted-foreground">Después: {status.queue.join(", ")}</p>}
        {progress.warnings.length > 0 && (
          <ul className="list-disc space-y-1 pl-4 text-xs text-warn">
            {progress.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        )}
        <p className="text-xs text-muted-foreground">
          Podés ir a otras pantallas y seguir trabajando: Micelya sigue bajando y avisa al terminar en el pie de la ventana y en
          Atención. Si el portal pide resolver el desafío, la ventana del portal vuelve sola.
        </p>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="outline" onClick={onShowPortal}>
            <Eye /> Ver el portal
          </Button>
          <Button size="sm" variant="outline" onClick={() => onRun(() => stopPortal())}>
            <Square /> Detener
          </Button>
        </div>
      </div>
    </div>
  )
}

const BAR_TONES: Record<"neutral" | "busy" | "warning", string> = {
  neutral: "bg-panel-sub text-muted-foreground",
  busy: "bg-info-bg text-info",
  warning: "bg-warn-bg text-warn",
}

function Bar({ tone, text, children }: { tone: keyof typeof BAR_TONES; text: string; children?: ReactNode }) {
  return (
    <div className={`flex min-h-10 shrink-0 items-center justify-between gap-3 border-b px-4 py-1.5 text-sm ${BAR_TONES[tone]}`}>
      <span>{text}</span>
      {children}
    </div>
  )
}

/**
 * The area the portal's window goes over. The window is drawn by the
 * system on top of the page, so it follows this area's place and size,
 * and it is hidden when the page goes (`portal-window.ts`).
 */
function PortalArea({ onProblem }: { onProblem: (problem: string) => void }) {
  const area = useRef<HTMLDivElement>(null)
  const report = useRef(onProblem)
  useEffect(() => {
    report.current = onProblem
  })
  useEffect(() => {
    const element = area.current
    if (!element) return
    const portal = portalWindow(PJN)
    let gone = false
    const sync = () => {
      if (gone) return
      const box = element.getBoundingClientRect()
      const rect = { x: Math.round(box.left), y: Math.round(box.top), width: Math.round(box.width), height: Math.round(box.height) }
      portal.show(rect).catch((cause) => {
        if (!gone) report.current(String(cause))
      })
    }
    sync()
    const observer = new ResizeObserver(sync)
    observer.observe(element)
    window.addEventListener("resize", sync)
    // A panel that opens or closes moves the area without resizing it.
    const timer = window.setInterval(sync, 700)
    return () => {
      gone = true
      observer.disconnect()
      window.removeEventListener("resize", sync)
      window.clearInterval(timer)
      portal.hide().catch(() => {})
    }
  }, [])
  return <div ref={area} className="min-h-0 flex-1 bg-muted/30" />
}
