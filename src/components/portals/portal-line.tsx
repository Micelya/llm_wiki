import { CircleAlert, CircleCheck, Hand, Landmark, LoaderCircle } from "lucide-react"
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

const ICONS: Record<PortalTone, { icon: typeof LoaderCircle; className: string }> = {
  busy: { icon: LoaderCircle, className: "animate-spin text-info" },
  person: { icon: Hand, className: "text-warn" },
  ok: { icon: CircleCheck, className: "text-ok" },
  stopped: { icon: Landmark, className: "text-muted-foreground" },
  error: { icon: CircleAlert, className: "text-err" },
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
      className="flex h-[34px] w-full shrink-0 items-center gap-2.5 border-t bg-bar px-5 text-left font-ui text-[0.8125rem] font-medium text-muted-foreground transition-colors hover:bg-hover"
      title={`Ir a ${vocabulary().portals.name}`}
    >
      <Icon className={`size-4 shrink-0 ${className}`} aria-hidden />
      <span className="min-w-0 flex-1 truncate">{line.text}</span>
      <span className="shrink-0 pl-2 font-semibold text-link">Ver «{vocabulary().portals.name}»</span>
    </button>
  )
}
