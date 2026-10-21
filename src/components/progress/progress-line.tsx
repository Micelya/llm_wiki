import { AlertCircle, CheckCircle2, ChevronRight, Loader2, PauseCircle } from "lucide-react"
import type { Progress } from "@/commands/docbase"
import { activityLine, type ActivityTone } from "@/lib/progress/stages"
import { useProgressStore } from "@/stores/progress-store"
import { vocabulary } from "@/vocabulary"
import { goToTarget } from "@/components/attention/navigate"

const ICONS: Record<ActivityTone, { icon: typeof Loader2; className: string }> = {
  busy: { icon: Loader2, className: "animate-spin text-blue-600 dark:text-blue-400" },
  error: { icon: AlertCircle, className: "text-destructive" },
  paused: { icon: PauseCircle, className: "text-amber-600 dark:text-amber-400" },
  ok: { icon: CheckCircle2, className: "text-emerald-600 dark:text-emerald-400" },
}

/**
 * The line fixed at the foot of the window, under the menu (D-426): what
 * is being processed now and how far it went («Procesando: …, n de N»),
 * with an icon that turns while there is work, or why it stopped, or that
 * everything is up to date. It leads to Atención → «En curso».
 */
export function ProgressLine({ progress: given }: { progress?: Progress | null }) {
  const stored = useProgressStore((s) => s.progress)
  const line = activityLine(given === undefined ? stored : given)
  if (!line) return null
  const { icon: Icon, className } = ICONS[line.tone]
  return (
    <button
      data-tone={line.tone}
      onClick={() => goToTarget({ view: "attention", tab: "ongoing" })}
      className="flex h-7 w-full shrink-0 items-center gap-2 border-t bg-muted/50 px-3 text-left text-xs text-muted-foreground hover:bg-accent/50"
      title={`Ver en ${vocabulary().attention.name} → En curso`}
    >
      <Icon className={`size-3.5 shrink-0 ${className}`} aria-hidden />
      <span className="min-w-0 flex-1 truncate">{line.text}</span>
      <ChevronRight className="size-3.5 shrink-0" aria-hidden />
    </button>
  )
}
