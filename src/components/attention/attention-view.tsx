import { useState } from "react"
import { ChevronDown, ChevronRight, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Callout } from "@/components/ui/callout"
import { Tab, TabList } from "@/components/ui/tabs"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useAttentionViewStore, type AttentionTab } from "@/stores/attention-view-store"
import { noticesOf, summaryText } from "@/lib/attention/order"
import type { AttentionAction, AttentionCase, AttentionNotice } from "@/lib/attention/types"
import { ongoingCount } from "@/lib/progress/stages"
import { vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { formatCount, plural } from "@/components/docbase/format"
import { OngoingTab } from "@/components/progress/ongoing-tab"
import { useProgress } from "@/components/progress/use-progress"
import { goToTarget } from "./navigate"
import { useAttention } from "./use-attention"

export const ATTENTION_TABS: { kind: AttentionTab; title: string; empty: string }[] = [
  { kind: "problem", title: "Problemas", empty: "Nada está fallando." },
  { kind: "decision", title: "Decisiones", empty: "Nada espera una decisión." },
  { kind: "ongoing", title: "En curso", empty: "La app no está haciendo nada ahora." },
]

/**
 * The tab shown: the one the person chose; until she chooses, the first
 * with something in it, problems first.
 */
export function shownTab(chosen: AttentionTab | null, counts: Record<AttentionTab, number>): AttentionTab {
  if (chosen) return chosen
  return ATTENTION_TABS.find((tab) => counts[tab.kind] > 0)?.kind ?? "problem"
}

/**
 * The single place for what needs a person, whatever part of the app
 * found it, in three tabs (D-424): «Problemas», «Decisiones» and
 * «En curso», each with how many things it has. Each line says what
 * happens, why it matters and what to do, and brings its action; opened,
 * it shows its cases one by one. «En curso» shows every stage of the
 * processing, done and to do; its figure counts over the origin or folder
 * the tab is narrowed to, as the tab does.
 */
export function AttentionView() {
  const { notices, counts } = useAttention()
  const scope = useAttentionViewStore((s) => s.scope)
  const { progress, error } = useProgress(scope)
  const chosen = useAttentionViewStore((s) => s.tab)
  const setTab = useAttentionViewStore((s) => s.setTab)
  const words = vocabulary()
  const summary = summaryText(counts)
  const lines: Record<AttentionTab, AttentionNotice[]> = {
    problem: noticesOf(notices, "problem"),
    decision: noticesOf(notices, "decision"),
    ongoing: noticesOf(notices, "ongoing"),
  }
  const tabCounts: Record<AttentionTab, number> = {
    problem: lines.problem.length,
    decision: lines.decision.length,
    ongoing: ongoingCount(progress) + lines.ongoing.length,
  }
  const tab = shownTab(chosen, tabCounts)
  const empty = ATTENTION_TABS.find((t) => t.kind === tab)?.empty ?? ""
  return (
    <DocbasePage title={words.attention.name} subtitle={summary ?? words.attention.description}>
      <div className="mx-auto flex max-w-4xl flex-col gap-4">
        <TabList label={words.attention.name}>
          {ATTENTION_TABS.map((item) => (
            <Tab
              key={item.kind}
              active={tab === item.kind}
              count={tabCounts[item.kind] > 0 ? formatCount(tabCounts[item.kind]) : undefined}
              countTone={item.kind === "problem" ? "err" : item.kind === "decision" ? "warn" : "neutral"}
              onClick={() => setTab(item.kind)}
            >
              {item.title}
            </Tab>
          ))}
        </TabList>
        <div role="tabpanel" data-tab={tab}>
          {tab === "ongoing" ? (
            <OngoingTab
              progress={progress}
              error={error}
              extra={lines.ongoing.length > 0 ? <NoticeList lines={lines.ongoing} /> : null}
            />
          ) : lines[tab].length === 0 ? (
            <Callout tone="quiet" title={null}>{empty}</Callout>
          ) : (
            <NoticeList lines={lines[tab]} />
          )}
        </div>
      </div>
    </DocbasePage>
  )
}

function NoticeList({ lines }: { lines: AttentionNotice[] }) {
  return (
    <ul className="flex flex-col divide-y rounded-lg border bg-card">
      {lines.map((notice) => (
        <NoticeLine key={notice.id} notice={notice} />
      ))}
    </ul>
  )
}

export function NoticeMark({ notice }: { notice: Pick<AttentionNotice, "kind" | "priority"> }) {
  if (notice.kind === "ongoing") {
    return <span className="mt-1.5 size-2.5 shrink-0 rounded-full border-2 border-info" aria-hidden />
  }
  // A problem is round, a decision a diamond; each with a halo of its tone.
  const color =
    notice.priority === "low"
      ? "bg-muted-foreground/50"
      : notice.kind === "problem"
        ? notice.priority === "high"
          ? "bg-err shadow-[0_0_0_3px_var(--err-bg)]"
          : "bg-warn shadow-[0_0_0_3px_var(--warn-bg)]"
        : "bg-info shadow-[0_0_0_3px_var(--info-bg)]"
  const shape = notice.kind === "decision" ? "rotate-45 scale-90 rounded-[2px]" : "rounded-full"
  return <span className={`mt-1.5 size-2.5 shrink-0 ${shape} ${color}`} aria-hidden />
}

function NoticeLine({ notice }: { notice: AttentionNotice }) {
  const [open, setOpen] = useState(false)
  const [cases, setCases] = useState<AttentionCase[] | null>(notice.cases ?? null)
  const [loading, setLoading] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const expandable = Boolean(notice.cases?.length || notice.loadCases)
  const shownCases = notice.cases ?? cases

  async function toggle() {
    const next = !open
    setOpen(next)
    if (next && !notice.cases && notice.loadCases) {
      setLoading(true)
      try {
        setCases(await notice.loadCases())
        setProblem(null)
      } catch (cause) {
        setProblem(String(cause))
      } finally {
        setLoading(false)
      }
    }
  }

  return (
    <li className="px-4 py-3" data-notice={notice.id}>
      <div className="flex items-start gap-3">
        <NoticeMark notice={notice} />
        <div className="min-w-0 flex-1">
          <button
            className="flex w-full items-start gap-1 text-left disabled:cursor-default"
            onClick={() => void toggle()}
            disabled={!expandable}
            aria-expanded={expandable ? open : undefined}
          >
            <span className="min-w-0 flex-1">
              <span className="font-ui text-[0.90625rem] font-bold text-strong">{notice.title}</span>
              {notice.scope && (
                <span className="ml-2 align-middle">
                  <StatusBadge tone="neutral">{notice.scope}</StatusBadge>
                </span>
              )}
              {notice.priority === "low" && notice.kind !== "ongoing" && (
                <span className="ml-2 align-middle">
                  <StatusBadge tone="neutral">puede esperar</StatusBadge>
                </span>
              )}
            </span>
            {expandable &&
              (open ? (
                <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
              ) : (
                <ChevronRight className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
              ))}
          </button>
          <p className="mt-0.5 text-[0.8125rem] text-muted-foreground">{notice.explanation}</p>
          <p className="mt-0.5 text-[0.8125rem]">{notice.todo}</p>
          {notice.progress && <Progress {...notice.progress} />}
        </div>
        {notice.actions.length > 0 && (
          <div className="flex shrink-0 flex-wrap justify-end gap-1.5">
            {notice.actions.map((action, index) => (
              <ActionButton key={action.label} action={action} primary={index === 0} onProblem={setProblem} />
            ))}
          </div>
        )}
      </div>
      {problem && (
        <div className="mt-2 ml-5.5">
          <ErrorNote>{problem}</ErrorNote>
        </div>
      )}
      {open && (
        <div className="mt-2 ml-5.5 rounded-[10px] border bg-panel-sub">
          {loading ? (
            <p className="flex items-center gap-2 px-3 py-2 text-xs text-muted-foreground">
              <Loader2 className="size-3 animate-spin" /> Buscando…
            </p>
          ) : shownCases && shownCases.length > 0 ? (
            <ul className="flex flex-col divide-y">
              {shownCases.map((item) => (
                <CaseRow key={item.id} item={item} onProblem={setProblem} />
              ))}
            </ul>
          ) : (
            <EmptyNote>Ya no queda ninguno.</EmptyNote>
          )}
          {(notice.moreCases ?? 0) > 0 && (
            <p className="border-t px-3 py-2 text-xs text-muted-foreground">
              Y {plural(notice.moreCases ?? 0, "más", "más")}: el botón de la línea lleva a la lista completa.
            </p>
          )}
          <p className="border-t px-3 py-1.5 text-[11px] text-muted-foreground">{notice.resolvedWhen}</p>
        </div>
      )}
    </li>
  )
}

function CaseRow({ item, onProblem }: { item: AttentionCase; onProblem: (problem: string | null) => void }) {
  return (
    <li className="flex items-start gap-3 px-3 py-2 text-sm">
      <div className="min-w-0 flex-1">
        <div className="break-words">{item.title}</div>
        {item.detail && <div className="text-xs break-words text-muted-foreground">{item.detail}</div>}
      </div>
      {item.actions && item.actions.length > 0 && (
        <div className="flex shrink-0 flex-wrap justify-end gap-1">
          {item.actions.map((action) => (
            <ActionButton key={action.label} action={action} small onProblem={onProblem} />
          ))}
        </div>
      )}
    </li>
  )
}

export function ActionButton({
  action,
  primary = false,
  small = false,
  look,
  onProblem,
}: {
  action: AttentionAction
  primary?: boolean
  small?: boolean
  /** How it is drawn, when not as the main or a quiet button: Inicio draws them outlined in the brand. */
  look?: "secondary"
  onProblem: (problem: string | null) => void
}) {
  const dialog = useAppDialog()
  const [busy, setBusy] = useState(false)

  async function run() {
    onProblem(null)
    if (action.kind === "go") {
      goToTarget(action.target)
      return
    }
    if (action.confirm) {
      const confirmed = await dialog.confirm({
        title: action.label,
        message: action.confirm,
        confirmLabel: action.label,
        cancelLabel: "Cancelar",
        variant: action.danger ? "destructive" : "default",
      })
      if (!confirmed) return
    }
    setBusy(true)
    try {
      await action.run()
    } catch (cause) {
      onProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  const danger = action.kind === "run" && action.danger
  return (
    <Button
      size={small ? "xs" : "sm"}
      variant={danger ? "ghost" : look ?? (primary ? "default" : "outline")}
      className={danger ? "text-err hover:text-err" : undefined}
      disabled={busy}
      onClick={() => void run()}
    >
      {busy && <Loader2 className="animate-spin" />}
      {action.label}
    </Button>
  )
}

function Progress({ done, total }: { done: number; total: number }) {
  const share = total > 0 ? Math.min(1, done / total) : 0
  return (
    <div className="mt-2 flex items-center gap-2">
      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-border">
        <div className="h-full rounded-full bg-grad-principal transition-all" style={{ width: `${share * 100}%` }} />
      </div>
      <span className="shrink-0 text-xs text-muted-foreground">
        {formatCount(done)} de {formatCount(total)}
      </span>
    </div>
  )
}
