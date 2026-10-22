import { useState } from "react"
import { Pencil, Plus, Undo2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  chooseMatterDocument,
  createMatter,
  decideMatter,
  getKnowledgeArticle,
  getMatter,
  leaveMatter,
  listMatters,
  renameMatter,
  undoIdentityDecision,
  type KnowledgeArticle,
  type MatterConfirmation,
  type MatterDetail,
  type MatterDocument,
  type MatterReason,
} from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { entityTypeName, relationName, vocabulary, type Vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { formatDate, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { ArticleText } from "./article-text"
import { DetailTabs, NoArticle, useDetailTab, type DetailTab } from "./detail-tabs"
import { articleBody, matterActions, rolesInWords } from "./knowledge"
import { DocumentPicker, NameForm } from "./matter-edit"
import { ResearchSection } from "./research-section"
import { Section, useAction, useOpenDocument, useOpenEntity } from "./shared"

/**
 * The matters of the archive: groups of documents joined by what is
 * specific to one matter (R-05), with the reasons each document is in it,
 * its parties, the documents the model proposes for it, and what a
 * person decided. What the app groups is a proposal until a person
 * confirms it (D-381); a person confirms or discards it, creates one,
 * names it, and puts a document in or takes it out (D-386).
 */
export function MattersView() {
  const matterId = useKnowledgeStore((s) => s.matterId)
  const openMatter = useKnowledgeStore((s) => s.openMatter)
  if (matterId !== null) return <MatterDetailView key={matterId} id={matterId} onBack={() => openMatter(null)} />
  return <MatterList />
}

function MatterList() {
  const words = vocabulary()
  const matters = useDocbaseQuery("matters", listMatters)
  const openMatter = useKnowledgeStore((s) => s.openMatter)
  const action = useAction()
  const [creating, setCreating] = useState(false)
  const rows = matters.data ?? []
  const name = words.knowledge.matter.toLowerCase()
  return (
    <DocbasePage title={words.knowledge.matters} subtitle={words.knowledge.mattersDescription}>
      <div className="mx-auto flex max-w-5xl flex-col gap-4">
        {matters.error && <ErrorNote>{matters.error}</ErrorNote>}
        {action.problem && <ErrorNote>{action.problem}</ErrorNote>}
        <div>
          {creating ? (
            <NameForm
              placeholder={`Nombre del ${name}`}
              saveLabel={`Crear ${name}`}
              busy={action.busy}
              onCancel={() => setCreating(false)}
              onSave={(value) =>
                void action.run(async () => {
                  const id = await createMatter(value)
                  setCreating(false)
                  openMatter(id)
                })
              }
            />
          ) : (
            <Button size="sm" variant="outline" onClick={() => setCreating(true)}>
              <Plus /> Nuevo {name}
            </Button>
          )}
        </div>
        {matters.data && rows.length === 0 && (
          <EmptyNote>
            Todavía no hay documentos agrupados: hacen falta dos que compartan algo propio de un mismo {name}. También
            podés crear uno y agregarle documentos.
          </EmptyNote>
        )}
        {rows.length > 0 && (
          <ul className="divide-y rounded-lg border bg-card">
            {rows.map((matter) => (
              <li key={matter.id}>
                <button className="flex w-full items-start gap-3 px-4 py-2.5 text-left hover:bg-muted/50" onClick={() => openMatter(matter.id)}>
                  <span className="min-w-0 flex-1 font-medium break-words">{matter.name}</span>
                  <MatterBadges confirmation={matter.confirmation} status={matter.status} words={words} />
                  {matter.proposed > 0 && (
                    <StatusBadge tone="busy">{plural(matter.proposed, "propuesta", "propuestas")}</StatusBadge>
                  )}
                  <span className="w-28 shrink-0 text-right text-xs text-muted-foreground">
                    {plural(matter.documents, "documento", "documentos")}
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

/** Whether a matter is a proposal of the app or confirmed, and its status. */
export function MatterBadges({ confirmation, status, words }: { confirmation: MatterConfirmation; status: string; words: Vocabulary }) {
  const states = words.knowledge.matterStates
  return (
    <>
      {confirmation === "proposed" && <StatusBadge tone="busy">{states.proposed}</StatusBadge>}
      {confirmation === "confirmed" && <StatusBadge tone="ok">{states.confirmed}</StatusBadge>}
      <StatusBadge tone="neutral">{states.status[status] ?? status}</StatusBadge>
    </>
  )
}

/** Why a document is in its matter, in words. */
export function reasonInWords(reason: MatterReason, words: Vocabulary = vocabulary()): string {
  const other = reason.with ? ` con «${reason.with.name}»` : ""
  const matter = words.knowledge.matter.toLowerCase()
  switch (reason.kind) {
    case "persona":
      return `una persona decidió que es del mismo ${matter}${other ? ` que «${reason.with?.name}»` : ""}`
    case "documento":
      return `unido por una persona${other} como «${relationName(reason.value, words)}»`
    case "asunto":
      return `nombra el mismo ${matter}${reason.valueName ? ` («${reason.valueName}»)` : ""}${other ? ` que «${reason.with?.name}»` : ""}`
    case "numero":
      return `comparte el número ${reason.value}${other}`
    case "importe":
      return `comparte el importe ${reason.value}${other}`
    default:
      return `${reason.kind} ${reason.value}${other}`
  }
}

/**
 * One matter, with the same two tabs as a person or organization (D-389,
 * D-423): «Artículo», the one the base writes for it (its parties with
 * their role, what its documents say with their quotes as notes, its
 * documents once with their type, and why they go together) and the
 * research on internet; «Datos y correcciones»,
 * what a person decides of it: its name, confirming or discarding it, its
 * documents firm and proposed, putting one in or taking it out, and the
 * decisions, to undo. The notes of a person at the foot of both.
 */
export function MatterDetailView({ id, onBack, initialTab = null }: { id: string; onBack: () => void; initialTab?: DetailTab | null }) {
  const words = vocabulary()
  const matter = useDocbaseQuery(`matter:${id}`, () => getMatter(id))
  const article = useDocbaseQuery(`article:${id}`, () => getKnowledgeArticle(id))
  const action = useAction()
  const [renaming, setRenaming] = useState(false)
  const [tab, setTab] = useDetailTab(!article.loading && !article.data, initialTab)
  const detail = matter.data
  const can = detail ? matterActions(detail) : null
  const name = words.knowledge.matter.toLowerCase()
  const data = tab === "data"
  // What waits for a person: the matter itself while it is a proposal, and each document the model proposes.
  const pending = detail ? detail.documents.filter((d) => d.state === "proposed").length + (can?.confirm ? 1 : 0) : 0
  return (
    <DetailTabs
      back={words.knowledge.matters}
      onBack={onBack}
      title={
        <div className="flex flex-wrap items-center gap-2">
          {detail && renaming && data ? (
            <NameForm
              initial={detail.name}
              placeholder={`Nombre del ${name}`}
              saveLabel="Guardar"
              busy={action.busy}
              onCancel={() => setRenaming(false)}
              onSave={(value) =>
                void action.run(async () => {
                  await renameMatter(detail.id, value)
                  setRenaming(false)
                })
              }
            />
          ) : (
            <>
              <h1 className="min-w-0 font-serif text-2xl break-words">{detail?.name ?? words.knowledge.matter}</h1>
              {detail && can?.edit && data && (
                <Button size="icon-xs" variant="ghost" title="Cambiar el nombre" onClick={() => setRenaming(true)}>
                  <Pencil />
                </Button>
              )}
            </>
          )}
          {detail && detail.confirmation !== "discarded" && (
            <MatterBadges confirmation={detail.confirmation} status={detail.status} words={words} />
          )}
          {detail?.origin === "person" && <span className="text-xs text-muted-foreground">creado a mano</span>}
        </div>
      }
      actions={
        detail && data ? (
          <>
            {can?.confirm && (
              <Button disabled={action.busy} onClick={() => action.run(() => decideMatter(detail.id, "confirm"))}>
                Confirmar
              </Button>
            )}
            {can?.discard && (
              <Button variant="outline" disabled={action.busy} onClick={() => action.run(() => decideMatter(detail.id, "discard"))}>
                Descartar
              </Button>
            )}
            {can?.reopen && (
              <Button variant="outline" disabled={action.busy} onClick={() => action.run(() => decideMatter(detail.id, "reopen"))}>
                <Undo2 /> Volver a propuesto
              </Button>
            )}
          </>
        ) : null
      }
      tab={tab}
      onTab={setTab}
      pending={pending}
      notesOf={detail?.id ?? null}
      notice={
        <>
          {(matter.error || action.problem) && (
            <div className="mb-4">
              <ErrorNote>{matter.error ?? action.problem}</ErrorNote>
            </div>
          )}
          {!matter.loading && !detail && !matter.error && (
            <EmptyNote>Este {name} ya no existe: sus documentos se agruparon de otra manera.</EmptyNote>
          )}
          {detail?.confirmation === "discarded" && (
            <p className="mb-4 rounded-md border border-dashed px-3 py-2 text-sm">
              Descartaste este {name}
              {detail.origin === "person" ? "." : ": no vuelve mientras la app encuentre los mismos documentos."}
            </p>
          )}
          {detail?.state === "retired" && detail.confirmation !== "discarded" && (
            <p className="mb-4 rounded-md border border-dashed px-3 py-2 text-sm">Ya no queda ningún grupo con estos documentos.</p>
          )}
        </>
      }
      article={
        detail ? <MatterArticle matter={detail} article={article.data} loading={article.loading} onShowData={() => setTab("data")} /> : null
      }
      data={
        detail ? (
          <>
            <Documents matter={detail} run={action.run} busy={action.busy} />
            <Parties matter={detail} />
            <Decisions matter={detail} run={action.run} busy={action.busy} />
          </>
        ) : null
      }
    />
  )
}

type Run = (action: () => Promise<unknown>) => Promise<void>

/**
 * The article of the matter, as the projection last wrote it in the base
 * (the screen writes the name above it), and below it the research on
 * internet. A matter with too few documents, or made by hand and not yet
 * passed over, has none: the tab says so.
 */
function MatterArticle({
  matter,
  article,
  loading,
  onShowData,
}: {
  matter: MatterDetail
  article: KnowledgeArticle | null
  loading: boolean
  onShowData: () => void
}) {
  const words = vocabulary()
  if (loading && !article) return null
  const name = words.knowledge.matter.toLowerCase()
  return (
    <article>
      {article ? (
        <ArticleText body={articleBody(article.body)} writtenAt={article.writtenAt} />
      ) : (
        <NoArticle onShowData={onShowData}>
          Este {name} todavía no tiene artículo: la app lo escribe desde sus documentos en su próxima pasada, cuando tiene
          documentos suficientes.
        </NoArticle>
      )}
      <div className="mt-8">
        <ResearchSection key={`research-${matter.id}`} objectId={matter.id} name={matter.name} />
      </div>
    </article>
  )
}

function Documents({ matter, run, busy }: { matter: MatterDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const openDocument = useOpenDocument()
  const firm = matter.documents.filter((d) => d.state === "firm")
  const proposed = matter.documents.filter((d) => d.state === "proposed")
  const name = words.knowledge.matter.toLowerCase()
  const editable = matterActions(matter).edit
  // A proposed matter keeps one document at least: to have none, it is discarded.
  const canTakeOut = editable && (matter.origin === "person" || firm.length > 1)

  async function takeOut(doc: MatterDocument) {
    const confirmed = await dialog.confirm({
      title: `Sacar del ${name}`,
      message:
        `«${doc.document.name}» deja de ser de este ${name}, y ninguna pasada lo vuelve a poner. ` +
        "Puede seguir en otros. Se puede volver a agregar.",
      confirmLabel: "Sacar",
      cancelLabel: "Cancelar",
    })
    if (confirmed) await run(() => chooseMatterDocument(matter.id, `content:${doc.document.contentId}`, "out"))
  }

  const row = (doc: MatterDocument) => (
    <li key={doc.document.contentId} className="px-3 py-2 text-sm">
      <div className="flex flex-wrap items-baseline gap-2">
        <button className="text-left text-primary hover:underline" onClick={() => openDocument(doc.document.locationId)}>
          {doc.document.name || `Documento ${doc.document.contentId}`}
        </button>
        <span className="ml-auto flex gap-2">
          {doc.state === "firm" ? (
            canTakeOut && (
              <Button size="xs" variant="ghost" disabled={busy} onClick={() => void takeOut(doc)}>
                No es de este {name}
              </Button>
            )
          ) : (
            <>
              <Button
                size="xs"
                variant="outline"
                disabled={busy}
                onClick={() => void run(() => chooseMatterDocument(matter.id, `content:${doc.document.contentId}`, "in"))}
              >
                Confirmar
              </Button>
              <Button
                size="xs"
                variant="ghost"
                disabled={busy}
                onClick={() => void run(() => leaveMatter(`content:${doc.document.contentId}`, matter.id, "propuesta del modelo descartada"))}
              >
                Descartar
              </Button>
            </>
          )}
        </span>
      </div>
      {doc.reasons.length > 0 && (
        <ul className="mt-1 flex flex-col gap-0.5 pl-3 text-xs text-muted-foreground">
          {doc.reasons.map((reason, index) => (
            <li key={index}>{reasonInWords(reason, words)}</li>
          ))}
        </ul>
      )}
      {doc.proposalReason && <p className="mt-1 pl-3 text-xs text-muted-foreground">El modelo: «{doc.proposalReason}»</p>}
    </li>
  )

  return (
    <>
      <Section title={`Documentos (${firm.length})`}>
        <p className="mb-2 text-sm text-muted-foreground">
          Debajo de cada uno, por qué está en este {name}. Compartir una persona o una organización no alcanza.
        </p>
        {firm.length === 0 ? (
          <p className="text-sm text-muted-foreground">Todavía no tiene documentos: buscalos abajo y agregalos.</p>
        ) : (
          <ul className="flex flex-col divide-y rounded-md border">{firm.map(row)}</ul>
        )}
      </Section>
      {editable && (
        <Section title="Agregar documentos">
          <DocumentPicker
            already={firm.map((d) => d.document.contentId)}
            busy={busy}
            onPick={(found) => void run(() => chooseMatterDocument(matter.id, `content:${found.contentId}`, "in"))}
          />
        </Section>
      )}
      {proposed.length > 0 && (
        <Section title={`Posiblemente de este ${name}`}>
          <p className="mb-2 text-sm text-muted-foreground">
            Ningún dato propio los une, pero el modelo cree que son de este {name}. Hasta que lo confirmes es solo una propuesta.
          </p>
          <ul className="flex flex-col divide-y rounded-md border border-dashed">{proposed.map(row)}</ul>
        </Section>
      )}
    </>
  )
}

function Parties({ matter }: { matter: MatterDetail }) {
  const words = vocabulary()
  const openEntity = useOpenEntity()
  if (matter.parties.length === 0) return null
  return (
    <Section title="Partes y personas">
      <ul className="flex flex-col gap-1 text-sm">
        {matter.parties.map((party) => (
          <li key={party.entityId} className="flex flex-wrap items-baseline gap-2">
            <button className="text-left text-primary hover:underline" onClick={() => openEntity(party.entityId)}>
              {party.name}
            </button>
            <span className="text-xs text-muted-foreground">
              {entityTypeName(party.kind, false, words).toLowerCase()} · {rolesInWords(party.roles, words)} ·{" "}
              {plural(party.documents, "documento", "documentos")}
            </span>
          </li>
        ))}
      </ul>
    </Section>
  )
}

function Decisions({ matter, run, busy }: { matter: MatterDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const matterWord = words.knowledge.matter.toLowerCase()
  const mattersWord = words.knowledge.matters.toLowerCase()
  if (matter.decisions.length === 0) return null
  const nameOf = (key: string | null) => {
    const content = Number(key?.replace("content:", ""))
    return matter.documents.find((d) => d.document.contentId === content)?.document.name ?? "otro documento"
  }
  return (
    <Section title="Decisiones">
      <ul className="flex flex-col divide-y rounded-md border text-sm">
        {matter.decisions.map((decision) => (
          <li key={decision.id} className="flex items-center gap-3 px-3 py-2">
            <span className="min-w-0 flex-1">
              <span className={decision.state === "undone" ? "text-muted-foreground line-through" : ""}>
                {decision.kind === "join" ? `Del mismo ${matterWord}` : `De ${mattersWord} distintos`}: «{nameOf(decision.a)}» y «
                {nameOf(decision.b)}»
              </span>
              <span className="block text-xs text-muted-foreground">
                {formatDate(decision.decidedAt)}
                {decision.note && ` · ${decision.note}`}
              </span>
            </span>
            {decision.state === "active" && (
              <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run(() => undoIdentityDecision(decision.id))}>
                <Undo2 /> Deshacer
              </Button>
            )}
          </li>
        ))}
      </ul>
    </Section>
  )
}
