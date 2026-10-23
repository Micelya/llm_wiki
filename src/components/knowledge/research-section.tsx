import { useState } from "react"
import { ChevronDown, ChevronRight, Globe2, Loader2, Search, Trash2 } from "lucide-react"
import { openUrl } from "@tauri-apps/plugin-opener"
import { Button } from "@/components/ui/button"
import { deleteResearch, listLooseResearch, listResearchOf, type Research } from "@/commands/docbase"
import { ErrorNote } from "@/components/docbase/page"
import { formatAgo } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { WikiReader } from "@/components/editor/wiki-reader"
import { queueResearch } from "@/lib/deep-research"
import { hasConfiguredDeepResearchSources } from "@/lib/web-search"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useResearchStore, type ResearchTask } from "@/stores/research-store"
import { useWikiStore } from "@/stores/wiki-store"
import { Section } from "./shared"

const RUNNING: ResearchTask["status"][] = ["queued", "searching", "synthesizing", "saving"]

const STATUS_WORDS: Record<ResearchTask["status"], string> = {
  queued: "en espera",
  searching: "buscando en internet…",
  synthesizing: "escribiendo la síntesis…",
  saving: "guardando…",
  done: "guardada",
  error: "no se pudo",
}

/**
 * «Investigación en internet» of a person, organization or matter (D-414):
 * a person asks for one with a topic the screen proposes and she can
 * change; it is kept in the base with its sources, apart from what the
 * documents say.
 */
export function ResearchSection({ objectId, name, id }: { objectId: string; name: string; id?: string }) {
  const researches = useDocbaseQuery(`research:${objectId}`, () => listResearchOf(objectId))
  const tasks = useResearchStore((s) => s.tasks)
  const llmConfig = useWikiStore((s) => s.llmConfig)
  const searchApiConfig = useWikiStore((s) => s.searchApiConfig)
  const appDialog = useAppDialog()
  const [topic, setTopic] = useState<string | null>(null)
  const running = tasks.filter((task) => task.objectId === objectId && RUNNING.includes(task.status))
  const failed = tasks.filter((task) => task.objectId === objectId && task.status === "error")
  const list = researches.data ?? []

  async function start() {
    const chosen = (topic ?? "").trim()
    if (!chosen) return
    if (!hasConfiguredDeepResearchSources(searchApiConfig)) {
      await appDialog.alert({
        title: "Falta configurar la búsqueda",
        message: "Para investigar en internet hace falta elegir un buscador en Ajustes → Servicios de IA, con sus datos cargados en «Búsqueda en internet».",
      })
      return
    }
    queueResearch(chosen, llmConfig, searchApiConfig, { target: { objectId, name }, openPanel: false })
    setTopic(null)
  }

  return (
    <Section
      id={id}
      title="Investigación en internet"
      action={
        topic === null && (
          <Button size="sm" variant="outline" onClick={() => setTopic(name)}>
            <Search /> Investigar
          </Button>
        )
      }
    >
      <p className="mb-2.5 text-[0.84375rem] text-muted-foreground">
        Lo que se encontró en internet sobre {name}. No sale de los documentos y no cambia lo que dicen.
      </p>
      {topic !== null && (
        <div className="mb-3 flex flex-wrap items-center gap-2 rounded-[10px] border bg-card p-2">
          <input
            autoFocus
            value={topic}
            onChange={(event) => setTopic(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void start()
              if (event.key === "Escape") setTopic(null)
            }}
            className="h-8 min-w-[16rem] flex-1 rounded-lg border border-border-strong bg-card px-3 text-sm outline-none focus:border-link focus:ring-3 focus:ring-ring/30"
            placeholder="Qué investigar"
          />
          <Button size="sm" disabled={!topic.trim()} onClick={() => void start()}>
            Investigar
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setTopic(null)}>
            Cancelar
          </Button>
        </div>
      )}
      {researches.error && <ErrorNote>{researches.error}</ErrorNote>}
      {running.map((task) => (
        <p key={task.id} className="mb-1 flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="size-3.5 animate-spin" /> {task.topic}: {STATUS_WORDS[task.status]}
        </p>
      ))}
      {failed.map((task) => (
        <p key={task.id} className="mb-1 text-sm text-err">
          {task.topic}: {task.error ?? STATUS_WORDS.error}
        </p>
      ))}
      {list.length === 0 && running.length === 0 && topic === null && (
        <p className="text-sm text-muted-foreground">Todavía no se investigó nada.</p>
      )}
      <div className="flex flex-col gap-2">
        {list.map((research) => (
          <ResearchItem key={research.id} research={research} onDeleted={researches.reload} />
        ))}
      </div>
    </Section>
  )
}

