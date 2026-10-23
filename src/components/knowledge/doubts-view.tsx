import { useEffect, useRef, useState } from "react"
import { ChevronDown, ChevronRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { decideIdentity, getEntity, listIdentityDoubts, type DoubtView, type EntityDetail } from "@/commands/docbase"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { entityTypeName, identifierName, vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { doubtReason, groupDoubts, rolesInWords, type DoubtGroup, type DoubtSide } from "./knowledge"
import { Quote, useAction, useOpenDocument, useOpenEntity } from "./shared"

/** Quotes shown of each side of a doubt. */
const SHOWN_MENTIONS = 4

/**
 * The identities the app could not decide, one card per decision (the
 * doubts that share an entity go together, as the families of similar
 * documents). Each side shows how its documents write it, its
 * identifiers and quotes; a person says whether they are the same. What
 * was shown is kept with the decision (R-08). When in doubt nothing is
 * joined (R-07): until someone decides, each one keeps its own page.
 */
export function IdentityDoubtsView() {
  const words = vocabulary()
  const doubts = useDocbaseQuery("identity-doubts", listIdentityDoubts)
  const focused = useKnowledgeStore((s) => s.doubtGroup)
  const groups = groupDoubts(doubts.data ?? [])
  const isFocused = (group: DoubtGroup) =>
    focused !== null && (group.id === focused || group.sides.some((side) => side.entityId === focused))
  const first = groups.findIndex(isFocused)

  return (
    <DocbasePage
      title={words.knowledge.doubts}
      subtitle={doubts.data ? `${plural(groups.length, "decisión espera", "decisiones esperan")} a una persona.` : undefined}
    >
      <div className="mx-auto flex max-w-5xl flex-col gap-4">
        {doubts.error && <ErrorNote>{doubts.error}</ErrorNote>}
        <p className="text-sm text-muted-foreground">
          La app une sola lo que comparte un identificador o el nombre completo; lo que no puede saber lo deja acá, sin unir.
          Una unión equivocada mezcla a dos clientes: ante la duda, no las unas. Todo se puede deshacer desde la entidad.
        </p>
        {doubts.data && groups.length === 0 && (
          <EmptyNote>No queda ninguna identidad dudosa.</EmptyNote>
        )}
        {groups.map((group, index) => (
          <GroupCard key={group.id} group={group} initiallyOpen={index === (first >= 0 ? first : 0)} reload={doubts.reload} />
        ))}
      </div>
    </DocbasePage>
  )
}

function GroupCard({ group, initiallyOpen, reload }: { group: DoubtGroup; initiallyOpen: boolean; reload: () => void }) {
  const words = vocabulary()
  const [open, setOpen] = useState(initiallyOpen)
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (initiallyOpen) ref.current?.scrollIntoView({ block: "start" })
  }, [initiallyOpen])
  return (
    <div ref={ref} className="rounded-lg border bg-card">
      <button className="flex w-full items-start gap-3 px-4 py-3 text-left" onClick={() => setOpen(!open)}>
        {open ? (
          <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        )}
        <span className="min-w-0 flex-1">
          <span className="block font-medium break-words">{group.sides.map((side) => `«${side.name}»`).join(" · ")}</span>
          <span className="block text-xs text-muted-foreground">
            {entityTypeName(group.kind, false, words)} · {plural(group.doubts.length, "par por decidir", "pares por decidir")}
          </span>
        </span>
      </button>
      {open && <GroupBody group={group} reload={reload} />}
    </div>
  )
}

function GroupBody({ group, reload }: { group: DoubtGroup; reload: () => void }) {
  const [details, setDetails] = useState<Record<string, EntityDetail | null>>({})
  const key = group.sides
    .map((side) => side.entityId)
    .filter((id): id is string => id !== null)
    .join(",")
  useEffect(() => {
    let stale = false
    const ids = key ? key.split(",") : []
    Promise.all(ids.map((id) => getEntity(id).then((detail) => [id, detail] as const).catch(() => [id, null] as const))).then(
      (found) => {
        if (!stale) setDetails(Object.fromEntries(found))
      },
    )
    return () => {
      stale = true
    }
  }, [key])
  const sideOf = (doubt: DoubtView, which: "a" | "b"): DoubtSide => {
    const entityId = which === "a" ? doubt.entityA : doubt.entityB
    return group.sides.find((side) => (entityId ? side.entityId === entityId : side.key === (which === "a" ? doubt.a : doubt.b))) as DoubtSide
  }
  return (
    <div className="flex flex-col gap-4 border-t px-4 py-3">
      <div className="grid gap-3 md:grid-cols-2">
        {group.sides.map((side) => (
          <SideCard key={side.entityId ?? side.key} side={side} detail={side.entityId ? details[side.entityId] : null} />
        ))}
      </div>
      <ul className="flex flex-col gap-2">
        {group.doubts.map((doubt) => (
          <PairRow
            key={`${doubt.a}|${doubt.b}`}
            doubt={doubt}
            a={sideOf(doubt, "a")}
            b={sideOf(doubt, "b")}
            details={details}
            reload={reload}
          />
        ))}
      </ul>
    </div>
  )
}

