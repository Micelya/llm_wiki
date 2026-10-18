import { useEffect, useState } from "react"
import { Input } from "@/components/ui/input"
import { listEntities } from "@/commands/docbase"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"
import { entityTypeName, vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { formatCount, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { EntityDetailView } from "./entity-detail"

/**
 * The people and organizations the documents of the archive name: a list
 * with a search box and a filter by type, and the detail of each one.
 */
export function EntitiesView() {
  const entityId = useKnowledgeStore((s) => s.entityId)
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  if (entityId !== null) {
    return <EntityDetailView key={entityId} id={entityId} onBack={() => openEntity(null)} />
  }
  return <EntityList />
}

function EntityList() {
  const words = vocabulary()
  const kind = useKnowledgeStore((s) => s.kind)
  const setKind = useKnowledgeStore((s) => s.setKind)
  const query = useKnowledgeStore((s) => s.query)
  const setQuery = useKnowledgeStore((s) => s.setQuery)
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const [typed, setTyped] = useState(query)

  // Typing searches after a short pause, not on every key.
  useEffect(() => {
    if (typed === query) return
    const timer = setTimeout(() => setQuery(typed), 250)
    return () => clearTimeout(timer)
  }, [typed, query, setQuery])

  const entities = useDocbaseQuery(`entities:${kind ?? ""}:${query}`, () => listEntities(kind, query.trim() || null))
  const kinds = Object.keys(words.knowledge.entityTypes)
  const rows = entities.data ?? []

  return (
    <DocbasePage
      title={words.knowledge.entities}
      subtitle={words.knowledge.entitiesDescription}
      actions={
        <button className="text-sm text-primary hover:underline" onClick={() => setActiveView("identity-doubts")}>
          {words.knowledge.doubts}
        </button>
      }
    >
      <div className="mx-auto flex max-w-5xl flex-col gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="h-8 w-72"
            placeholder="Buscar por nombre"
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
          />
          <div className="flex flex-wrap gap-1" role="group" aria-label="Tipo">
            <KindButton active={kind === null} onClick={() => setKind(null)}>
              Todos
            </KindButton>
            {kinds.map((id) => (
              <KindButton key={id} active={kind === id} onClick={() => setKind(id)}>
                {entityTypeName(id, true, words)}
              </KindButton>
            ))}
          </div>
          {entities.data && (
            <span className="ml-auto text-xs text-muted-foreground">{plural(rows.length, "resultado", "resultados")}</span>
          )}
        </div>
        {entities.error && <ErrorNote>{entities.error}</ErrorNote>}
        {entities.data && rows.length === 0 && (
          <EmptyNote>
            {query
              ? "Nada coincide con lo que buscás."
              : `Todavía no hay nadie en el registro: los documentos se leen mientras el ${words.archive.wiki.toLowerCase()} está activo.`}
          </EmptyNote>
        )}
        {rows.length > 0 && (
          <ul className="divide-y rounded-lg border bg-card">
            {rows.map((entity) => (
              <li key={entity.id}>
                <button
                  className="flex w-full items-start gap-3 px-4 py-2.5 text-left hover:bg-muted/50"
                  onClick={() => openEntity(entity.id)}
                >
                  <span className="min-w-0 flex-1">
                    <span className="block font-medium break-words">{entity.name}</span>
                    {entity.aliases.length > 0 && (
                      <span className="block text-xs break-words text-muted-foreground">
                        También: {entity.aliases.join(" · ")}
                      </span>
                    )}
                  </span>
                  <StatusBadge tone="neutral">{entityTypeName(entity.kind, false, words)}</StatusBadge>
                  <span className="w-28 shrink-0 text-right text-xs text-muted-foreground">
                    {plural(entity.documents, "documento", "documentos")}
                    {entity.mentions > entity.documents && ` · ${formatCount(entity.mentions)} menciones`}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </DocbasePage>
  )
}

function KindButton({ active, onClick, children }: { active: boolean; onClick: () => void; children: string }) {
  return (
    <button
      className={`rounded-md border px-2.5 py-1 text-xs ${active ? "border-primary bg-primary/10 font-medium" : "hover:bg-muted"}`}
      aria-pressed={active}
      onClick={onClick}
    >
      {children}
    </button>
  )
}
