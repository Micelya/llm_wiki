import { getGeneralProject } from "@/commands/docbase"
import type { AttentionNotice } from "@/lib/attention/types"
import { normalizePath } from "@/lib/path-utils"
import type { LintItem } from "@/stores/lint-store"
import { useLintStore } from "@/stores/lint-store"
import type { ReviewItem } from "@/stores/review-store"
import { useReviewStore } from "@/stores/review-store"
import { useWikiStore } from "@/stores/wiki-store"
import { loadLintItems, loadReviewItems } from "@/lib/persist"
import { useEffect, useState } from "react"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"

/**
 * What the wiki of the open project raises: the proposals its review
 * keeps (contradictions, pages that are missing…) and the links its
 * structural check found broken. Both are of the project that is open;
 * when it is not the archive, the line says whose they are.
 */
export const REVIEW_SOURCE = "wiki-review"
export const LINT_SOURCE = "wiki-lint"

/** Proposals listed when the line is opened; the rest are in Revisión. */
export const REVIEW_CASES = 20

const REVIEW_KINDS: Record<ReviewItem["type"], [string, string]> = {
  contradiction: ["contradicción entre páginas", "contradicciones entre páginas"],
  duplicate: ["página duplicada", "páginas duplicadas"],
  "missing-page": ["página que falta", "páginas que faltan"],
  confirm: ["confirmación", "confirmaciones"],
  suggestion: ["sugerencia", "sugerencias"],
}

/** "1 contradicción entre páginas y 2 páginas que faltan". */
export function reviewKindsText(items: readonly ReviewItem[]): string {
  const counts = new Map<ReviewItem["type"], number>()
  for (const item of items) counts.set(item.type, (counts.get(item.type) ?? 0) + 1)
  const parts = [...counts.entries()].map(([type, n]) => plural(n, ...(REVIEW_KINDS[type] ?? [type, type])))
  return parts.length <= 1 ? (parts[0] ?? "") : `${parts.slice(0, -1).join(", ")} y ${parts[parts.length - 1]}`
}

export function reviewNotices(items: readonly ReviewItem[], scope: string | undefined): AttentionNotice[] {
  const pending = items.filter((item) => !item.resolved)
  if (pending.length === 0) return []
  return [
    {
      source: REVIEW_SOURCE,
      id: `${REVIEW_SOURCE}:proposals`,
      kind: "decision",
      topic: "wiki-review",
      priority: "normal",
      count: pending.length,
      title: plural(pending.length, "propuesta del wiki espera tu revisión", "propuestas del wiki esperan tu revisión"),
      explanation: `${reviewKindsText(pending)}. El modelo las encontró al incorporar documentos; hasta que alguien las mire, el wiki queda como está.`,
      todo: "Crealas, investigalas o descartalas en Revisión.",
      resolvedWhen: "Cada una desaparece cuando se resuelve o se descarta.",
      actions: [{ kind: "go", label: "Revisar", target: { view: "review" } }],
      cases: pending.slice(0, REVIEW_CASES).map((item) => ({
        id: item.id,
        title: item.title,
        detail: REVIEW_KINDS[item.type]?.[0],
      })),
      moreCases: Math.max(0, pending.length - REVIEW_CASES),
      scope,
    },
  ]
}

export function lintNotices(items: readonly LintItem[], scope: string | undefined): AttentionNotice[] {
  const broken = items.filter((item) => item.type === "broken-link")
  if (broken.length === 0) return []
  return [
    {
      source: LINT_SOURCE,
      id: `${LINT_SOURCE}:broken-links`,
      kind: "decision",
      topic: "broken-links",
      priority: "low",
      count: broken.length,
      title: plural(
        broken.length,
        "enlace del wiki apunta a una página que no existe",
        "enlaces del wiki apuntan a páginas que no existen",
      ),
      explanation:
        "Suelen ser páginas que todavía no se escribieron, o que cambiaron de nombre. La revisión de enlaces se hace sola cada vez que la cola del wiki termina.",
      todo: "Abrí cada página y arreglá el enlace, o dejalo hasta que la página exista.",
      resolvedWhen: "Desaparece cuando la próxima revisión de enlaces ya no lo encuentra.",
      actions: [{ kind: "go", label: "Ver enlaces", target: { view: "lint" } }],
      cases: broken.slice(0, REVIEW_CASES).map((item) => ({
        id: item.id,
        title: item.page,
        detail: item.brokenTarget ? `apunta a «${item.brokenTarget}»` : item.detail,
      })),
      moreCases: Math.max(0, broken.length - REVIEW_CASES),
      scope,
    },
  ]
}

/**
 * Whose the open project's notices are, when it is not the archive:
 * "Proyecto «Pericias»". `undefined` for the archive itself.
 */
export function projectScope(project: { name: string; path: string } | null, archivePath: string | null): string | undefined {
  if (!project) return undefined
  if (archivePath && normalizePath(archivePath) === normalizePath(project.path)) return undefined
  return `Proyecto «${project.name}»`
}

export function useOpenProjectScope(): string | undefined {
  const project = useWikiStore((s) => s.project)
  const general = useDocbaseQuery("attention:general-project", getGeneralProject)
  return projectScope(project, general.data?.path ?? null)
}

export function useReviewNotices(): AttentionNotice[] {
  const items = useReviewStore((s) => s.items)
  return reviewNotices(items, useOpenProjectScope())
}

export function useLintNotices(): AttentionNotice[] {
  const items = useLintStore((s) => s.items)
  return lintNotices(items, useOpenProjectScope())
}

/** How often the archive's wiki files are read again while another project is open. */
const ARCHIVE_POLL_MS = 30_000

/**
 * The archive's own wiki while another project is open: its proposals
 * and broken links are still the archive's, but they are resolved with
 * the archive open, so the lines carry no button.
 */
export function archiveWikiNotices(
  review: readonly ReviewItem[],
  lint: readonly LintItem[],
  words: Vocabulary = vocabulary(),
): AttentionNotice[] {
  const scope = words.archive.wiki
  const open = `Abrí ${words.archive.mine.toLowerCase()} (Cambiar de proyecto) para resolverlas.`
  return [...reviewNotices(review, scope), ...lintNotices(lint, scope)].map((notice) => ({
    ...notice,
    source: ARCHIVE_WIKI_SOURCE,
    id: notice.id.replace(/^[^:]+/, ARCHIVE_WIKI_SOURCE),
    todo: open,
    actions: [],
    cases: undefined,
    moreCases: undefined,
  }))
}

export const ARCHIVE_WIKI_SOURCE = "archive-wiki"

export function useArchiveWikiNotices(): AttentionNotice[] {
  const project = useWikiStore((s) => s.project)
  const general = useDocbaseQuery("attention:general-project", getGeneralProject)
  const archivePath = general.data?.path ?? null
  const elsewhere = Boolean(project && archivePath && projectScope(project, archivePath))
  const [notices, setNotices] = useState<AttentionNotice[]>([])
  useEffect(() => {
    if (!elsewhere || !archivePath) {
      setNotices([])
      return
    }
    let active = true
    const read = () =>
      Promise.all([loadReviewItems(archivePath), loadLintItems(archivePath)])
        .then(([review, lint]) => {
          if (active) setNotices(archiveWikiNotices(review, lint))
        })
        .catch(() => {
          if (active) setNotices([])
        })
    void read()
    const timer = setInterval(read, ARCHIVE_POLL_MS)
    return () => {
      active = false
      clearInterval(timer)
    }
  }, [elsewhere, archivePath])
  return notices
}
