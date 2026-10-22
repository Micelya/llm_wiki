import { useEffect, useRef, useState, type ReactNode } from "react"
import { FolderOpen, Hand, RefreshCw, Square, Download, Play } from "lucide-react"
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
import { formatAgo, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { vocabulary } from "@/vocabulary"
import { portalWindow } from "./portal-window"
import { publishPortalStatus, usePortalStatus } from "./use-portal-status"

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

  const busy = Boolean(status?.running)
  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-start justify-between gap-4 border-b px-8 py-5">
        <div className="min-w-0">
          <h1 className="text-xl font-semibold">{terms.name}</h1>
          <p className="mt-1 text-sm text-muted-foreground">{terms.description}</p>
        </div>
        <Button variant="outline" onClick={() => run(() => openPortalFolder(PJN))}>
          <FolderOpen /> Abrir la carpeta
        </Button>
      </header>
      <div className="flex min-h-0 flex-1">
        <aside className="w-[360px] shrink-0 space-y-4 overflow-y-auto border-r p-4">
          {problem && <ErrorNote>{problem}</ErrorNote>}
          <FetchForm portalName={followed.data?.name} busy={busy} onFetch={(j, n, y) => run(() => fetchRecord(PJN, j, n, y))} />
          {status && status.progress.state && <StatusPanel status={status} onRun={run} />}
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
          <LockBar status={status} onRun={run} />
          <PortalArea onProblem={setProblem} />
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
          <span className="mb-1 block font-medium">Jurisdicción</span>
          <select
            className="h-8 w-full rounded-lg border bg-transparent px-2 text-sm"
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
            <span className="mb-1 block font-medium">Número</span>
            <Input inputMode="numeric" value={number} onChange={(event) => setNumber(event.target.value)} />
          </label>
          <label className="block w-24 text-sm">
            <span className="mb-1 block font-medium">Año</span>
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
          <ul className="list-disc space-y-1 pl-4 text-xs text-amber-700 dark:text-amber-400">
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
  onFolder,
  onFetchLinked,
}: {
  record: FollowedRecord
  running: boolean
  onUpdate: () => void
  onFolder: () => void
  onFetchLinked: (jurisdiction: string, number: string, year: string) => void
}) {
  const knowledge = vocabulary().knowledge
  const updated = record.updatedAt ? Date.parse(record.updatedAt) : NaN
  return (
    <li className="space-y-1.5 py-3 text-sm">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="font-medium">{record.reference}</p>
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
      <div className="flex gap-2">
        <Button size="xs" variant="outline" disabled={running} onClick={onUpdate}>
          <RefreshCw /> {running ? "Actualizando…" : "Actualizar"}
        </Button>
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
function LockBar({ status, onRun }: { status: PortalStatus | null; onRun: (action: () => Promise<PortalStatus | void>) => void }) {
  const progress = status?.progress
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
    const what = progress.person === "login" ? "iniciar sesión" : "resolver el desafío"
    return (
      <Bar tone="warning" text={`Te toca: el portal pide ${what}. Hacelo en la ventana; Micelya sigue sola cuando la página cambie.`}>
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

const BAR_TONES: Record<"neutral" | "busy" | "warning", string> = {
  neutral: "bg-muted/50 text-muted-foreground",
  busy: "bg-blue-500/10 text-blue-700 dark:text-blue-400",
  warning: "bg-amber-500/15 text-amber-800 dark:text-amber-300",
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