/** One side as a person needs to see it to decide. */
function SideCard({ side, detail }: { side: DoubtSide; detail: EntityDetail | null | undefined }) {
  const words = vocabulary()
  const openEntity = useOpenEntity()
  const openDocument = useOpenDocument()
  const documents = detail ? new Set(detail.mentions.map((m) => m.contentId)).size : 0
  return (
    <div className="rounded-md border px-3 py-2 text-sm">
      <div className="flex items-baseline justify-between gap-2">
        {side.entityId ? (
          <button className="text-left font-medium text-link hover:underline" onClick={() => openEntity(side.entityId as string)}>
            {side.name}
          </button>
        ) : (
          <span className="font-medium">{side.name}</span>
        )}
        {detail && <span className="shrink-0 text-xs text-muted-foreground">{plural(documents, "documento", "documentos")}</span>}
      </div>
      {detail === undefined && side.entityId && <p className="text-xs text-muted-foreground">Buscando…</p>}
      {detail && (
        <>
          {detail.aliases.length > 1 && (
            <p className="text-xs text-muted-foreground">Se escribe: {detail.aliases.map((a) => a.written).join(" · ")}</p>
          )}
          {detail.identifiers.length > 0 && (
            <p className="text-xs">
              {detail.identifiers.map((i) => `${identifierName(i.kind, words)} ${i.value}`).join(" · ")}
            </p>
          )}
          <ul className="mt-1 flex flex-col gap-1">
            {detail.mentions.slice(0, SHOWN_MENTIONS).map((mention) => (
              <li key={mention.mentionId} className="text-xs">
                <Quote>{mention.quote}</Quote>{" "}
                <span className="text-muted-foreground">
                  —{" "}
                  <button className="hover:underline" onClick={() => openDocument(mention.locationId)}>
                    {mention.document}
                  </button>
                  , {rolesInWords([mention.role], words)}
                </span>
              </li>
            ))}
            {detail.mentions.length > SHOWN_MENTIONS && (
              <li className="text-xs text-muted-foreground">y {plural(detail.mentions.length - SHOWN_MENTIONS, "mención más", "menciones más")}</li>
            )}
          </ul>
        </>
      )}
    </div>
  )
}

function PairRow({
  doubt,
  a,
  b,
  details,
  reload,
}: {
  doubt: DoubtView
  a: DoubtSide
  b: DoubtSide
  details: Record<string, EntityDetail | null>
  reload: () => void
}) {
  const action = useAction()
  const shown = () =>
    JSON.stringify({
      reason: doubt.reason,
      detail: doubt.detail,
      sides: [a, b].map((side) => {
        const detail = side.entityId ? details[side.entityId] : null
        return {
          key: side.key,
          name: side.name,
          aliases: detail?.aliases.map((x) => x.written) ?? [],
          identifiers: detail?.identifiers.map((x) => `${x.kind}:${x.value}`) ?? [],
          documents: detail ? [...new Set(detail.mentions.map((m) => m.document))] : [],
        }
      }),
    })
  const decide = (kind: "join" | "separate") =>
    action.run(async () => {
      await decideIdentity({ kind, a: doubt.a, b: doubt.b }, null, shown())
      reload()
    })
  return (
    <li className="rounded-[10px] border border-dashed border-border-strong px-3 py-2 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">
          «{a.name}» y «{b.name}»
        </span>
        <StatusBadge tone="busy">dudosa</StatusBadge>
        <span className="ml-auto flex gap-2">
          <Button size="sm" variant="outline" disabled={action.busy} onClick={() => void decide("join")}>
            Son la misma
          </Button>
          <Button size="sm" variant="outline" disabled={action.busy} onClick={() => void decide("separate")}>
            No son la misma
          </Button>
        </span>
      </div>
      <p className="mt-1 text-xs text-muted-foreground">{doubtReason(doubt.reason, doubt.detail)}</p>
      {action.problem && (
        <div className="mt-2">
          <ErrorNote>{action.problem}</ErrorNote>
        </div>
      )}
    </li>
  )
}
