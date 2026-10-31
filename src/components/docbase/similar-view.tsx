import { useEffect, useRef, useState } from "react"
import { ChevronDown, ChevronRight, LayoutTemplate } from "lucide-react"
import { Button } from "@/components/ui/button"
import { LongName } from "@/components/ui/long-name"
import {
  listOpenSimilarPairs,
  listSimilarDocuments,
  listSimilarGroups,
  listSimilarRelations,
  type RelationDef,
  type SimilarDecision,
  type SimilarDocument,
  type SimilarGroup,
} from "@/commands/docbase"
import { useAttentionViewStore } from "@/stores/attention-view-store"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, Panel, StatusBadge } from "./page"
import { formatCount, plural } from "./format"
import { decideAndFollow, SimilarCard } from "./similar-documents"
import { FamilyTemplate } from "./template-panel"
import { familyPairs, hasCopies, orderFamilies, orderOpenPairs } from "./similar"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * «Copias y versiones» (the vocabulary's `copies`): the documents that may
 * be one same document, by family: the principal and its copies and
 * versions, the relation proposed for each, what changes, and the buttons
 * to join, choose another relation or discard. What changes what goes to
 * the wiki (two that could be the same document) comes first. Documents
 * made with one template are different documents: they make no family
 * and wait for no one (D-497); they are in «Plantillas», not here. The
 * detail of each document keeps showing its copies and versions.
 */
export function SimilarView() {
  const groups = useDocbaseQuery("similar-view:groups", listSimilarGroups)
  const open = useDocbaseQuery("similar-view:open-pairs", listOpenSimilarPairs)
  const relations = useDocbaseQuery("similar-relations", listSimilarRelations)
  const focused = useAttentionViewStore((s) => s.family)
  const [showDecided, setShowDecided] = useState(false)
  const definitions = relations.data ?? []
  const words = vocabulary()

  const ordered = orderFamilies((groups.data ?? []).filter((group) => hasCopies(group, definitions)))
  const waiting = ordered.filter((group) => group.pending > 0 || group.possiblySame > 0)
  const decided = ordered.filter((group) => group.pending === 0 && group.possiblySame === 0)
  const openPairs = orderOpenPairs(open.data ?? [], definitions)
  const sameDocument = waiting.filter((group) => group.pendingSameDocument > 0).length

  return (
    <DocbasePage
      title={words.copies.name}
      subtitle={
        groups.data
          ? `${plural(waiting.length, "familia espera", "familias esperan")} una decisión` +
            (sameDocument > 0 ? `; en ${formatCount(sameDocument)}, dos podrían ser el mismo documento` : "") +
            "."
          : undefined
      }
    >
      <div className="mx-auto flex max-w-5xl flex-col gap-6">
        {groups.error && <ErrorNote>{groups.error}</ErrorNote>}
        <p className="text-sm text-muted-foreground">
          {words.copies.description} Nada se une solo: si dos son el mismo documento, al unirlos cuenta uno solo, el que
          elijas.
        </p>

        {groups.data && waiting.length === 0 && openPairs.length === 0 && (
          <Panel>
            <EmptyNote>No queda ninguna copia ni versión por decidir.</EmptyNote>
          </Panel>
        )}

        {waiting.length > 0 && (
          <section className="flex flex-col gap-3">
            <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">Familias</h2>
            {waiting.map((group) => (
              <FamilyCard
                key={group.principal.contentId}
                group={group}
                relations={definitions}
                initiallyOpen={focused === group.principal.contentId}
              />
            ))}
          </section>
        )}

        {openPairs.length > 0 && (
          <section className="flex flex-col gap-3">
            <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              Pares fuera de las familias
            </h2>
            <p className="text-xs text-muted-foreground">
              Los que no son entre el documento principal de una familia y uno de sus miembros: dos miembros de la misma
              familia, dos documentos que no forman una familia, o un documento que está casi entero dentro de otro.
            </p>
            <ul className="flex flex-col gap-3">
              {openPairs.map((pair) => (
                <OpenPairCard key={`${pair.doc.contentId}-${pair.similar.other.contentId}`} pair={pair} relations={definitions} />
              ))}
            </ul>
          </section>
        )}

        {decided.length > 0 && (
          <section className="flex flex-col gap-3">
            <button
              className="flex items-center gap-1 text-left text-xs font-semibold tracking-wide text-muted-foreground uppercase"
              onClick={() => setShowDecided(!showDecided)}
            >
              {showDecided ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />}
              Familias ya decididas ({formatCount(decided.length)})
            </button>
            {showDecided &&
              decided.map((group) => (
                <FamilyCard key={group.principal.contentId} group={group} relations={definitions} initiallyOpen={false} />
              ))}
          </section>
        )}
      </div>
    </DocbasePage>
  )
}

function FamilyCard({
  group,
  relations,
  initiallyOpen,
}: {
  group: SimilarGroup
  relations: RelationDef[]
  initiallyOpen: boolean
}) {
  const [open, setOpen] = useState(initiallyOpen)
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (initiallyOpen) ref.current?.scrollIntoView({ block: "start" })
  }, [initiallyOpen])

  return (
    <div ref={ref} className="rounded-lg border bg-card" data-family={group.principal.contentId}>
      {/* Where the name and the marks do not fit side by side, the marks go under the name. */}
      <button className="flex w-full flex-wrap items-start gap-x-3 gap-y-1.5 px-4 py-3 text-left" onClick={() => setOpen(!open)}>
        {open ? (
          <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        )}
        <span className="min-w-0 flex-1 basis-60">
          <LongName className="block font-medium">{group.principal.name}</LongName>
          <LongName className="block text-xs text-muted-foreground">
            {`Principal de una familia de ${formatCount(group.members.length)} documentos · ${group.principal.originName} › ${group.principal.relPath}`}
          </LongName>
        </span>
        <span className="ml-auto flex flex-wrap justify-end gap-1">
          {group.pendingSameDocument > 0 && (
            <StatusBadge tone="busy">
              {plural(group.pendingSameDocument, "podría ser el mismo", "podrían ser el mismo")}
            </StatusBadge>
          )}
          {group.possiblySame > 0 && (
            <StatusBadge tone="busy">
              {plural(group.possiblySame, "par sin datos que cambien", "pares sin datos que cambien")}
            </StatusBadge>
          )}
          {group.uncertain > 0 && (
            <span title={vocabulary().included.uncertainHelp} data-uncertain={group.uncertain}>
              <StatusBadge tone="busy">
                {vocabulary().included.uncertain} · {group.uncertain}
              </StatusBadge>
            </span>
          )}
          {group.templateFields !== null && (
            <StatusBadge tone="neutral">
              plantilla: {plural(group.templateFields, "campo", "campos")}
            </StatusBadge>
          )}
          {group.flags.includes("large") && <StatusBadge tone="warning">creció mucho</StatusBadge>}
          {group.flags.includes("dispersed") && <StatusBadge tone="warning">dispersa</StatusBadge>}
        </span>
      </button>
      {open && <FamilyPairs group={group} relations={relations} />}
    </div>
  )
}

