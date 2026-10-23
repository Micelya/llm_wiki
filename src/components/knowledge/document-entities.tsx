import { chooseMatterDocument, getDocumentEntities } from "@/commands/docbase"
import { Button } from "@/components/ui/button"
import { entityTypeName, vocabulary } from "@/vocabulary"
import { StatusBadge } from "@/components/docbase/page"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { rolesInWords } from "./knowledge"
import { AddToMatter } from "./matter-edit"
import { ReadAgainButton } from "./read-again"
import { Section, useAction, useOpenEntity, useOpenMatter } from "./shared"

/**
 * Who a document names, with the role each one has in it, and its matter:
 * what the knowledge layer read in it. Each name leads to its entity.
 */
export function DocumentEntitiesSection({ locationId }: { locationId: number }) {
  const words = vocabulary()
  const found = useDocbaseQuery(`document-entities:${locationId}`, () => getDocumentEntities(locationId))
  const openEntity = useOpenEntity()
  const openMatter = useOpenMatter()
  const action = useAction()
  const data = found.data
  if (!data || data.contentId === null) return null
  const title = `${words.knowledge.entities} (${data.entities.length})`
  const read = data.state !== "pending" || data.reread
  return (
    <Section
      title={title}
      action={read && !data.reread ? <ReadAgainButton target={{ locationId }} onDone={found.reload} /> : undefined}
    >
      {data.reread && (
        <p className="mb-2 text-sm text-muted-foreground">
          Pediste volver a leerlo con el modelo: se lee de nuevo primero que el resto. Hasta entonces queda la lectura que tiene.
        </p>
      )}
      {data.older && !data.reread && data.state === "done" && (
        <p className="mb-2 text-xs text-muted-foreground">
          Se leyó con una versión anterior de la lectura. Sigue valiendo y se controla con las reglas de ahora; para leerlo
          de nuevo con la versión actual, usá «Volver a leer con el modelo».
        </p>
      )}
      {(data.state === "pending" || data.state === "stale") && !data.reread ? (
        <p className="text-sm text-muted-foreground">
          {data.state === "pending" ? "Todavía no se leyó" : "Se va a volver a leer porque cambió su texto o falló su lectura"}:
          se lee mientras la lectura automática está activa.
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
              <button className="text-left text-link hover:underline" onClick={() => openMatter(matter.matterId)}>
                {matter.name}
              </button>
              {matter.proposed && <StatusBadge tone="neutral">propuesto por el modelo</StatusBadge>}
              {!matter.proposed && (
                <Button
                  size="xs"
                  variant="ghost"
                  disabled={action.busy}
                  title={`Sacar este documento del ${words.knowledge.matter.toLowerCase()}`}
                  onClick={() => void action.run(() => chooseMatterDocument(matter.matterId, `location:${locationId}`, "out"))}
                >
                  Sacar
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      {action.problem && <p className="mb-2 text-sm text-destructive">{action.problem}</p>}
      <AddToMatter locationId={locationId} current={data.matters} />
      {data.state === "done" && data.entities.length === 0 && (
        <p className="text-sm text-muted-foreground">El documento no nombra a nadie que la lectura haya encontrado.</p>
      )}
      {data.entities.length > 0 && (
        <ul className="flex flex-col divide-y rounded-[10px] border bg-card text-sm">
          {data.entities.map((entity) => (
            <li key={entity.entityId ?? `${entity.kind}:${entity.name}`} className="flex flex-wrap items-baseline gap-2 px-3 py-1.5">
              {entity.entityId ? (
                <button className="text-left text-link hover:underline" onClick={() => openEntity(entity.entityId as string)}>
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
          <ul className="flex flex-col divide-y rounded-[10px] border bg-card text-sm">
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
