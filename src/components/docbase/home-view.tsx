import { useState, type ReactNode } from "react"
import {
  ArrowRight, ChevronRight, CircleAlert, CirclePause, Diamond, FilePen, FilePlus, FileX, FolderPlus,
  LoaderCircle, TriangleAlert, type LucideIcon,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { MetricList, MetricRow, Meter, MiniHeading, Panel, StatTile } from "@/components/ui/panel"
import { getHome, getKnowledgeOverview, listMatters, type Activity } from "@/commands/docbase"
import { ArticleText } from "@/components/knowledge/article-text"
import { articleBody, articleTitle } from "@/components/knowledge/knowledge"
import { useWikiStore } from "@/stores/wiki-store"
import { useProgressStore } from "@/stores/progress-store"
import { noticesOf, summaryText, type AttentionCounts } from "@/lib/attention/order"
import type { AttentionNotice } from "@/lib/attention/types"
import { openWikiPath } from "@/lib/open-wiki-path"
import { ActionButton, NoticeMark } from "@/components/attention/attention-view"
import { goToTarget } from "@/components/attention/navigate"
import { useAttention } from "@/components/attention/use-attention"
import { processedPercent } from "./attention"
import { DocbasePage, EmptyNote, ErrorNote } from "./page"
import { formatAgo, formatCount, plural } from "./format"
import { parseOverview, type Overview, type OverviewItem } from "./overview"
import { useDocbaseQuery } from "./use-docbase-query"
import { vocabulary, type Vocabulary } from "@/vocabulary"

/** How many lines that need a person Inicio shows before "Ver todo". */
const HOME_GLANCE = 3

/** How many names each list of the overview shows in Inicio. */
const OVERVIEW_ROWS = 5

/**
 * Inicio, as the approved mockup (P-057, D-493): what waits in Atención,
 * each line with its explanation and its button; the overview of the
 * archive with large figures and its two lists; what happened in the
 * folders, an icon for each kind; and at the right «Mi semana», which
 * shows only what exists today (the active matters and what waits in
 * Atención; deadlines, meetings and approvals join it when they exist,
 * P-059, P-062), and the health of the archive.
 */
export function HomeView() {
  const home = useDocbaseQuery("home", getHome)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const { notices, counts } = useAttention()
  const progress = useProgressStore((s) => s.progress)
  // What is really being processed, every stage counted (D-424).
  const inProcess = progress?.inProcess ?? 0
  const paused = progress?.paused ?? 0
  const words = vocabulary()

  const data = home.data
  const empty = data !== null && data.health.origins === 0
  const processed = data ? processedPercent(data.health) : null
  const today = new Date().toLocaleDateString("es-AR", { weekday: "long", day: "numeric", month: "long", year: "numeric" })

  return (
    <DocbasePage title="Inicio" subtitle={today}>
      <div className="grid max-w-[1240px] gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="flex min-w-0 flex-col gap-6">
          {home.error && <ErrorNote>{home.error}</ErrorNote>}

          {empty ? (
            <Panel>
              <div className="flex flex-col items-center gap-4 py-8 text-center">
                <p className="max-w-md text-sm text-muted-foreground">
                  La base documental todavía está vacía. Incorporá la primera carpeta y la app empieza a
                  inventariar tus documentos, sin moverlos ni copiarlos.
                </p>
                <Button size="lg" onClick={() => setActiveView("origins")}>
                  <FolderPlus /> Agregar una carpeta
                </Button>
              </div>
            </Panel>
          ) : (
            <>
              <AttentionPanel notices={notices} counts={counts} inProcess={inProcess} words={words} />
              <OverviewPanel words={words} />
              <Panel title="Actividad reciente" meta="en las carpetas incorporadas" flush>
                {data && data.activity.length === 0 ? (
                  <EmptyNote>Todavía no pasó nada en las carpetas incorporadas.</EmptyNote>
                ) : (
                  <ul className="flex flex-col divide-y">
                    {(data?.activity ?? []).map((entry, index) => (
                      <ActivityRow key={index} entry={entry} />
                    ))}
                  </ul>
                )}
              </Panel>
            </>
          )}
        </div>

        <aside className="flex min-w-0 flex-col gap-6">
          {!empty && <WeekPanel counts={counts} words={words} />}

          {data && data.health.origins > 0 && (
            <Panel title={words.archive.health}>
              <MetricList>
                <MetricRow label="Documentos distintos">{formatCount(data.health.documents)}</MetricRow>
                <MetricRow label="Archivos en el inventario">{formatCount(data.health.locations)}</MetricRow>
                <MetricRow
                  label="Procesados"
                  below={processed !== null && <Meter share={processed / 100} label="Procesados" className="mt-2" />}
                >
                  {processed === null ? "—" : `${processed.toLocaleString("es-AR")} %`}
                </MetricRow>
                <MetricRow label="En curso">
                  <button
                    className="flex items-center gap-1.5 rounded-md hover:opacity-80"
                    title={`Ver en ${words.attention.name} → En curso`}
                    onClick={() => goToTarget({ view: "attention", tab: "ongoing" })}
                  >
                    {inProcess > 0 ? (
                      <Badge tone="info" icon={LoaderCircle} spin>
                        {formatCount(inProcess)}
                      </Badge>
                    ) : (
                      "0"
                    )}
                    {paused > 0 && (
                      <Badge tone="warn" icon={CirclePause}>
                        {formatCount(paused)} en pausa
                      </Badge>
                    )}
                  </button>
                </MetricRow>
                <MetricRow label="Con copias en otro lugar">{formatCount(data.health.copies)}</MetricRow>
                <MetricRow label="Palabras en la base">{formatCount(data.health.words)}</MetricRow>
                <MetricRow label="Carpetas">
                  {data.health.unreachableOrigins > 0 ? (
                    <Badge tone="warn" icon={TriangleAlert}>
                      {data.health.unreachableOrigins} de {data.health.origins} sin conexión
                    </Badge>
                  ) : (
                    plural(data.health.origins, "conectada", "conectadas")
                  )}
                </MetricRow>
                <MetricRow label="Último barrido">
                  {data.health.lastScanAt === null ? "en curso" : formatAgo(data.health.lastScanAt)}
                </MetricRow>
              </MetricList>
            </Panel>
          )}
        </aside>
      </div>
    </DocbasePage>
  )
}

/** «2 problemas», «3 decisiones»: each with its icon, never only its colour. */
function CountBadges({ counts }: { counts: AttentionCounts }) {
  return (
    <>
      {counts.problems > 0 && (
        <Badge tone="err" icon={CircleAlert}>
          {plural(counts.problems, "problema", "problemas")}
        </Badge>
      )}
      {counts.decisions > 0 && (
        <Badge tone="info" icon={Diamond}>
          {plural(counts.decisions, "decisión", "decisiones")}
        </Badge>
      )}
    </>
  )
}

/**
 * What waits for a person, at a glance: the first lines, each with what
 * kind of thing it is in a word, what happens, why it matters and its
 * button; the rest is one click away, in the view.
 */
function AttentionPanel({
  notices,
  counts,
  inProcess,
  words,
}: {
  notices: AttentionNotice[]
  counts: AttentionCounts
  inProcess: number
  words: Vocabulary
}) {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const [problem, setProblem] = useState<string | null>(null)
  const summary = summaryText(counts)
  const glance = [...noticesOf(notices, "problem"), ...noticesOf(notices, "decision")]
    .filter((notice) => notice.priority !== "low")
    .slice(0, HOME_GLANCE)
  return (
    <Panel
      title={words.attention.name}
      meta={
        summary !== null && (
          <span className="inline-flex flex-wrap items-center gap-1.5">
            <CountBadges counts={counts} />
          </span>
        )
      }
      action={
        <Button size="sm" variant="outline" onClick={() => setActiveView("attention")}>
          Ver todo <ChevronRight />
        </Button>
      }
    >
      {summary === null ? (
        <EmptyNote>
          Nada requiere tu atención.
          {inProcess > 0 && " La app está trabajando: lo ves en la pestaña «En curso»."}
        </EmptyNote>
      ) : (
        <div className="flex flex-col gap-3">
          <ul className="flex flex-col divide-y">
            {glance.map((notice) => (
              <li
                key={notice.id}
                data-notice={notice.id}
                className="grid grid-cols-[14px_minmax(0,1fr)_auto] items-start gap-x-3 py-3 first:pt-0"
              >
                <NoticeMark notice={notice} />
                <div className="min-w-0">
                  <p className="font-ui text-[0.90625rem] font-bold text-strong">
                    <NoticeKind notice={notice} />
                    {notice.title}
                  </p>
                  <p className="mt-0.5 text-[0.84375rem] text-muted-foreground">{notice.explanation}</p>
                </div>
                {notice.actions[0] ? (
                  <ActionButton action={notice.actions[0]} look="secondary" onProblem={setProblem} />
                ) : (
                  <span />
                )}
              </li>
            ))}
          </ul>
          {problem && <ErrorNote>{problem}</ErrorNote>}
          {inProcess > 0 && (
            <p className="flex flex-wrap items-center gap-x-2 border-t pt-3 text-[0.84375rem] text-muted-foreground">
              <LoaderCircle className="size-4 shrink-0 animate-spin text-info" aria-hidden />
              <span>
                Además, {inProcess === 1 ? "hay un documento en proceso" : `hay ${formatCount(inProcess)} documentos en proceso`}.
              </span>
              <button
                className="font-semibold text-link hover:underline"
                onClick={() => goToTarget({ view: "attention", tab: "ongoing" })}
              >
                Ver «En curso»
              </button>
            </p>
          )}
        </div>
      )}
    </Panel>
  )
}

/** «Problema» or «Decisión», written before the title in the tone of its mark. */
function NoticeKind({ notice }: { notice: AttentionNotice }) {
  const problem = notice.kind === "problem"
  const tone = problem ? (notice.priority === "high" ? "text-err" : "text-warn") : "text-info"
  return (
    <span className={`mr-1.5 font-ui text-[11px] font-bold tracking-[0.06em] uppercase ${tone}`}>
      {problem ? "Problema" : "Decisión"}
    </span>
  )
}

/**
 * «Mi semana» (D-493): the place of the activities and the agenda of the
 * week. Today it shows what exists, the active matters and what waits in
 * Atención; deadlines, meetings and approvals join it when they exist
 * (P-059, P-062), never as empty boxes.
 */
function WeekPanel({ counts, words }: { counts: AttentionCounts; words: Vocabulary }) {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const matters = useDocbaseQuery("matters", listMatters)
  const active = matters.data
    ? matters.data.filter((matter) => matter.status === "open" && matter.confirmation !== "discarded").length
    : null
  const waiting = counts.problems + counts.decisions
  return (
    <Panel title="Mi semana">
      <MetricList>
        {active !== null && (
          <MetricRow label={`${words.knowledge.matters} activos`}>
            <button className="rounded-sm text-link hover:underline" onClick={() => setActiveView("matters")}>
              {formatCount(active)}
            </button>
          </MetricRow>
        )}
        <MetricRow label={`Esperan en ${words.attention.name}`}>
          {waiting > 0 ? (
            <button
              className="flex flex-wrap items-center justify-end gap-1.5 hover:opacity-80"
              title={`Ir a ${words.attention.name}`}
              onClick={() => setActiveView("attention")}
            >
              <CountBadges counts={counts} />
            </button>
          ) : (
            <span className="font-reading font-normal text-muted-foreground">nada</span>
          )}
        </MetricRow>
      </MetricList>
    </Panel>
  )
}

/**
 * The overview of the archive, as the projection last wrote it in the
 * base (D-407): the large figures of what was read and who has a page,
 * and the two lists, the most named and the matters with more documents;
 * each name opens its screen. Shown as an article when it is of another
 * shape.
 */
function OverviewPanel({ words }: { words: Vocabulary }) {
  const overview = useDocbaseQuery("overview", getKnowledgeOverview)
  if (!overview.data) return null
  const title = articleTitle(overview.data.body) ?? "Resumen del archivo"
  const parsed = parseOverview(overview.data.body)
  if (!parsed) {
    return (
      <Panel title={title}>
        <ArticleText body={articleBody(overview.data.body)} writtenAt={overview.data.writtenAt} />
      </Panel>
    )
  }
  return (
    <Panel title={title} meta={`Lo escribió la app ${formatAgo(overview.data.writtenAt)}, desde los documentos`}>
      <OverviewBody overview={parsed} words={words} />
    </Panel>
  )
}

function OverviewBody({ overview, words }: { overview: Overview; words: Vocabulary }) {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const [read, failed] = overview.figures
  const matterName = words.knowledge.matters.toLowerCase()
  // Who has a page, by kind; the matters named as such count with the matters.
  const types = Object.entries(words.knowledge.entityTypes)
  const termOf = (kind: string) => types.find(([, term]) => term.one === kind)
  const kinds = overview.withPage.filter(({ kind }) => {
    const id = termOf(kind)?.[0]
    return !id || !words.knowledge.referenceTypes.includes(id)
  })
  const moreMatters = overview.matters.length - OVERVIEW_ROWS
  return (
    <div className="flex flex-col gap-5">
      <div className="grid grid-cols-[repeat(auto-fit,minmax(9.5rem,1fr))] gap-3">
        <StatTile
          value={formatCount(read.value)}
          label={read.label.toLowerCase()}
          onClick={() => setActiveView("documents")}
          title="Ver los documentos"
        />
        {kinds.map(({ kind, count }) => {
          const name = count === 1 ? kind : (termOf(kind)?.[1].many ?? kind)
          return (
            <StatTile
              key={kind}
              value={formatCount(count)}
              label={`${name.toLowerCase()} con página`}
              onClick={() => setActiveView("entities")}
              title={`Ver ${words.knowledge.entities.toLowerCase()}`}
            />
          )
        })}
        <StatTile
          value={formatCount(overview.matters.length)}
          label={overview.matters.length === 1 ? words.knowledge.matter.toLowerCase() : matterName}
          onClick={() => setActiveView("matters")}
          title={`Ver los ${matterName}`}
        />
      </div>

      {(overview.mostNamed.length > 0 || overview.matters.length > 0) && (
        <div className="grid gap-7 md:grid-cols-2">
          <div className="min-w-0">
            <MiniHeading>Los más nombrados</MiniHeading>
            <RankList items={overview.mostNamed.slice(0, OVERVIEW_ROWS)} />
          </div>
          <div className="min-w-0">
            <MiniHeading>{words.knowledge.matters} con más documentos</MiniHeading>
            <RankList items={overview.matters.slice(0, OVERVIEW_ROWS)} />
          </div>
        </div>
      )}

      {(moreMatters > 0 || (failed !== undefined && failed.value > 0)) && (
        <p className="text-[0.8125rem] text-muted-foreground">
          {moreMatters > 0 && `Y ${formatCount(moreMatters)} ${moreMatters === 1 ? words.knowledge.matter.toLowerCase() : matterName} más. `}
          {failed !== undefined && failed.value > 0 && (
            <>
              {failed.value === 1
                ? "1 documento cuya lectura falló está en "
                : `${formatCount(failed.value)} documentos cuya lectura falló están en `}
              <button className="text-link hover:underline" onClick={() => setActiveView("attention")}>
                {words.attention.name}
              </button>
              .
            </>
          )}
        </p>
      )}
    </div>
  )
}

/** The rows of a list of the overview: the name that opens its screen, its kind, and in how many documents. */
function RankList({ items }: { items: OverviewItem[] }) {
  const [gone, setGone] = useState<string | null>(null)
  if (items.length === 0) return <p className="text-sm text-muted-foreground">Ninguno todavía.</p>
  return (
    <>
      <ol className="flex flex-col divide-y">
        {items.map((item, index) => (
          <li key={item.page} className="flex items-baseline gap-2.5 py-[7px] text-sm">
            <span className="min-w-0 flex-1 truncate">
              <button
                className="text-link hover:underline"
                onClick={async () => setGone((await openWikiPath(item.page)) ? null : item.name)}
              >
                {item.name}
              </button>
              {item.kind && <span className="text-[0.78125rem] text-muted-foreground"> · {item.kind}</span>}
            </span>
            <span className="shrink-0 text-[0.8125rem] whitespace-nowrap text-muted-foreground tabular-nums">
              {index === 0 ? plural(item.documents, "documento", "documentos") : formatCount(item.documents)}
            </span>
          </li>
        ))}
      </ol>
      {gone && <p className="mt-2 text-xs text-muted-foreground">«{gone}» ya no está en la base.</p>}
    </>
  )
}

const ACTIVITY_ICONS: Record<Activity["kind"], { icon: LucideIcon; className: string }> = {
  added: { icon: FilePlus, className: "text-ok" },
  modified: { icon: FilePen, className: "text-muted-foreground" },
  moved: { icon: ArrowRight, className: "text-muted-foreground" },
  removed: { icon: FileX, className: "text-err" },
}

/** One thing that happened in the folders, with the icon of its kind. */
function ActivityRow({ entry }: { entry: Activity }) {
  const { icon: Icon, className } = ACTIVITY_ICONS[entry.kind] ?? ACTIVITY_ICONS.modified
  return (
    <li className="grid grid-cols-[22px_minmax(0,1fr)_auto] items-center gap-2.5 px-[18px] py-2.5 text-sm">
      <Icon className={`size-4 ${className}`} aria-hidden />
      <span className="min-w-0 truncate" title={entry.fromPath ? `${entry.fromPath} → ${entry.relPath}` : entry.relPath}>
        {describeActivity(entry)}
      </span>
      <span className="shrink-0 text-[0.8125rem] text-muted-foreground">
        {entry.originName} · {formatAgo(entry.at)}
      </span>
    </li>
  )
}

function fileName(relPath: string): string {
  return relPath.split("/").pop() ?? relPath
}

/** «Apareció **nota.pdf**», «Aparecieron 23 archivos»: the file in bold when there is one. */
function describeActivity(entry: Activity): ReactNode {
  const one = entry.count === 1
  const file = (path: string) => <b className="font-semibold text-strong">{fileName(path)}</b>
  switch (entry.kind) {
    case "added":
      return one ? <>Apareció {file(entry.relPath)}</> : `Aparecieron ${formatCount(entry.count)} archivos`
    case "modified":
      return one ? <>Cambió {file(entry.relPath)}</> : `Cambiaron ${formatCount(entry.count)} archivos`
    case "removed":
      return one ? <>Ya no está {file(entry.relPath)}</> : `Ya no están ${formatCount(entry.count)} archivos`
    case "moved":
      return entry.fromPath ? (
        <>
          Se movió {file(entry.fromPath)} a {entry.relPath}
        </>
      ) : (
        <>Se movió {file(entry.relPath)}</>
      )
    default:
      return fileName(entry.relPath)
  }
}
