import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Table, Td, Th } from "@/components/ui/table"
import { FilterChip } from "@/components/ui/tabs"
import { listEntities } from "@/commands/docbase"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"
import { entityTypeName, vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote } from "@/components/docbase/page"
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
  // A reference (a record, a file, an act) is data of its document, not an entity.
  const references = words.knowledge.referenceTypes
  const kinds = Object.keys(words.knowledge.entityTypes).filter((id) => !references.includes(id))
  const rows = (entities.data ?? []).filter((entity) => !references.includes(entity.kind))

  return (
    <DocbasePage
      title={words.knowledge.entities}
      subtitle={words.knowledge.entitiesDescription}
      actions={
        <Button variant="outline" onClick={() => setActiveView("identity-doubts")}>
          {words.knowledge.doubts}
        </Button>
      }
    >
      <div className="flex max-w-5xl flex-col gap-4">
        <div className="flex flex-wrap items-center gap-2">
          <Input
            className="h-8 w-72"
            placeholder="Buscar por nombre"
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
          />
          <div className="flex flex-wrap gap-1.5" role="group" aria-label="Tipo">
            <FilterChip active={kind === null} onClick={() => setKind(null)}>
              Todos
            </FilterChip>
            {kinds.map((id) => (
              <FilterChip key={id} active={kind === id} onClick={() => setKind(id)}>
                {entityTypeName(id, true, words)}
              </FilterChip>
            ))}
          </div>
          {entities.data && (
            <span className="ml-auto text-[0.8125rem] text-muted-foreground tabular-nums">{plural(rows.length, "resultado", "resultados")}</span>
          )}
        </div>
        {entities.error && <ErrorNote>{entities.error}</ErrorNote>}
        {entities.data && rows.length === 0 && (
          <EmptyNote>
            {query
              ? "Nada coincide con lo que buscás."
              : "Todavía no hay nadie en el registro: los documentos se leen mientras la lectura automática está activa."}
          </EmptyNote>
        )}
        {rows.length > 0 && (
          <Table>
            <thead>
              <tr>
                <Th>Nombre</Th>
                <Th width="11rem">Tipo</Th>
                <Th align="right" width="8rem">
                  Documentos
                </Th>
                <Th align="right" width="8rem">
                  Menciones
                </Th>
              </tr>
            </thead>
            <tbody>
              {rows.map((entity) => (
                <tr key={entity.id} className="cursor-pointer" onClick={() => openEntity(entity.id)}>
                  <Td>
                    <button className="text-left font-semibold break-words text-link hover:underline" onClick={() => openEntity(entity.id)}>
                      {entity.name}
                    </button>
                    {entity.aliases.length > 0 && (
                      <span className="block text-[0.78125rem] break-words text-muted-foreground">También: {entity.aliases.join(" · ")}</span>
                    )}
                  </Td>
                  <Td muted>{entityTypeName(entity.kind, false, words)}</Td>
                  <Td align="right">{formatCount(entity.documents)}</Td>
                  <Td align="right" muted>
                    {formatCount(entity.mentions)}
                  </Td>
                </tr>
              ))}
            </tbody>
          </Table>
        )}
      </div>
    </DocbasePage>
  )
}
