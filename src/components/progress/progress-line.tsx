import { CircleAlert, CircleCheck, CirclePause, LoaderCircle } from "lucide-react"
import type { Progress } from "@/commands/docbase"
import { activityLine, activityShare, type ActivityTone } from "@/lib/progress/stages"
import { useProgressStore } from "@/stores/progress-store"
import { vocabulary } from "@/vocabulary"
import { goToTarget } from "@/components/attention/navigate"
import { formatCount } from "@/components/docbase/format"

const ICONS: Record<ActivityTone, { icon: typeof LoaderCircle; className: string }> = {
  busy: { icon: LoaderCircle, className: "animate-spin text-info" },
  error: { icon: CircleAlert, className: "text-err" },
  paused: { icon: CirclePause, className: "text-warn" },
  ok: { icon: CircleCheck, className: "text-ok" },
}

/**
 * The line fixed at the foot of the work area (D-426): what is being
 * processed now and how far it went («Procesando: …, n de N»), with a bar
 * in the brand's gradient and how many documents wait paused, with an
 * icon that turns while there is work, or why it stopped, or that
 * everything is up to date. It leads to Atención → «En curso».
 */
export function ProgressLine({ progress: given }: { progress?: Progress | null }) {
  const stored = useProgressStore((s) => s.progress)
  const progress = given === undefined ? stored : given
  const line = activityLine(progress)
  if (!line) return null
  const share = line.tone === "busy" ? activityShare(progress) : null
  const paused = line.tone === "busy" ? (progress?.paused ?? 0) : 0
  const { icon: Icon, className } = ICONS[line.tone]
  return (
    <button
      data-tone={line.tone}
      onClick={() => goToTarget({ view: "attention", tab: "ongoing" })}
      className="flex h-[34px] w-full shrink-0 items-center gap-2.5 border-t bg-bar px-5 text-left font-ui text-[0.8125rem] font-medium text-muted-foreground transition-colors hover:bg-hover"
      title={`Ver en ${vocabulary().attention.name} → En curso`}
    >
      <Icon className={`size-4 shrink-0 ${className}`} aria-hidden />
      <span className="min-w-0 truncate">{line.text}</span>
      {share !== null && (
        <span aria-hidden className="h-1 w-[120px] shrink-0 overflow-hidden rounded-full bg-border">
          <span className="block h-full rounded-full bg-grad-vibrante" style={{ width: `${Math.round(share * 100)}%` }} />
        </span>
      )}
      {paused > 0 && (
        <>
          <span aria-hidden className="text-border-strong">·</span>
          <span className="shrink-0">{formatCount(paused)} en pausa</span>
        </>
      )}
      <span className="ml-auto shrink-0 pl-2 font-semibold text-link">Ver «En curso»</span>
    </button>
  )
}
