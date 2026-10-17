import { AlertCircle, ChevronRight, Loader2 } from "lucide-react"
import { useWikiStore } from "@/stores/wiki-store"
import { noticesOf } from "@/lib/attention/order"
import type { AttentionNotice } from "@/lib/attention/types"
import { useAttention } from "@/components/attention/use-attention"
import { QUEUE_SOURCE } from "@/components/attention/sources/wiki-queue"
import { FILE_SYNC_SOURCE } from "@/components/attention/sources/file-sync"
import { vocabulary } from "@/vocabulary"

/** The wiki's own lines, the ones the old activity panel showed. */
const WIKI_SOURCES = new Set([QUEUE_SOURCE, FILE_SYNC_SOURCE])

/**
 * What is left of the activity panel at the foot of the wiki's side
 * panel: one line that says what the wiki is doing or that something of
 * it failed, and leads to the attention view, where it is controlled.
 * Nothing when the wiki has nothing to say.
 */
export function ActivityLink() {
  const { notices } = useAttention()
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const own = notices.filter((notice) => WIKI_SOURCES.has(notice.source))
  const line = activityLine(own)
  if (!line) return null
  return (
    <button
      onClick={() => setActiveView("attention")}
      className="flex w-full items-center gap-2 border-t bg-muted/30 px-3 py-1.5 text-left text-xs text-muted-foreground hover:bg-accent/50"
      title={`Ver en ${vocabulary().attention.name}`}
    >
      {line.problem ? (
        <AlertCircle className="h-3 w-3 shrink-0 text-destructive" />
      ) : (
        <Loader2 className="h-3 w-3 shrink-0 animate-spin" />
      )}
      <span className="min-w-0 flex-1 truncate">{line.text}</span>
      <ChevronRight className="h-3 w-3 shrink-0" />
    </button>
  )
}

/** The one line: a problem first, otherwise what is in progress. */
export function activityLine(notices: readonly AttentionNotice[]): { text: string; problem: boolean } | null {
  const problems = noticesOf(notices, "problem")
  if (problems.length > 0) return { text: problems[0].title, problem: true }
  const ongoing = noticesOf(notices, "ongoing").filter((notice) => notice.priority !== "low")
  if (ongoing.length > 0) return { text: ongoing[0].title, problem: false }
  return null
}
