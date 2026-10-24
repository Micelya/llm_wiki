import { Fragment, useEffect, useState, type ReactNode } from "react"
import { Link2, PenLine, Undo2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { TextMark } from "@/components/ui/badge"
import { Callout } from "@/components/ui/callout"
import { Input } from "@/components/ui/input"
import { MoreButton, Table, Td, useFirstRows } from "@/components/ui/table"
import {
  decideIdentity,
  getEntity,
  getKnowledgeArticle,
  listEntities,
  undoIdentityDecision,
  type ClaimView,
  type EntityDetail,
  type EntitySummary,
  type KnowledgeArticle,
} from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"
import { entityTypeName, identifierName, predicateName, roleName, vocabulary } from "@/vocabulary"
import { EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { formatDate, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { ArticleBody, DocumentIcon, indexOf, useArticle, type ArticleModel } from "./article-page"
import { DetailTabs, DetailTitle, NoArticle, useDetailTab, type DetailTab } from "./detail-tabs"
import { articleBody, certaintyInWords, decisionInWords, documentsByMatter, resolvedByName, reviewInWords, rolesInWords } from "./knowledge"
import { InfoBox, InfoHeading, InfoPending, InfoRow, InfoRows, PageAside, PageIndex, TextLink } from "./page-aside"
import { ResearchSection } from "./research-section"
import { Quote, Section, useAction, useOpenDocument, useOpenMatter } from "./shared"

/** After an identifier that another entity has too, as a second number of one that names one only. */
const DOUBTFUL_IDENTIFIER = " · dudoso: otro también lo tiene"

/** How many data, documents and their mentions a list shows before «Ver…». */
const FIRST_CLAIMS = 6
const FIRST_DOCUMENTS = 4

/**
 * The one page of a person or organization (D-376, D-389), as a page of an
 * encyclopedia (D-493): the article on top (what the documents say of it,
 * with quotes and how sure each thing is, its documents by matter, who
 * else it is linked to), written in the base by the projection from the
 * record and the synthesis it keeps, without asking the model, with its
 * research on internet, and at its right a fact box with its identifiers
 * and data, its matters and what waits for a decision, and the index of
 * the page; in a tab its data and corrections: how the documents write it,
 * its identifiers, every claim and mention with what a person can do about
 * it, and the decisions, to undo (R-08, R-09). The notes of a person at
 * the foot of both (D-407, D-423). Never an edit of the article.
 */
export function EntityDetailView({ id, onBack, initialTab = null }: { id: string; onBack: () => void; initialTab?: DetailTab | null }) {
  const words = vocabulary()
  const entity = useDocbaseQuery(`entity:${id}`, () => getEntity(id))
  const article = useDocbaseQuery(`article:${id}`, () => getKnowledgeArticle(id))
  const model = useArticle(article.data ? articleBody(article.data.body) : null)
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  const action = useAction()
  const [joining, setJoining] = useState(false)
  const [naming, setNaming] = useState(false)
  const detail = entity.data
  // Without a page (only named in passing) there is no article to read.
  const [shown, setTab] = useDetailTab(!article.loading && !article.data, initialTab)
  const pending = detail ? conflictsOf(detail).length + detail.doubts.length : 0
  const index = indexOf(model, [
    ...(detail?.state === "active" ? [{ id: "investigacion", label: "Investigación en internet" }] : []),
    { id: "notas", label: "Notas" },
  ])

  return (
    <DetailTabs
      back={words.knowledge.entities}
      onBack={onBack}
      current={detail?.name ?? "…"}
      kicker={detail ? entityTypeName(detail.kind, false, words) : undefined}
      title={<DetailTitle>{detail?.name ?? "…"}</DetailTitle>}
      facts={detail ? <OtherNames entity={detail} /> : null}
      actions={
        detail && detail.state === "active" && shown === "data" ? (
          <>
            <Button variant="secondary" onClick={() => setNaming(!naming)}>
              <PenLine /> Elegir el nombre
            </Button>
            <Button variant="secondary" onClick={() => setJoining(!joining)}>
              <Link2 /> Unir con otra
            </Button>
          </>
        ) : null
      }
      tab={shown}
      onTab={setTab}
      pending={pending}
      notesOf={detail?.id ?? null}
      notice={
        <>
          {(entity.error || action.problem) && (
            <div className="mb-4">
              <ErrorNote>{entity.error ?? action.problem}</ErrorNote>
            </div>
          )}
          {!entity.loading && !detail && !entity.error && <EmptyNote>Esta entidad ya no está en el registro.</EmptyNote>}
          {detail && detail.state !== "active" && (
            <Callout tone="quiet" title={null} className="mb-6">
              {detail.state === "merged" && detail.mergedInto ? (
                <>
                  Se unió con otra entidad. <TextLink onClick={() => openEntity(detail.mergedInto)}>Ver la entidad que quedó</TextLink>
                </>
              ) : (
                "Ya no la nombra ningún documento que cuente."
              )}
            </Callout>
          )}
        </>
      }
      article={
        detail ? <Article entity={detail} article={article.data} model={model} loading={article.loading} onShowData={() => setTab("data")} /> : null
      }
      articleAside={
        detail ? (
          <PageAside
            box={<EntityBox entity={detail} onShowData={() => setTab("data")} />}
            index={<PageIndex entries={index} jump={{ label: "Datos y correcciones", onClick: () => setTab("data") }} />}
          />
        ) : null
      }
      data={
        detail ? (
          <>
            {naming && <NamePicker entity={detail} onDone={() => setNaming(false)} run={action.run} />}
            {joining && <JoinPicker entity={detail} onDone={() => setJoining(false)} run={action.run} />}
            <Claims entity={detail} run={action.run} busy={action.busy} />
            <Documents entity={detail} run={action.run} busy={action.busy} />
            <Incoming entity={detail} />
            <Decisions entity={detail} run={action.run} busy={action.busy} />
          </>
        ) : null
      }
      dataAside={detail ? <PageAside box={<FactBox entity={detail} />} /> : null}
    />
  )
}

/** Its claims in which the documents differ. */
function conflictsOf(entity: EntityDetail): ClaimView[] {
  return entity.claims.filter((claim) => claim.state === "conflict")
}

function sameName(a: string, b: string): boolean {
  return a.toLocaleLowerCase("es").replace(/\s+/g, " ").trim() === b.toLocaleLowerCase("es").replace(/\s+/g, " ").trim()
}

/** «También escrito como A · B · C», under the name: the other ways the documents write it. */
function OtherNames({ entity }: { entity: EntityDetail }) {
  const others = entity.aliases.filter((alias) => !sameName(alias.written, entity.name))
  if (others.length === 0) return null
  const shown = others.slice(0, 4)
  return (
    <>
      También escrito como{" "}
      {shown.map((alias, index) => (
        <Fragment key={alias.written}>
          {index > 0 && <span className="mx-1.5 text-border-strong">·</span>}
          <b>{alias.written}</b>
        </Fragment>
      ))}
      {others.length > shown.length && <span className="ml-1.5">y {plural(others.length - shown.length, "forma más", "formas más")}</span>}
    </>
  )
}

/**
 * The article, as the projection last wrote it in the base (the screen
 * writes the name above it), and below it the research on internet; the
 * notes of a person go at the foot of both tabs.
 */
function Article({
  entity,
  article,
  model,
  loading,
  onShowData,
}: {
  entity: EntityDetail
  article: KnowledgeArticle | null
  model: ArticleModel | null
  loading: boolean
  onShowData: () => void
}) {
  if (loading && !article) return null
  return (
    <article>
      {article && model ? (
        <ArticleBody model={model} withRoles onShowData={onShowData} writtenAt={article.writtenAt} />
      ) : (
        <NoArticle onShowData={onShowData}>
          {entity.pagePath
            ? "Su artículo todavía no está escrito: la app lo escribe en su próxima pasada."
            : "Los documentos la nombran de paso: no tiene artículo."}
        </NoArticle>
      )}
      {entity.state === "active" && (
        <div className="mt-10">
          <ResearchSection key={entity.id} id="investigacion" objectId={entity.id} name={entity.name} />
        </div>
      )}
    </article>
  )
}

/**
 * The fact box of the article: its identifiers and the data the documents
 * give of it (one per kind, the first), its matters and documents, and
 * what waits for a person's decision, with the way to it.
 */
function EntityBox({ entity, onShowData }: { entity: EntityDetail; onShowData: () => void }) {
  const words = vocabulary()
  const openMatter = useOpenMatter()
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const focusDoubt = useKnowledgeStore((s) => s.focusDoubt)
  const live = entity.claims.filter((claim) => claim.state !== "retired" && claim.objectEntity === null && claim.objectValue)
  const byPredicate = new Map<string, ClaimView[]>()
  for (const claim of live) byPredicate.set(claim.predicate, [...(byPredicate.get(claim.predicate) ?? []), claim])
  const values = [...byPredicate.entries()].slice(0, 6)
  const matters = [...new Map(entity.matters.map((m) => [m.matterId, m])).values()]
  const documents = new Set(entity.mentions.map((m) => m.contentId)).size
  const conflicts = new Set(conflictsOf(entity).map((claim) => claim.predicate)).size
  return (
    <InfoBox kind={entityTypeName(entity.kind, false, words)} name={entity.name}>
      {(entity.identifiers.length > 0 || values.length > 0) && (
        <InfoRows>
          {entity.identifiers.map((identifier) => (
            <InfoRow
              key={`${identifier.kind}:${identifier.value}`}
              label={identifierName(identifier.kind, words)}
              below={`en ${plural(identifier.documents, "documento", "documentos")}${identifier.doubtful ? DOUBTFUL_IDENTIFIER : ""}`}
            >
              <span className="tabular-nums">{identifier.value}</span>
            </InfoRow>
          ))}
          {values.map(([predicate, claims]) => {
            const first = claims[0]
            const said = certaintyInWords(first)
            return (
              <InfoRow key={predicate} label={predicateName(predicate, false, words)}>
                {first.objectValue}
                {claims.some((claim) => claim.state === "conflict") ? (
                  <TextMark tone="differ">difieren</TextMark>
                ) : said.tone === "doubt" ? (
                  <TextMark tone="doubt" title={said.why || undefined}>
                    {said.label}
                  </TextMark>
                ) : null}
              </InfoRow>
            )
          })}
        </InfoRows>
      )}
      <InfoHeading>En el archivo</InfoHeading>
      <InfoRows>
        {matters.length > 0 && (
          <InfoRow label={words.knowledge.matters}>
            {matters.map((matter) => (
              <button key={matter.matterId} className="block text-left text-link hover:underline" onClick={() => openMatter(matter.matterId)}>
                {matter.name}
                {matter.proposed && <span className="ml-1 text-xs text-muted-foreground">(propuesto)</span>}
              </button>
            ))}
          </InfoRow>
        )}
        <InfoRow label="Documentos">
          <span className="tabular-nums">{documents}</span>
        </InfoRow>
      </InfoRows>
      {(conflicts > 0 || entity.doubts.length > 0) && (
        <>
          <InfoHeading>Espera una decisión</InfoHeading>
          <InfoPending
            items={[
              ...(conflicts > 0
                ? [
                    {
                      tone: "warn" as const,
                      text: `${plural(conflicts, "dato", "datos")} en ${conflicts === 1 ? "el" : "los"} que los documentos difieren.`,
                      action: <TextLink onClick={onShowData}>Ver</TextLink>,
                    },
                  ]
                : []),
              ...(entity.doubts.length > 0
                ? [
                    {
                      tone: "decision" as const,
                      text: `${plural(entity.doubts.length, "posible igual", "posibles iguales")}.`,
                      action: (
                        <TextLink
                          onClick={() => {
                            focusDoubt(entity.id)
                            setActiveView("identity-doubts")
                          }}
                        >
                          Resolver
                        </TextLink>
                      ),
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

type Run = (action: () => Promise<unknown>) => Promise<void>

/**
 * What each document says of the entity, the data in which they differ
 * first: each datum in a fixed row (what it is, its value, how sure, the
 * actions always in the same place) and its quotes below (D-493).
 */
function Claims({ entity, run, busy }: { entity: EntityDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  const openDocument = useOpenDocument()
  const live = entity.claims.filter((claim) => claim.state !== "retired")
  const conflicts = live.filter((claim) => claim.state === "conflict")
  const supported = live.filter((claim) => claim.state !== "conflict")
  const first = useFirstRows(supported, FIRST_CLAIMS)

  async function markWrong(reference: string, what: string) {
    const confirmed = await dialog.confirm({
      title: "Marcar como incorrecto",
      message: `${what} deja de contar para esta entidad y sus páginas. Queda guardado como decisión tuya y se puede deshacer.`,
      confirmLabel: "Marcar",
      cancelLabel: "Cancelar",
    })
    if (confirmed) await run(() => decideIdentity({ kind: "wrong", a: reference }, null, JSON.stringify({ entity: entity.name, what })))
  }

  async function confirm(claim: ClaimView, what: string) {
    const confirmed = await dialog.confirm({
      title: "Confirmar el dato",
      message: `${what}: leíste el documento y el dato es correcto. Las páginas lo dicen como confirmado por una persona. Queda guardado como decisión tuya y se puede deshacer.`,
      confirmLabel: "Confirmar",
      cancelLabel: "Cancelar",
    })
    if (confirmed) await run(() => decideIdentity({ kind: "confirm", a: `claim:${claim.id}` }, null, JSON.stringify({ entity: entity.name, what })))
  }

  const value = (claim: ClaimView): ReactNode =>
    claim.objectEntity ? (
      <>
        <button className="text-link underline decoration-[var(--link-line)] underline-offset-[3px] hover:decoration-link" onClick={() => openEntity(claim.objectEntity)}>
          {claim.objectName ?? claim.objectEntity}
        </button>
        {/* An amount between two parties: the other party and the amount. */}
        {claim.objectValue ? <span> · {claim.objectValue}</span> : null}
      </>
    ) : (
      <span>{claim.objectValue}</span>
    )

  const item = (claim: ClaimView, warn: boolean) => {
    const what = `«${predicateName(claim.predicate, false, words)}: ${claim.objectName ?? claim.objectValue ?? ""}»`
    return (
      <li key={claim.id} className={`border-t px-3.5 py-3 first:border-t-0 ${warn ? "border-warn-line" : ""}`}>
        <div className="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-3 gap-y-1 md:grid-cols-[8.75rem_minmax(0,1fr)_auto]">
          <span className="col-span-2 font-ui text-[0.8125rem] font-semibold text-muted-foreground md:col-span-1">
            {predicateName(claim.predicate, false, words)}
          </span>
          <span className="flex min-w-0 flex-wrap items-baseline gap-x-2.5 gap-y-1">
            <span className="text-[0.90625rem] font-semibold text-strong">{value(claim)}</span>
            {claim.validFrom && <span className="text-[0.8125rem] text-muted-foreground">desde {claim.validFrom}</span>}
            {warn && <TextMark tone="differ">los documentos difieren</TextMark>}
            {/* Doubtful only because the documents differ: the mark above says it already. */}
            {!(warn && claim.certainty === "doubtful" && claim.reasons.every((reason) => reason === "conflict")) && <Certainty claim={claim} />}
          </span>
          <span className="flex gap-0.5 whitespace-nowrap">
            {claim.certainty !== "confirmed" && (
              <Button size="xs" variant="ghost" className="text-link" disabled={busy} onClick={() => void confirm(claim, what)}>
                Confirmar
              </Button>
            )}
            <Button
              size="xs"
              variant="ghost"
              className="text-muted-foreground hover:bg-err-bg hover:text-err"
              disabled={busy}
              onClick={() => void markWrong(`claim:${claim.id}`, what)}
            >
              Marcar como incorrecto
            </Button>
          </span>
        </div>
        <ul className="mt-1.5 flex flex-col gap-1 md:ml-[9.5rem]">
          {claim.evidence.map((evidence) => (
            <li key={evidence.evidenceId} className="text-[0.84375rem] leading-normal text-muted-foreground">
              <Quote>{evidence.quote}</Quote> —{" "}
              <button className="text-link hover:underline" onClick={() => openDocument(locationOf(entity, evidence.contentId))}>
                {evidence.document}
              </button>
              {evidence.page !== null && `, p. ${evidence.page}`}
              {evidence.factTime && ` · ${evidence.factTime}`}
              {claim.evidence.length > 1 && (
                <button
                  className="ml-1.5 font-ui text-[0.78125rem] font-semibold text-muted-foreground hover:text-err"
                  disabled={busy}
                  onClick={() => void markWrong(`evidence:${evidence.evidenceId}`, `Lo que dice «${evidence.document}»`)}
                >
                  no lo dice
                </button>
              )}
            </li>
          ))}
        </ul>
      </li>
    )
  }

  const hiddenNames = [...new Set(supported.slice(FIRST_CLAIMS).map((claim) => predicateName(claim.predicate, false, words).toLowerCase()))]
  return (
    <>
      {conflicts.length > 0 && (
        <Section title="Datos en los que los documentos difieren" intro="Dos documentos dicen cosas distintas de lo mismo. La app no elige: si uno está mal, marcalo.">
          <ul className="rounded-[10px] border border-warn-line bg-warn-bg">{conflicts.map((claim) => item(claim, true))}</ul>
        </Section>
      )}
      <Section title="Lo que dicen los documentos">
        {supported.length === 0 ? (
          <p className="text-sm text-muted-foreground">Los documentos no dicen de esta entidad nada más que su nombre y su rol.</p>
        ) : (
          <>
            <ul className="rounded-[10px] border bg-card">{first.shown.map((claim) => item(claim, false))}</ul>
            {first.hidden > 0 && (
              <MoreButton className="mt-2.5" onClick={first.showAll}>
                Ver {plural(first.hidden, "dato más", "datos más")}
                {hiddenNames.length > 0 && ` (${hiddenNames.slice(0, 4).join(", ")}${hiddenNames.length > 4 ? "…" : ""})`}
              </MoreButton>
            )}
          </>
        )}
      </Section>
    </>
  )
}

/** How sure the record is of a claim: doubtful in amber, with why; a person's word named; nothing for a plain quote. */
function Certainty({ claim }: { claim: ClaimView }) {
  const said = certaintyInWords(claim)
  if (said.tone === "plain") return null
  if (said.tone === "person") return <TextMark tone="person">{said.label}</TextMark>
  return (
    <TextMark tone="doubt" title={said.why || undefined}>
      {said.label}
      {said.why && `: ${said.why}`}
    </TextMark>
  )
}

function locationOf(entity: EntityDetail, contentId: number): number | null {
  return entity.mentions.find((m) => m.contentId === contentId)?.locationId ?? null
}

/** Its documents, by matter, with its role and how each one writes it. */
function Documents({ entity, run, busy }: { entity: EntityDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const groups = documentsByMatter(entity.mentions, entity.matters, words.knowledge.withoutMatter)
  const documents = new Set(entity.mentions.map((m) => m.contentId)).size
  return (
    <Section title="Documentos" count={documents}>
      <div className="flex flex-col gap-[18px]">
        {groups.map((group) => (
          <DocumentGroupList key={group.matterId ?? "none"} entity={entity} group={group} run={run} busy={busy} />
        ))}
      </div>
    </Section>
  )
}

function DocumentGroupList({
  entity,
  group,
  run,
  busy,
}: {
  entity: EntityDetail
  group: ReturnType<typeof documentsByMatter>[number]
  run: Run
  busy: boolean
}) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const openDocument = useOpenDocument()
  const openMatter = useOpenMatter()
  const first = useFirstRows(group.documents, FIRST_DOCUMENTS, 1)

  async function separate(mentionId: number, written: string, document: string) {
    const confirmed = await dialog.confirm({
      title: "Separar la mención",
      message:
        `«${written}» en «${document}» deja de ser ${entity.name}: queda como otra entidad, y ninguna pasada la vuelve a unir. ` +
        "Se puede deshacer.",
      confirmLabel: "Separar",
      cancelLabel: "Cancelar",
    })
    if (confirmed) {
      await run(() =>
        decideIdentity(
          { kind: "separate", a: `mention:${mentionId}`, b: `entity:${entity.id}` },
          null,
          JSON.stringify({ entity: entity.name, mention: written, document }),
        ),
      )
    }
  }

  async function notAnEntity(mentionId: number, written: string, document: string) {
    const confirmed = await dialog.confirm({
      title: "No es una entidad",
      message: `«${written}» en «${document}» no nombra a nadie: deja de contar. Se puede deshacer.`,
      confirmLabel: "Confirmar",
      cancelLabel: "Cancelar",
    })
    if (confirmed) {
      await run(() => decideIdentity({ kind: "wrong", a: `mention:${mentionId}` }, null, JSON.stringify({ mention: written, document })))
    }
  }

  return (
    <div>
      <h3 className="mb-2 flex flex-wrap items-baseline gap-x-2 font-ui text-[0.9375rem] font-bold text-strong">
        {group.matterId ? (
          <button className="text-left text-link underline decoration-[var(--link-line)] underline-offset-[3px] hover:decoration-link" onClick={() => openMatter(group.matterId as string)}>
            {group.name}
          </button>
        ) : (
          group.name
        )}
        <span className="font-sans text-[0.8125rem] font-normal text-muted-foreground">{plural(group.documents.length, "documento", "documentos")}</span>
      </h3>
      <ul className="rounded-[10px] border bg-card">
        {first.shown.map((doc) => (
          <li key={`${group.matterId}-${doc.contentId}`} className="border-t px-3.5 py-2.5 first:border-t-0">
            <div className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1.5 text-sm">
              <DocumentIcon />
              <button className="text-left font-semibold text-link hover:underline" onClick={() => openDocument(doc.locationId)}>
                {doc.name || `Documento ${doc.contentId}`}
              </button>
              <span className="text-[0.8125rem] text-muted-foreground">{rolesInWords(doc.roles, words)}</span>
              {doc.proposed && <StatusBadge tone="neutral">propuesto por el modelo</StatusBadge>}
            </div>
            {!doc.proposed && (
              <ul className="mt-1.5 ml-6 flex flex-col gap-[3px]">
                {doc.mentions.map((mention) => (
                  <li key={mention.mentionId} className="flex flex-wrap items-baseline gap-2 text-[0.8125rem] text-muted-foreground">
                    <span className="text-foreground">«{mention.written}»</span>
                    <span>
                      {roleName(mention.role, words)}
                      {mention.page !== null && ` · p. ${mention.page}`}
                      {resolvedByName(mention.resolvedBy) && ` · ${resolvedByName(mention.resolvedBy)}`}
                    </span>
                    <span className="ml-auto flex gap-0.5">
                      <Button size="xs" variant="ghost" className="text-link" disabled={busy} onClick={() => void separate(mention.mentionId, mention.written, doc.name)}>
                        Separar
                      </Button>
                      <Button
                        size="xs"
                        variant="ghost"
                        className="text-muted-foreground hover:bg-err-bg hover:text-err"
                        disabled={busy}
                        onClick={() => void notAnEntity(mention.mentionId, mention.written, doc.name)}
                      >
                        No es una entidad
                      </Button>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </li>
        ))}
        {first.hidden > 0 && (
          <li className="border-t px-3.5 py-2">
            <MoreButton onClick={first.showAll}>
              Ver los otros {plural(first.hidden, "documento", "documentos")}
              {group.matterId ? ` del ${words.knowledge.matter.toLowerCase()}` : ""}
            </MoreButton>
          </li>
        )}
      </ul>
    </div>
  )
}

/** What other entities' claims say of this one ("represented by"). */
function Incoming({ entity }: { entity: EntityDetail }) {
  const words = vocabulary()
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  if (entity.incoming.length === 0) return null
  return (
    <Section title="Vínculos desde otras entidades">
      <Table>
        <tbody>
          {entity.incoming.map((claim) => (
            <tr key={claim.claimId}>
              <Td muted width="34%">
                {predicateName(claim.predicate, true, words)}
              </Td>
              <Td>
                <button className="text-left text-link hover:underline" onClick={() => openEntity(claim.subjectId)}>
                  {claim.subjectName}
                </button>
              </Td>
              <Td muted align="right" nowrap>
                {entityTypeName(claim.subjectKind, false, words).toLowerCase()} · {plural(claim.documents, "documento", "documentos")}
              </Td>
            </tr>
          ))}
        </tbody>
      </Table>
    </Section>
  )
}

/** What a person decided about it, to undo. */
function Decisions({ entity, run, busy }: { entity: EntityDetail; run: Run; busy: boolean }) {
  if (entity.decisions.length === 0) return null
  return (
    <Section title="Decisiones">
      <ul className="rounded-[10px] border bg-card text-sm">
        {entity.decisions.map((decision) => (
          <li key={decision.id} className="flex items-center gap-3 border-t px-3.5 py-2.5 first:border-t-0">
            <span className="min-w-0 flex-1">
              <span className={decision.state === "undone" ? "text-muted-foreground line-through" : ""}>{decisionInWords(decision, entity)}</span>
              <span className="block text-[0.78125rem] text-muted-foreground">
                {formatDate(decision.decidedAt)}
                {decision.note && ` · ${decision.note}`}
                {decision.state === "undone" && decision.undoneAt && ` · deshecha el ${formatDate(decision.undoneAt)}`}
              </span>
              {decision.state === "active" && decision.review && <span className="block text-xs text-warn">{reviewInWords(decision.review)}</span>}
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

/** The fact box of «Datos y correcciones»: how the documents write it, its identifiers, its doubts. */
function FactBox({ entity }: { entity: EntityDetail }) {
  const words = vocabulary()
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const focusDoubt = useKnowledgeStore((s) => s.focusDoubt)
  return (
    <InfoBox kind={entityTypeName(entity.kind, false, words)} name={entity.name}>
      <InfoHeading>Se escribe</InfoHeading>
      <ul className="px-4 pt-1 pb-2">
        {entity.aliases.map((alias) => (
          <li key={alias.written} className="flex justify-between gap-2 py-1">
            <span className="break-words">{alias.written}</span>
            <span className="shrink-0 text-muted-foreground tabular-nums">{alias.mentions}</span>
          </li>
        ))}
      </ul>
      {entity.identifiers.length > 0 && (
        <>
          <InfoHeading>Identificadores</InfoHeading>
          <InfoRows>
            {entity.identifiers.map((identifier) => (
              <InfoRow
                key={`${identifier.kind}:${identifier.value}`}
                label={identifierName(identifier.kind, words)}
                below={`${plural(identifier.documents, "documento", "documentos")}${identifier.doubtful ? DOUBTFUL_IDENTIFIER : ""}`}
              >
                <span className="tabular-nums">{identifier.value}</span>
              </InfoRow>
            ))}
          </InfoRows>
        </>
      )}
      {entity.doubts.length > 0 && (
        <>
          <InfoHeading>Dudas</InfoHeading>
          <InfoPending items={[{ tone: "decision", text: `${plural(entity.doubts.length, "posible igual espera", "posibles iguales esperan")} una decisión.` }]} />
          <div className="px-4 pb-3.5">
            <Button
              size="sm"
              onClick={() => {
                focusDoubt(entity.id)
                setActiveView("identity-doubts")
              }}
            >
              Resolver
            </Button>
          </div>
        </>
      )}
    </InfoBox>
  )
}

/**
 * Chooses how it is named (D-516): one of the ways its documents write it,
 * or one written here. It stands over what the documents say, the app does
 * not change it, and it is undone from «Decisiones».
 */
function NamePicker({ entity, onDone, run }: { entity: EntityDetail; onDone: () => void; run: Run }) {
  const [typed, setTyped] = useState("")
  const others = entity.aliases.filter((alias) => !sameName(alias.written, entity.name))

  async function choose(name: string) {
    const chosen = name.trim()
    if (!chosen) return
    await run(() => decideIdentity({ kind: "name", a: `entity:${entity.id}`, name: chosen }, null, JSON.stringify({ name: entity.name, chosen })))
    onDone()
  }

  return (
    <div className="mb-8 rounded-xl border bg-card px-[18px] py-4 shadow-card">
      <p className="text-sm">
        Hoy se llama «{entity.name}». Elegí cómo se llama: queda así aunque los documentos digan otra cosa, hasta que lo deshagas en «Decisiones».
      </p>
      {others.length > 0 && (
        <ul className="mt-3 rounded-[10px] border text-sm">
          {others.map((alias) => (
            <li key={alias.written} className="flex items-center gap-3 border-t px-3 py-2 first:border-t-0">
              <span className="min-w-0 flex-1 break-words">
                {alias.written}
                <span className="ml-2 text-xs text-muted-foreground">{plural(alias.mentions, "mención", "menciones")}</span>
              </span>
              <Button size="sm" variant="secondary" onClick={() => void choose(alias.written)}>
                Usar
              </Button>
            </li>
          ))}
        </ul>
      )}
      <form
        className="mt-3 flex items-center gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          void choose(typed)
        }}
      >
        <Input className="h-9 w-80" placeholder="Otro nombre" value={typed} onChange={(event) => setTyped(event.target.value)} />
        <Button size="sm" type="submit" disabled={!typed.trim()}>
          Usar este nombre
        </Button>
      </form>
    </div>
  )
}

/** Looks for another entity of the same type to join with this one. */
function JoinPicker({ entity, onDone, run }: { entity: EntityDetail; onDone: () => void; run: Run }) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const [typed, setTyped] = useState("")
  const [found, setFound] = useState<EntitySummary[]>([])
  useEffect(() => {
    const query = typed.trim()
    if (query.length < 2) {
      setFound([])
      return
    }
    let stale = false
    const timer = setTimeout(() => {
      listEntities(entity.kind, query)
        .then((list) => !stale && setFound(list.filter((other) => other.id !== entity.id).slice(0, 12)))
        .catch(() => !stale && setFound([]))
    }, 250)
    return () => {
      stale = true
      clearTimeout(timer)
    }
  }, [typed, entity.id, entity.kind])

  async function join(other: EntitySummary) {
    const confirmed = await dialog.confirm({
      title: "Unir",
      message:
        `${entity.name} y ${other.name} pasan a ser una sola entidad, con todos sus documentos y lo que dicen. ` +
        "Ninguna pasada las vuelve a separar. Se puede deshacer.",
      confirmLabel: "Unir",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    await run(() =>
      decideIdentity(
        { kind: "join", a: `entity:${entity.id}`, b: `entity:${other.id}` },
        null,
        JSON.stringify({ a: { name: entity.name, aliases: entity.aliases.map((a) => a.written) }, b: { name: other.name, aliases: other.aliases } }),
      ),
    )
    onDone()
  }

  return (
    <div className="mb-8 rounded-xl border bg-card px-[18px] py-4 shadow-card">
      <p className="text-sm">¿Con quién es la misma entidad? Buscá entre {entityTypeName(entity.kind, true, words).toLowerCase()} por nombre.</p>
      <Input className="mt-2 h-9 w-80" autoFocus value={typed} onChange={(event) => setTyped(event.target.value)} />
      {found.length > 0 && (
        <ul className="mt-3 rounded-[10px] border text-sm">
          {found.map((other) => (
            <li key={other.id} className="flex items-center gap-3 border-t px-3 py-2 first:border-t-0">
              <span className="min-w-0 flex-1 break-words">
                {other.name}
                <span className="ml-2 text-xs text-muted-foreground">{plural(other.documents, "documento", "documentos")}</span>
              </span>
              <Button size="sm" variant="secondary" onClick={() => void join(other)}>
                Unir
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
