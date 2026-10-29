import { useState } from "react"
import { Archive, Check, Lock, LockOpen, Pencil, Plus, Undo2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Callout } from "@/components/ui/callout"
import { LongName } from "@/components/ui/long-name"
import { Table, Td, Th } from "@/components/ui/table"
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
import { applyOperation, undoOperation, type OperationCard } from "@/commands/operations"
import { WikiReader } from "@/components/editor/wiki-reader"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { entityTypeName, relationName, vocabulary, type Vocabulary } from "@/vocabulary"
import { DocbasePage, EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { formatDate, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { ArticleBody, DocumentIcon, indexOf, useArticle, type ArticleModel } from "./article-page"
import { DetailTabs, DetailTitle, NoArticle, useDetailTab, type DetailTab } from "./detail-tabs"
import { articleBody, matterActions, rolesInWords } from "./knowledge"
import { DocumentPicker, NameForm } from "./matter-edit"
import { InfoBox, InfoHeading, InfoPending, InfoRow, InfoRows, PageAside, PageIndex, TextLink } from "./page-aside"
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
  const [showClosed, setShowClosed] = useState(false)
  const all = matters.data ?? []
  // A closed or archived matter leaves the list unless a person asks for it (D-475).
  const closed = all.filter((matter) => matter.status !== "open").length
  const rows = showClosed ? all : all.filter((matter) => matter.status === "open")
  const name = words.knowledge.matter.toLowerCase()
  return (
    <DocbasePage
      title={words.knowledge.matters}
      subtitle={words.knowledge.mattersDescription}
      actions={
        !creating && (
          <Button variant="secondary" onClick={() => setCreating(true)}>
            <Plus /> Nuevo {name}
          </Button>
        )
      }
    >
      <div className="flex max-w-5xl flex-col gap-4">
        {matters.error && <ErrorNote>{matters.error}</ErrorNote>}
        {action.problem && <ErrorNote>{action.problem}</ErrorNote>}
        {creating && (
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
        )}
        {closed > 0 && (
          <label className="flex items-center gap-1.5 text-[0.8125rem] text-muted-foreground">
            <input type="checkbox" className="accent-[var(--primary)]" checked={showClosed} onChange={(event) => setShowClosed(event.target.checked)} />
            Mostrar cerrados y archivados ({closed})
          </label>
        )}
        {matters.data && all.length === 0 && (
          <EmptyNote>
            Todavía no hay documentos agrupados: hacen falta dos que compartan algo propio de un mismo {name}. También
            podés crear uno y agregarle documentos.
          </EmptyNote>
        )}
        {rows.length > 0 && (
          <Table>
            <thead>
              <tr>
                <Th>{words.knowledge.matter}</Th>
                <Th width="16rem">Estado</Th>
                <Th align="right" width="8rem">
                  Documentos
                </Th>
              </tr>
            </thead>
            <tbody>
              {rows.map((matter) => (
                <tr key={matter.id} className="cursor-pointer" onClick={() => openMatter(matter.id)}>
                  <Td>
                    <button className="text-left font-semibold text-link hover:underline" onClick={() => openMatter(matter.id)}>
                      <LongName>{matter.name}</LongName>
                    </button>
                  </Td>
                  <Td>
                    <span className="flex flex-wrap gap-1.5">
                      <MatterBadges confirmation={matter.confirmation} status={matter.status} words={words} />
                      {matter.proposed > 0 && <StatusBadge tone="busy">{plural(matter.proposed, "propuesta", "propuestas")}</StatusBadge>}
                    </span>
                  </Td>
                  <Td align="right" muted>
                    {matter.documents}
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

/** Whether a matter is a proposal of the app or confirmed, and its status. */
export function MatterBadges({ confirmation, status, words }: { confirmation: MatterConfirmation; status: string; words: Vocabulary }) {
  const states = words.knowledge.matterStates
  return (
    <>
      {confirmation === "proposed" && <StatusBadge tone="busy">{states.proposed}</StatusBadge>}
      {confirmation === "confirmed" && (
        <StatusBadge tone="ok" icon={Check}>
          {states.confirmed}
        </StatusBadge>
      )}
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

/** Why the documents of a matter go together, by kind of reason, for its fact box. */
function reasonKindInWords(kind: string, words: Vocabulary): string {
  const matter = words.knowledge.matter.toLowerCase()
  switch (kind) {
    case "persona":
      return "Lo decidió una persona"
    case "documento":
      return "Unidos por una persona"
    case "asunto":
      return `Mismo ${matter} nombrado`
    case "numero":
      return "Mismo número"
    case "importe":
      return "Mismo importe"
    default:
      return kind
  }
}

/**
 * One matter, with the same two tabs as a person or organization (D-389,
 * D-423), as a page of an encyclopedia (D-493): «Artículo», the one the
 * base writes for it (its parties with their role, what its documents say
 * with their quotes as notes, its documents once with their type, and why
 * they go together) and the research on internet, with a fact box and the
 * index of the page at its right; «Datos y correcciones», what a person
 * decides of it: its name, confirming or discarding it, its documents firm
 * and proposed, putting one in or taking it out, and the decisions, to
 * undo. The notes of a person at the foot of both.
 */
export function MatterDetailView({ id, onBack, initialTab = null }: { id: string; onBack: () => void; initialTab?: DetailTab | null }) {
  const words = vocabulary()
  const matter = useDocbaseQuery(`matter:${id}`, () => getMatter(id))
  const article = useDocbaseQuery(`article:${id}`, () => getKnowledgeArticle(id))
  const model = useArticle(article.data ? articleBody(article.data.body) : null)
  const action = useAction()
  const [renaming, setRenaming] = useState(false)
  const [statusDone, setStatusDone] = useState<OperationCard | null>(null)
  const [tab, setTab] = useDetailTab(!article.loading && !article.data, initialTab, { view: "matters", subject: id })
  const detail = matter.data
  const can = detail ? matterActions(detail) : null
  const name = words.knowledge.matter.toLowerCase()
  const data = tab === "data"
  // What waits for a person: the matter itself while it is a proposal, and each document the model proposes.
  const pending = detail ? detail.documents.filter((d) => d.state === "proposed").length + (can?.confirm ? 1 : 0) : 0
  const index = indexOf(model, [
    { id: "investigacion", label: "Investigación en internet" },
    { id: "notas", label: "Notas" },
  ])
  return (
    <DetailTabs
      back={words.knowledge.matters}
      onBack={onBack}
      current={detail?.name ?? words.knowledge.matter}
      kicker={words.knowledge.matter}
      title={
        detail && renaming && data ? (
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
            <DetailTitle>{detail?.name ?? words.knowledge.matter}</DetailTitle>
            {detail && detail.confirmation !== "discarded" && (
              <span className="flex flex-wrap gap-1.5">
                <MatterBadges confirmation={detail.confirmation} status={detail.status} words={words} />
              </span>
            )}
            {detail?.origin === "person" && <span className="text-xs text-muted-foreground">creado a mano</span>}
            {detail && detail.records.length > 0 && (
              <ul className="basis-full space-y-0.5 text-sm text-muted-foreground">
                {detail.records.map((record) => (
                  <li key={`${record.portal}:${record.reference}`}>
                    {words.portals.record} {record.reference}
                    {record.title && (
                      <>
                        {" · "}
                        {words.portals.recordTitle}: {record.title}
                      </>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </>
        )
      }
      facts={model?.facts ? <WikiReader body={model.facts} inline /> : null}
      actions={
        detail && data ? (
          <>
            {can?.edit && !renaming && (
              <Button variant="outline" onClick={() => setRenaming(true)}>
                <Pencil /> Cambiar el nombre
              </Button>
            )}
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
              <Button variant="secondary" disabled={action.busy} onClick={() => action.run(() => decideMatter(detail.id, "reopen"))}>
                <Undo2 /> Volver a propuesto
              </Button>
            )}
            <StatusButtons
              matter={detail}
              busy={action.busy}
              onChange={(estado) =>
                action.run(async () => setStatusDone(await applyOperation("caso_cambiar_estado", { caso: detail.id, estado })))
              }
            />
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
          {statusDone && statusDone.state === "applied" && (
            <Callout
              tone="ok"
              title={null}
              className="mb-4"
              action={
                <Button
                  size="xs"
                  variant="outline"
                  disabled={action.busy}
                  onClick={() => action.run(async () => setStatusDone((await undoOperation(statusDone.id, true)).card))}
                >
                  <Undo2 /> Deshacer
                </Button>
              }
            >
              {statusDone.result}
            </Callout>
          )}
          {detail && detail.status !== "open" && detail.state === "active" && (
            <Callout tone="info" title={null} className="mb-4">
              Este {name} está {(words.knowledge.matterStates.status[detail.status] ?? detail.status).toLowerCase()}: no aparece en la lista ni en
              Atención, sigue en la búsqueda y no recibe propuestas nuevas.
            </Callout>
          )}
          {!matter.loading && !detail && !matter.error && (
            <EmptyNote>Este {name} ya no existe: sus documentos se agruparon de otra manera.</EmptyNote>
          )}
          {detail?.confirmation === "discarded" && (
            <Callout tone="quiet" title={null} className="mb-4">
              Descartaste este {name}
              {detail.origin === "person" ? "." : ": no vuelve mientras la app encuentre los mismos documentos."}
            </Callout>
          )}
          {detail?.state === "retired" && detail.confirmation !== "discarded" && (
            <Callout tone="quiet" title={null} className="mb-4">
              Ya no queda ningún grupo con estos documentos.
            </Callout>
          )}
        </>
      }
      article={
        detail ? (
          <MatterArticle matter={detail} article={article.data} model={model} loading={article.loading} onShowData={() => setTab("data")} />
        ) : null
      }
      articleAside={
        detail ? (
          <PageAside
            box={<MatterBox matter={detail} onShowData={() => setTab("data")} />}
            index={<PageIndex entries={index} jump={{ label: "Datos y correcciones", onClick: () => setTab("data") }} />}
          />
        ) : null
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
      dataAside={detail ? <PageAside box={<MatterDataBox matter={detail} />} /> : null}
    />
  )
}

type Run = (action: () => Promise<unknown>) => Promise<void>

/**
 * Closing, archiving or reopening a matter (D-475), by the operation of
 * the contract (D-498): the person decides, so it is done at once, kept in
 * the record of operations and undone from the notice.
 */
function StatusButtons({ matter, busy, onChange }: { matter: MatterDetail; busy: boolean; onChange: (estado: "abierto" | "cerrado" | "archivado") => void }) {
  if (matter.state !== "active" || matter.confirmation === "discarded") return null
  if (matter.status !== "open") {
    return (
      <Button variant="outline" disabled={busy} onClick={() => onChange("abierto")}>
        <LockOpen /> Reabrir
      </Button>
    )
  }
  return (
    <>
      <Button variant="outline" disabled={busy} onClick={() => onChange("cerrado")}>
        <Lock /> Cerrar
      </Button>
      <Button variant="outline" disabled={busy} onClick={() => onChange("archivado")}>
        <Archive /> Archivar
      </Button>
    </>
  )
}

/**
 * The article of the matter, as the projection last wrote it in the base
 * (the screen writes the name above it), and below it the research on
 * internet. A matter with too few documents, or made by hand and not yet
 * passed over, has none: the tab says so.
 */
function MatterArticle({
  matter,
  article,
  model,
  loading,
  onShowData,
}: {
  matter: MatterDetail
  article: KnowledgeArticle | null
  model: ArticleModel | null
  loading: boolean
  onShowData: () => void
}) {
  const words = vocabulary()
  if (loading && !article) return null
  const name = words.knowledge.matter.toLowerCase()
  return (
    <article>
      {article && model ? (
        <ArticleBody model={model} withRoles={false} onShowData={onShowData} writtenAt={article.writtenAt} />
      ) : (
        <NoArticle onShowData={onShowData}>
          Este {name} todavía no tiene artículo: la app lo escribe desde sus documentos en su próxima pasada, cuando tiene
          documentos suficientes.
        </NoArticle>
      )}
      <div className="mt-10">
        <ResearchSection key={`research-${matter.id}`} id="investigacion" objectId={matter.id} name={matter.name} />
      </div>
    </article>
  )
}

/**
 * The fact box of the article: who is in it, how many documents and who
 * grouped them, and what waits for a person's decision.
 */
function MatterBox({ matter, onShowData }: { matter: MatterDetail; onShowData: () => void }) {
  const words = vocabulary()
  const openEntity = useOpenEntity()
  const name = words.knowledge.matter.toLowerCase()
  const firm = matter.documents.filter((d) => d.state === "firm").length
  const proposed = matter.documents.filter((d) => d.state === "proposed").length
  const parties = [...matter.parties].sort((a, b) => b.documents - a.documents).slice(0, 4)
  const origin =
    matter.origin === "person"
      ? "lo creó una persona"
      : matter.confirmation === "confirmed"
        ? "lo agrupó la app; lo confirmó una persona"
        : "lo agrupó la app"
  return (
    <InfoBox
      kind={words.knowledge.matter}
      name={matter.name}
      badges={matter.confirmation !== "discarded" ? <MatterBadges confirmation={matter.confirmation} status={matter.status} words={words} /> : null}
    >
      {parties.length > 0 && (
        <InfoRows>
          <InfoRow label="Partes y personas">
            {parties.map((party) => (
              <span key={party.entityId} className="block">
                <button className="text-left text-link hover:underline" onClick={() => openEntity(party.entityId)}>
                  <LongName>{party.name}</LongName>
                </button>
                <span className="block text-xs text-muted-foreground">{rolesInWords(party.roles, words) || entityTypeName(party.kind, false, words)}</span>
              </span>
            ))}
          </InfoRow>
        </InfoRows>
      )}
      <InfoHeading>En el archivo</InfoHeading>
      <InfoRows>
        <InfoRow label="Documentos" below={proposed > 0 ? `y ${plural(proposed, "propuesto", "propuestos")}` : undefined}>
          <span className="tabular-nums">{firm}</span>
        </InfoRow>
        <InfoRow label="Origen">{origin}</InfoRow>
      </InfoRows>
      {(proposed > 0 || matter.confirmation === "proposed") && (
        <>
          <InfoHeading>Espera una decisión</InfoHeading>
          <InfoPending
            items={[
              ...(matter.confirmation === "proposed"
                ? [{ tone: "decision" as const, text: `El ${name} es una propuesta de la app.`, action: <TextLink onClick={onShowData}>Ver</TextLink> }]
                : []),
              ...(proposed > 0
                ? [
                    {
                      tone: "decision" as const,
                      text: `${plural(proposed, "documento propuesto", "documentos propuestos")} por el modelo.`,
                      action: <TextLink onClick={onShowData}>Ver</TextLink>,
                    },
                  ]
                : []),
            ]}
          />
        </>
      )}
    </InfoBox>
  )
}

/** The fact box of «Datos y correcciones»: its documents, what waits, and why they go together. */
function MatterDataBox({ matter }: { matter: MatterDetail }) {
  const words = vocabulary()
  const firm = matter.documents.filter((d) => d.state === "firm")
  const proposed = matter.documents.filter((d) => d.state === "proposed").length
  const decisions = matter.decisions.filter((d) => d.state === "active").length
  const kinds = new Map<string, number>()
  for (const doc of firm) {
    for (const kind of new Set(doc.reasons.map((reason) => reason.kind))) kinds.set(kind, (kinds.get(kind) ?? 0) + 1)
  }
  return (
    <InfoBox
      kind={words.knowledge.matter}
      name={matter.name}
      badges={matter.confirmation !== "discarded" ? <MatterBadges confirmation={matter.confirmation} status={matter.status} words={words} /> : null}
    >
      <InfoRows>
        <InfoRow label="Documentos">{plural(firm.length, "firme", "firmes")}</InfoRow>
        <InfoRow label="Propuestos" below={proposed > 0 ? "espera tu decisión" : undefined}>
          <span className="tabular-nums">{proposed}</span>
        </InfoRow>
        <InfoRow label="Decisiones" below={decisions > 0 ? "se pueden deshacer" : undefined}>
          <span className="tabular-nums">{decisions}</span>
        </InfoRow>
      </InfoRows>
      {kinds.size > 0 && (
        <>
          <InfoHeading>Por qué van juntos</InfoHeading>
          <InfoRows>
            {[...kinds.entries()].map(([kind, count]) => (
              <InfoRow key={kind} label={reasonKindInWords(kind, words)}>
                {plural(count, "documento", "documentos")}
              </InfoRow>
            ))}
          </InfoRows>
        </>
      )}
    </InfoBox>
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
  const [all, setAll] = useState(false)
  const FIRST = 6
  const shown = all || firm.length <= FIRST + 1 ? firm : firm.slice(0, FIRST)

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
    <li key={doc.document.contentId} className="border-t px-3.5 py-2.5 first:border-t-0">
      <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1.5 text-sm">
        <DocumentIcon />
        <button className="text-left font-semibold text-link hover:underline" onClick={() => openDocument(doc.document.locationId)}>
          {doc.document.name || `Documento ${doc.document.contentId}`}
        </button>
        <span className="ml-auto flex gap-1.5">
          {doc.state === "firm" ? (
            canTakeOut && (
              <Button size="xs" variant="ghost" className="text-muted-foreground hover:bg-err-bg hover:text-err" disabled={busy} onClick={() => void takeOut(doc)}>
                No es de este {name}
              </Button>
            )
          ) : (
            <>
              <Button size="xs" disabled={busy} onClick={() => void run(() => chooseMatterDocument(matter.id, `content:${doc.document.contentId}`, "in"))}>
                Confirmar
              </Button>
              <Button
                size="xs"
                variant="outline"
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
        <ul className="mt-1.5 ml-6 flex flex-col gap-0.5 text-[0.8125rem] text-muted-foreground">
          {doc.reasons.map((reason, index) => (
            <li key={index} className="before:text-border-strong before:content-['↳_']">
              {reasonInWords(reason, words)}
            </li>
          ))}
        </ul>
      )}
      {doc.proposalReason && <p className="mt-1.5 ml-6 text-sm text-muted-foreground">El modelo: «{doc.proposalReason}»</p>}
      {doc.uncertain && (
        <p className="mt-1.5 ml-6 text-sm text-amber-800 dark:text-amber-300" data-uncertain="">
          {words.included.uncertain}: {words.included.uncertainHelp}
        </p>
      )}
    </li>
  )

  return (
    <>
      <Section title="Documentos" count={firm.length} intro={`Debajo de cada uno, por qué está en este ${name}. Compartir una persona o una organización no alcanza.`}>
        {firm.length === 0 ? (
          <p className="text-sm text-muted-foreground">Todavía no tiene documentos: buscalos abajo y agregalos.</p>
        ) : (
          <ul className="rounded-[10px] border bg-card">
            {shown.map(row)}
            {shown.length < firm.length && (
              <li className="border-t px-3.5 py-2">
                <button
                  type="button"
                  onClick={() => setAll(true)}
                  className="font-ui text-[0.8125rem] font-bold text-link hover:underline"
                >
                  Ver los otros {plural(firm.length - shown.length, "documento", "documentos")}
                </button>
              </li>
            )}
          </ul>
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
        <Section
          title={`Posiblemente de este ${name}`}
          intro={`Ningún dato propio los une, pero el modelo cree que son de este ${name}. Hasta que lo confirmes es solo una propuesta.`}
        >
          <ul className="rounded-[10px] border-[1.5px] border-dashed border-border-strong">{proposed.map(row)}</ul>
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
      <Table>
        <tbody>
          {matter.parties.map((party) => (
            <tr key={party.entityId}>
              <Td width="30%">
                <button className="text-left text-link hover:underline" onClick={() => openEntity(party.entityId)}>
                  <LongName>{party.name}</LongName>
                </button>
              </Td>
              <Td muted>
                {entityTypeName(party.kind, false, words).toLowerCase()} · {rolesInWords(party.roles, words)}
              </Td>
              <Td muted align="right" nowrap>
                {plural(party.documents, "documento", "documentos")}
              </Td>
            </tr>
          ))}
        </tbody>
      </Table>
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
      <ul className="rounded-[10px] border bg-card text-sm">
        {matter.decisions.map((decision) => (
          <li key={decision.id} className="flex items-center gap-3 border-t px-3.5 py-2.5 first:border-t-0">
            <span className="min-w-0 flex-1">
              <span className={decision.state === "undone" ? "text-muted-foreground line-through" : ""}>
                {decision.kind === "join" ? `Del mismo ${matterWord}` : `De ${mattersWord} distintos`}: «{nameOf(decision.a)}» y «
                {nameOf(decision.b)}»
              </span>
              <span className="block text-[0.78125rem] text-muted-foreground">
                {formatDate(decision.decidedAt)}
                {decision.note && ` · ${decision.note}`}
              </span>
            </span>
            {decision.state === "active" && (
              <Button size="sm" variant="outline" disabled={busy} onClick={() => void run(() => undoIdentityDecision(decision.id))}>
                <Undo2 /> Deshacer
              </Button>
            )}
          </li>
        ))}
      </ul>
    </Section>
  )
}
