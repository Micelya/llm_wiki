import { useState, type ReactNode } from "react"
import { ChevronRight, FolderPlus, Search } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { getHome, type Activity } from "@/commands/docbase"
import { DEFAULT_DOCUMENT_FILTER, useDocbaseStore } from "@/stores/docbase-store"
import { useReviewStore } from "@/stores/review-store"
import { useWikiStore } from "@/stores/wiki-store"
import { buildAttention, pendingCount, processedPercent, type AttentionItem } from "./attention"
import { DocbasePage, EmptyNote, ErrorNote, Panel, StatusBadge } from "./page"
import { formatAgo, formatCount, plural } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"
import { vocabulary } from "@/vocabulary"

export function HomeView() {
  const home = useDocbaseQuery("home", getHome)
  const reviewPending = useReviewStore((s) => s.items.filter((item) => !item.resolved).length)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const setFilter = useDocbaseStore((s) => s.setFilter)
  const [query, setQuery] = useState("")

  function goTo(item: AttentionItem) {
    if (item.target.view === "documents") {
      setFilter({ ...DEFAULT_DOCUMENT_FILTER, state: item.target.state })
    }
    setActiveView(item.target.view)
  }

  function search() {
    setFilter({ ...DEFAULT_DOCUMENT_FILTER, query: query.trim() })
    setActiveView("documents")
  }

  const data = home.data
  const attention = data ? buildAttention(data.health, data.origins, reviewPending) : []
  const today = new Date().toLocaleDateString("es-AR", { weekday: "long", day: "numeric", month: "long", year: "numeric" })

  return (
    <DocbasePage title="Inicio" subtitle={today}>
      <div className="mx-auto grid max-w-6xl gap-6 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <div className="flex min-w-0 flex-col gap-6">
          {home.error && <ErrorNote>{home.error}</ErrorNote>}

          {data && data.health.origins === 0 ? (
            <Panel>
              <div className="flex flex-col items-center gap-3 py-8 text-center">
                <p className="max-w-md text-sm text-muted-foreground">
                  La base documental todavía está vacía. Incorporá la primera carpeta y la app empieza a
                  inventariar tus documentos, sin moverlos ni copiarlos.
                </p>
                <Button onClick={() => setActiveView("origins")}>
                  <FolderPlus /> Agregar una carpeta
                </Button>
              </div>
            </Panel>
          ) : (
            <>
              <Panel title="Requiere atención" count={attention.length}>
                {attention.length === 0 ? (
                  <EmptyNote>Nada pendiente. Todo lo que se pudo procesar está procesado.</EmptyNote>
                ) : (
                  <ul className="flex flex-col divide-y">
                    {attention.map((item) => (
                      <li key={item.id}>
                        <button
                          className="flex w-full items-center gap-3 py-2.5 text-left hover:bg-muted/40"
                          onClick={() => goTo(item)}
                        >
                          <AttentionMark tone={item.tone} />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium">{item.title}</span>
                            <span className="block text-xs text-muted-foreground">{item.detail}</span>
                          </span>
                          <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </Panel>

              <Panel title="Actividad reciente">
                {data && data.activity.length === 0 ? (
                  <EmptyNote>Todavía no pasó nada en las carpetas incorporadas.</EmptyNote>
                ) : (
                  <ul className="flex flex-col divide-y">
                    {(data?.activity ?? []).map((entry, index) => (
                      <li key={index} className="flex items-start gap-3 py-2 text-sm">
                        <span className="min-w-0 flex-1 break-words">{describeActivity(entry)}</span>
                        <span className="shrink-0 text-xs text-muted-foreground">
                          {entry.originName} · {formatAgo(entry.at)}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </Panel>
            </>
          )}
        </div>

        <aside className="flex flex-col gap-6">
          <Panel title="Búsqueda rápida">
            <form
              className="flex gap-2"
              onSubmit={(event) => {
                event.preventDefault()
                search()
              }}
            >
              <Input
                value={query}
                placeholder="nombre, carpeta o texto…"
                onChange={(event) => setQuery(event.target.value)}
              />
              <Button type="submit" variant="outline" size="icon" title="Buscar en los documentos">
                <Search />
              </Button>
            </form>
          </Panel>

          {data && data.health.origins > 0 && (
            <Panel title={vocabulary().archive.health}>
              <dl className="flex flex-col divide-y text-sm">
                <Metric label="Documentos distintos">{formatCount(data.health.documents)}</Metric>
                <Metric label="Archivos en el inventario">{formatCount(data.health.locations)}</Metric>
                <Metric label="Procesados">
                  {processedPercent(data.health) === null ? "—" : `${processedPercent(data.health)!.toLocaleString("es-AR")} %`}
                </Metric>
                <Metric label="En curso">
                  {pendingCount(data.health) > 0 ? (
                    <StatusBadge tone="busy">{formatCount(pendingCount(data.health))}</StatusBadge>
                  ) : (
                    "0"
                  )}
                </Metric>
                <Metric label="Con copias en otro lugar">{formatCount(data.health.copies)}</Metric>
                <Metric label="Palabras en la base">{formatCount(data.health.words)}</Metric>
                <Metric label="Carpetas">
                  {data.health.unreachableOrigins > 0 ? (
                    <StatusBadge tone="warning">
                      {data.health.unreachableOrigins} de {data.health.origins} sin conexión
                    </StatusBadge>
                  ) : (
                    plural(data.health.origins, "conectada", "conectadas")
                  )}
                </Metric>
                <Metric label="Último barrido">
                  {data.health.lastScanAt === null ? "en curso" : formatAgo(data.health.lastScanAt)}
                </Metric>
              </dl>
            </Panel>
          )}
        </aside>
      </div>
    </DocbasePage>
  )
}

function AttentionMark({ tone }: { tone: AttentionItem["tone"] }) {
  const color =
    tone === "error"
      ? "bg-destructive"
      : tone === "warning"
        ? "bg-amber-500"
        : tone === "busy"
          ? "bg-blue-500"
          : "bg-muted-foreground/50"
  return <span className={`size-2 shrink-0 rounded-full ${color}`} />
}

function Metric({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 py-1.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-medium">{children}</dd>
    </div>
  )
}

function fileName(relPath: string): string {
  return relPath.split("/").pop() ?? relPath
}

function describeActivity(entry: Activity): string {
  const one = entry.count === 1
  switch (entry.kind) {
    case "added":
      return one ? `Apareció ${fileName(entry.relPath)}` : `Aparecieron ${formatCount(entry.count)} archivos`
    case "modified":
      return one ? `Cambió ${fileName(entry.relPath)}` : `Cambiaron ${formatCount(entry.count)} archivos`
    case "removed":
      return one ? `Ya no está ${fileName(entry.relPath)}` : `Ya no están ${formatCount(entry.count)} archivos`
    case "moved":
      return entry.fromPath
        ? `Se movió ${entry.fromPath} a ${entry.relPath}`
        : `Se movió ${fileName(entry.relPath)}`
    default:
      return fileName(entry.relPath)
  }
}