function FamilyPairs({ group, relations }: { group: SimilarGroup; relations: RelationDef[] }) {
  const principal = group.principal
  const similar = useDocbaseQuery(`similar:${principal.locationId}`, () => listSimilarDocuments(principal.locationId))
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const [problem, setProblem] = useState<string | null>(null)
  const [showTemplate, setShowTemplate] = useState(group.possiblySame > 0)
  const words = vocabulary()
  const items = similar.data ? familyPairs(similar.data, group, relations) : null

  function open(locationId: number) {
    setActiveView("documents")
    openDocument(locationId)
  }

  async function decide(item: SimilarDocument, decision: SimilarDecision) {
    await decideAndFollow(item, decision, setProblem, similar.reload)
  }

  return (
    <div className="flex flex-col gap-3 border-t px-4 py-3">
      {problem && <ErrorNote>{problem}</ErrorNote>}
      {similar.error && <ErrorNote>{similar.error}</ErrorNote>}
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" onClick={() => open(principal.locationId)}>
          Abrir el principal
        </Button>
        {group.templateFields !== null && (
          <Button size="sm" variant="outline" aria-expanded={showTemplate} onClick={() => setShowTemplate(!showTemplate)}>
            <LayoutTemplate /> {showTemplate ? "Ocultar la plantilla" : "Ver la plantilla"}
          </Button>
        )}
      </div>
      {showTemplate && group.templateFields !== null && (
        <div className="rounded-md border px-4 py-3">
          <FamilyTemplate principalLocationId={principal.locationId} onOpen={open} />
        </div>
      )}
      {items === null ? (
        <p className="text-xs text-muted-foreground">Buscando los pares…</p>
      ) : items.length === 0 ? (
        <EmptyNote>Esta familia no tiene copias ni versiones de su documento principal.</EmptyNote>
      ) : (
        <ul className="flex flex-col gap-3">
          {items.map((item) => (
            <SimilarCard
              key={item.other.contentId}
              item={item}
              relations={relations}
              name={principal.name}
              selfLocationId={principal.locationId}
              words={words}
              onOpen={open}
              onDecide={decide}
            />
          ))}
        </ul>
      )}
    </div>
  )
}

function OpenPairCard({ pair, relations }: { pair: { doc: SimilarGroup["principal"]; similar: SimilarDocument }; relations: RelationDef[] }) {
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const [problem, setProblem] = useState<string | null>(null)
  const words = vocabulary()

  function open(locationId: number) {
    setActiveView("documents")
    openDocument(locationId)
  }

  return (
    <li className="flex flex-col gap-1">
      <button className="self-start text-left text-xs text-muted-foreground hover:underline" onClick={() => open(pair.doc.locationId)}>
        Desde «{pair.doc.name}» ({pair.doc.originName} › {pair.doc.relPath})
      </button>
      {problem && <ErrorNote>{problem}</ErrorNote>}
      <ul>
        <SimilarCard
          item={pair.similar}
          relations={relations}
          name={pair.doc.name}
          selfLocationId={pair.doc.locationId}
          words={words}
          onOpen={open}
          onDecide={(item, decision) => decideAndFollow(item, decision, setProblem)}
        />
      </ul>
    </li>
  )
}
