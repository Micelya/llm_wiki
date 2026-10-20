import { getDocumentEntities } from "@/commands/docbase"
import { entityTypeName, vocabulary } from "@/vocabulary"
import { StatusBadge } from "@/components/docbase/page"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { rolesInWords } from "./knowledge"
import { Section, useOpenEntity, useOpenMatter } from "./shared"

/**
 * Who a document names, with the role each one has in it, and its matter:
 * what the knowledge layer read in it. Each name leads to its entity.
 */
export function DocumentEntitiesSection({ locationId }: { locationId: number }) {
  const words = vocabulary()
  const found = useDocbaseQuery(`document-entities:${locationId}`, () => getDocumentEntities(locationId))
  const openEntity = useOpenEntity()
  const openMatter = useOpenMatter()
  const data = found.data
  if (!data || data.contentId === null) return null
  const title = `${words.knowledge.entities} (${data.entities.length})`
  return (
    <Section title={title}>
      {data.state === "pending" || data.state === "stale" ? (
        <p className="text-sm text-muted-foreground">
          {data.state === "pending" ? "Todavía no se leyó" : "Su texto cambió y se va a volver a leer"}: se lee mientras el{" "}
          {words.archive.wiki.toLowerCase()} está activo.
        </p>
      ) : data.state === "failed" ? (
        <p className="text-sm text-muted-foreground">
          No se pudo leer{data.error ? `: ${data.error}` : "."} Se vuelve a intentar cuando cambia su texto.
        </p>
      ) : null}
      {data.matters.length > 0 && (
        <ul className="mb-3 flex flex-col gap-1 text-sm">
          {data.matters.map((matter) => (
            <li key={matter.matterId} className="flex flex-wrap items-baseline gap-2">
              <span className="text-xs font-medium text-muted-foreground">{words.knowledge.matter}</span>
              <button className="text-left text-primary hover:underline" onClick={() => openMatter(matter.matterId)}>
                {matter.name}
              </button>
              {matter.proposed && <StatusBadge tone="neutral">propuesto por el modelo</StatusBadge>}
            </li>
          ))}
        </ul>
      )}
      {data.state === "done" && data.entities.length === 0 && (
        <p className="text-sm text-muted-foreground">El documento no nombra a nadie que la lectura haya encontrado.</p>
      )}
      {data.entities.length > 0 && (
        <ul className="flex flex-col divide-y rounded-md border text-sm">
          {data.entities.map((entity) => (
            <li key={entity.entityId ?? `${entity.kind}:${entity.name}`} className="flex flex-wrap items-baseline gap-2 px-3 py-1.5">
              {entity.entityId ? (
                <button className="text-left text-primary hover:underline" onClick={() => openEntity(entity.entityId as string)}>
                  {entity.name}
                </button>
              ) : (
                <span>{entity.name}</span>
              )}
              <span className="text-xs text-muted-foreground">
                {entityTypeName(entity.kind, false, words).toLowerCase()} · {rolesInWords(entity.roles, words)}
              </span>
              {entity.written.some((w) => w !== entity.name) && (
                <span className="text-xs text-muted-foreground">
                  · escrito «{entity.written.filter((w) => w !== entity.name).join("», «")}»
                </span>
              )}
              {!entity.entityId && <StatusBadge tone="neutral">sin resolver</StatusBadge>}
            </li>
          ))}
        </ul>
      )}
      {data.references.length > 0 && (
        <div className="mt-3">
          <h4 className="mb-1 text-xs font-medium text-muted-foreground">
            {words.knowledge.references} ({data.references.length})
          </h4>
          <ul className="flex flex-col divide-y rounded-md border text-sm">
            {data.references.map((reference) => (
              <li key={reference.normalized} className="flex flex-col gap-0.5 px-3 py-1.5">
                <span className="flex flex-wrap items-baseline gap-2">
                  <span>{reference.written}</span>
                  {reference.identifiers.map((identifier) => (
                    <span key={`${identifier.kind}:${identifier.value}`} className="text-xs text-muted-foreground">
                      {words.knowledge.identifiers[identifier.kind] ?? identifier.kind} {identifier.value}
                    </span>
                  ))}
                  {reference.alsoWritten.length > 0 && (
                    <span className="text-xs text-muted-foreground">· también «{reference.alsoWritten.join("», «")}»</span>
                  )}
                </span>
                {reference.quotes.map((quote) => (
                  <span key={quote.quote} className="text-xs text-muted-foreground">
                    «{quote.quote}»{quote.page !== null ? ` (pág. ${quote.page})` : ""}
                  </span>
                ))}
              </li>
            ))}
          </ul>
        </div>
      )}
      {data.discarded > 0 && (
        <p className="mt-2 text-xs text-muted-foreground">
          {plural(data.discarded, "cosa que dijo el modelo se descartó", "cosas que dijo el modelo se descartaron")} al
          comprobarlas contra el texto.
        </p>
      )}
    </Section>
  )
}
