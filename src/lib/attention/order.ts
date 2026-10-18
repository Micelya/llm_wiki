import type { AttentionNotice, NoticeKind, NoticePriority } from "./types"

/**
 * How the attention view orders and counts what the sources raise. It
 * only gathers: each source decides what it raises and when it stops.
 */

const KIND_ORDER: Record<NoticeKind, number> = { problem: 0, decision: 1, ongoing: 2 }
const PRIORITY_ORDER: Record<NoticePriority, number> = { high: 0, normal: 1, low: 2 }

/**
 * Every notice of every source, problems first, then decisions, then
 * what is in progress; inside each class by priority, and then in the
 * order the sources are registered and each one listed them.
 */
export function orderNotices(bySource: ReadonlyArray<readonly AttentionNotice[]>): AttentionNotice[] {
  const all = bySource.flat()
  return all
    .map((notice, index) => ({ notice, index }))
    .sort(
      (a, b) =>
        KIND_ORDER[a.notice.kind] - KIND_ORDER[b.notice.kind] ||
        PRIORITY_ORDER[a.notice.priority] - PRIORITY_ORDER[b.notice.priority] ||
        a.index - b.index,
    )
    .map(({ notice }) => notice)
}

export interface AttentionCounts {
  /** Lines of problems: each one is something that does not work. */
  problems: number
  /** Decisions a person has to make: a family of documents is one. */
  decisions: number
  /** Lines of what is in progress. Not part of the counter. */
  ongoing: number
}

/**
 * What the counters say. Low-priority lines are not counted: they
 * explain something that stays out, or that can wait.
 */
export function attentionCounts(notices: readonly AttentionNotice[]): AttentionCounts {
  let problems = 0
  let decisions = 0
  let ongoing = 0
  for (const notice of notices) {
    if (notice.kind === "ongoing") ongoing += 1
    else if (notice.priority === "low") continue
    else if (notice.kind === "problem") problems += 1
    else decisions += Math.max(0, notice.count)
  }
  return { problems, decisions, ongoing }
}

/** The counter of the navigation: problems and decisions, not what is in progress. */
export function navigationCount(counts: AttentionCounts): number {
  return counts.problems + counts.decisions
}

/** "2 problemas · 9 decisiones"; `null` when nothing needs a person. */
export function summaryText(counts: AttentionCounts): string | null {
  const parts: string[] = []
  if (counts.problems > 0) parts.push(`${counts.problems} ${counts.problems === 1 ? "problema" : "problemas"}`)
  if (counts.decisions > 0) parts.push(`${counts.decisions} ${counts.decisions === 1 ? "decisión" : "decisiones"}`)
  return parts.length > 0 ? parts.join(" · ") : null
}

/** The notices of one class, in order. */
export function noticesOf(notices: readonly AttentionNotice[], kind: NoticeKind): AttentionNotice[] {
  return notices.filter((notice) => notice.kind === kind)
}
