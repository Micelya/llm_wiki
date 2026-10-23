import { AlertCircle, CheckCircle2, ChevronRight, Hand, Landmark, Loader2 } from "lucide-react"
import type { PortalStatus } from "@/commands/portals"
import { goToTarget } from "@/components/attention/navigate"
import { walkFigures } from "@/components/attention/sources/portals"
import { usePortalStore } from "@/stores/portal-store"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { usePortalStatus } from "./use-portal-status"

export type PortalTone = "busy" | "person" | "ok" | "stopped" | "error"

/**
 * The line of the portals at the foot of the window, from any screen: the
 * walk in course and how far it went, that the portal waits for the
 * person, or how the last walk ended while she was in other screens,
 * until she opens «Portales». `null` when there is nothing to tell.
 */
export function portalLine(
  status: PortalStatus | null,
  endUnseen: boolean,
  words: Vocabulary = vocabulary(),
): { tone: PortalTone; text: string } | null {
  if (!status) return null
  const p = status.progress
  const name = words.portals.name
  const record = p.reference ?? status.running ?? ""
  if (status.running) {
    if (p.state === "needs-person") {
      const what = p.person === "login" ? "iniciar sesión" : "resolver el desafío"
      return { tone: "person", text: `${name}: el portal te necesita para ${what} y seguir con ${record}` }
    }
    return { tone: "busy", text: `${name}: trayendo ${record} · ${walkFigures(status)}` }
  }
  if (!endUnseen) return null
  switch (p.state) {
    case "done":
      return { tone: "ok", text: `${name}: terminó. ${p.message}` }
    case "stopped":
      return { tone: "stopped", text: `${name}: se detuvo ${record}. Lo que se bajó queda guardado.` }
    case "error":
      return { tone: "error", text: `${name}: no se pudo terminar ${record}. ${p.message}` }
    default:
      return null
  }
}

const ICONS: Record<PortalTone, { icon: typeof Loader2; className: string }> = {
  busy: { icon: Loader2, className: "animate-spin text-blue-600 dark:text-blue-400" },
  person: { icon: Hand, className: "text-amber-600 dark:text-amber-400" },
  ok: { icon: CheckCircle2, className: "text-emerald-600 dark:text-emerald-400" },
  stopped: { icon: Landmark, className: "text-muted-foreground" },
  error: { icon: AlertCircle, className: "text-destructive" },
}

/** The line itself, over the line of the processing (D-426); it leads to «Portales». */
export function PortalLine() {
  const status = usePortalStatus()
  const endUnseen = usePortalStore((s) => s.endUnseen)
  const line = portalLine(status, endUnseen)
  if (!line) return null
  const { icon: Icon, className } = ICONS[line.tone]
  return (
    <button
      data-tone={line.tone}
      onClick={() => goToTarget({ view: "portals" })}
      className="flex h-7 w-full shrink-0 items-center gap-2 border-t bg-muted/50 px-3 text-left text-xs text-muted-foreground hover:bg-accent/50"
      title={`Ir a ${vocabulary().portals.name}`}
    >
      <Icon className={`size-3.5 shrink-0 ${className}`} aria-hidden />
      <span className="min-w-0 flex-1 truncate">{line.text}</span>
      <ChevronRight className="size-3.5 shrink-0" aria-hidden />
    </button>
  )
}
