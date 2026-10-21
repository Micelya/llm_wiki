import { useEffect, useRef, useState } from "react"
import { CheckCheck, ChevronDown, ChevronRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  decideSimilarMany,
  listOpenSimilarPairs,
  listSimilarDocuments,
  listSimilarGroups,
  listSimilarRelations,
  type RelationDef,
  type SimilarDecision,
  type SimilarDocument,
  type SimilarGroup,
} from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useAttentionViewStore } from "@/stores/attention-view-store"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { relationName, vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, Panel, StatusBadge } from "./page"
import { formatCount, plural } from "./format"
import { decideAndFollow, SimilarCard } from "./similar-documents"
import { familyPairs, orderFamilies, orderOpenPairs, templateConfirmations, TEMPLATE_RELATION } from "./similar"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * The similar documents, by family: the principal and its members, the
 * relation proposed for each, what changes, and the buttons to join,
 * choose another relation or discard. What changes what goes to the wiki
 * (two that could be the same document) comes first. A family of the
 * same template is confirmed in one click, so the passes that come after
 * have a base a person confirmed (R-03). The detail of each document
 * keeps showing its similar documents.
 */
export function SimilarView() {
  const groups = useDocbaseQuery("similar-view:groups", listSimilarGroups)
  const open = useDocbaseQuery("similar-view:open-pairs", listOpenSimilarPairs)
  const relations = useDocbaseQuery("similar-relations", listSimilarRelations)
  const focused = useAttentionViewStore((s) => s.family)
  const [showDecided, setShowDecided] = useState(false)
  const definitions = relations.data ?? []

  const ordered = orderFamilies(groups.data ?? [])
  const waiting = ordered.filter((group) => group.pending > 0)
  const decided = ordered.filter((group) => group.pending === 0)
  const openPairs = orderOpenPairs(open.data ?? [], definitions)
  const sameDocument = waiting.filter((group) => group.pendingSameDocument > 0).length

  return (
    <DocbasePage
      title="Documentos parecidos"
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
          Nada se une solo. Si dos son el mismo documento (prácticamente el mismo, un ejemplar firmado, una versión),
          al unirlos cuenta uno solo, el que elijas. Si comparten la plantilla con otros datos, son documentos
          distintos: confirmarlo no cambia lo que cuenta, pero da una base confirmada a las pasadas que vienen.
        </p>

        {groups.data && waiting.length === 0 && openPairs.length === 0 && (
          <Panel>
            <EmptyNote>No queda ningún parecido por decidir.</EmptyNote>
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
              Parecidos que no son entre el documento principal de una familia y uno de sus miembros: dos miembros de
              la misma familia, o un documento que está casi entero dentro de otro.
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
  const sameTemplatePending = group.members.filter(
    (member) => member.state === "proposed" && member.relation === TEMPLATE_RELATION,
  ).length

  return (
    <div ref={ref} className="rounded-lg border bg-card" data-family={group.principal.contentId}>
      <button className="flex w-full items-start gap-3 px-4 py-3 text-left" onClick={() => setOpen(!open)}>
        {open ? (
          <ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        )}
        <span className="min-w-0 flex-1">
          <span className="block font-medium break-words">{group.principal.name}</span>
          <span className="block text-xs break-words text-muted-foreground">
            Principal de una familia de {formatCount(group.members.length)} documentos · {group.principal.originName} ›{" "}
            {group.principal.relPath}
          </span>
        </span>
        <span className="flex shrink-0 flex-wrap justify-end gap-1">
          {group.pendingSameDocument > 0 && (
            <StatusBadge tone="busy">
              {plural(group.pendingSameDocument, "podría ser el mismo", "podrían ser el mismo")}
            </StatusBadge>
          )}
          {sameTemplatePending > 0 && (
            <StatusBadge tone="neutral">{formatCount(sameTemplatePending)} de la misma plantilla, sin confirmar</StatusBadge>
          )}
          {group.flags.includes("large") && <StatusBadge tone="warning">creció mucho</StatusBadge>}
          {group.flags.includes("dispersed") && <StatusBadge tone="warning">dispersa</StatusBadge>}
        </span>
      </button>
      {open && <FamilyPairs group={group} relations={relations} sameTemplatePending={sameTemplatePending} />}
    </div>
  )
}

function FamilyPairs({
  group,
  relations,
  sameTemplatePending,
}: {
  group: SimilarGroup
  relations: RelationDef[]
  sameTemplatePending: number
}) {
  const principal = group.principal
  const similar = useDocbaseQuery(`similar:${principal.locationId}`, () => listSimilarDocuments(principal.locationId))
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const dialog = useAppDialog()
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const words = vocabulary()
  const items = similar.data ? familyPairs(similar.data, group, relations) : null

  function open(locationId: number) {
    setActiveView("documents")
    openDocument(locationId)
  }

  async function decide(item: SimilarDocument, decision: SimilarDecision) {
    await decideAndFollow(item, decision, setProblem, similar.reload)
  }

  async function confirmTemplate() {
    if (!similar.data) return
    const decisions = templateConfirmations(similar.data, group)
    if (decisions.length === 0) return
    const confirmed = await dialog.confirm({
      title: "Confirmar la familia",
      message:
        `Se unen ${plural(decisions.length, "documento", "documentos")} con «${principal.name}» como ` +
        `«${relationName(TEMPLATE_RELATION, words)}»: la misma plantilla con otros datos. Son documentos distintos, ` +
        "así que todos siguen contando. Las pasadas que vienen se apoyan en lo que confirmes. Se puede deshacer " +
        "de a uno, desde cada par.",
      confirmLabel: "Confirmar",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    setBusy(true)
    setProblem(null)
    try {
      await decideSimilarMany(decisions)
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
      similar.reload()
    }
  }

  return (
    <div className="flex flex-col gap-3 border-t px-4 py-3">
      {problem && <ErrorNote>{problem}</ErrorNote>}
      {similar.error && <ErrorNote>{similar.error}</ErrorNote>}
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="outline" onClick={() => open(principal.locationId)}>
          Abrir el principal
        </Button>
        {sameTemplatePending > 0 && (
          <Button size="sm" disabled={busy || !similar.data} onClick={() => void confirmTemplate()}>
            <CheckCheck /> Confirmar la familia como «{relationName(TEMPLATE_RELATION, words)}» (
            {formatCount(sameTemplatePending)})
          </Button>
        )}
      </div>
      {items === null ? (
        <p className="text-xs text-muted-foreground">Buscando los pares…</p>
      ) : items.length === 0 ? (
        <EmptyNote>Esta familia ya no tiene pares.</EmptyNote>
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