/** The loose researches (of no object), for the research panel. */
export function LooseResearchList({ openId }: { openId?: number | null }) {
  const researches = useDocbaseQuery("research:loose", listLooseResearch)
  const list = researches.data ?? []
  if (list.length === 0) return null
  return (
    <div className="flex flex-col gap-1 p-2">
      <div className="px-1 text-[11px] font-medium text-muted-foreground">Investigaciones guardadas</div>
      {list.map((research) => (
        <ResearchItem
          key={`${research.id}-${openId === research.id}`}
          research={research}
          compact
          defaultOpen={openId === research.id}
          onDeleted={researches.reload}
        />
      ))}
    </div>
  )
}

/** One research kept in the base: its topic and date; open, its text and sources. */
export function ResearchItem({
  research,
  onDeleted,
  compact = false,
  defaultOpen = false,
}: {
  research: Research
  onDeleted: () => void
  compact?: boolean
  defaultOpen?: boolean
}) {
  const [open, setOpen] = useState(defaultOpen)
  const appDialog = useAppDialog()

  async function remove() {
    const confirmed = await appDialog.confirm({
      title: "Borrar la investigación",
      message: `¿Borrar la investigación «${research.topic}»? No se puede deshacer.`,
      confirmLabel: "Borrar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (!confirmed) return
    await deleteResearch(research.id)
    onDeleted()
  }

  return (
    <div className={`rounded-[10px] border bg-card ${compact ? "text-xs" : "text-sm"}`}>
      <div className="flex items-center gap-2 px-3 py-2">
        <button type="button" className="flex min-w-0 flex-1 items-center gap-2 text-left" onClick={() => setOpen(!open)}>
          {open ? <ChevronDown className="size-3.5 shrink-0" /> : <ChevronRight className="size-3.5 shrink-0" />}
          <span className="truncate font-ui font-bold text-strong">{research.topic}</span>
          <span className="shrink-0 text-xs text-muted-foreground">
            {formatAgo(research.createdAt)} · {research.sources.length} {research.sources.length === 1 ? "fuente" : "fuentes"}
          </span>
        </button>
        <button type="button" title="Borrar" className="shrink-0 rounded p-1 text-muted-foreground hover:bg-err-bg hover:text-err" onClick={() => void remove()}>
          <Trash2 className="size-3.5" />
        </button>
      </div>
      {open && (
        <div className="border-t px-3 py-2">
          <WikiReader body={research.body} compact />
          {research.sources.length > 0 && (
            <ol className="mt-3 flex flex-col gap-1 border-t pt-2 text-xs">
              {research.sources.map((source) => (
                <li key={source.index} className="flex items-start gap-1.5">
                  <span className="shrink-0 font-mono text-muted-foreground">[{source.index}]</span>
                  <Globe2 className="mt-0.5 size-3 shrink-0 text-muted-foreground" />
                  <button
                    type="button"
                    className="min-w-0 truncate text-left text-link hover:underline"
                    title={source.url}
                    onClick={() => void openUrl(source.url).catch(() => {})}
                  >
                    {source.title || source.url}
                  </button>
                  {source.source && <span className="shrink-0 text-muted-foreground">· {source.source}</span>}
                </li>
              ))}
            </ol>
          )}
        </div>
      )}
    </div>
  )
}
