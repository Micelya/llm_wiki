import { useEffect, useState, type ReactNode } from "react"
import { ArrowLeft, Link2, NotebookPen, Undo2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  decideIdentity,
  getEntity,
  getEntityArticle,
  listEntities,
  undoIdentityDecision,
  type ClaimView,
  type EntityArticle,
  type EntityDetail,
  type EntitySummary,
} from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"
import { entityTypeName, identifierName, predicateName, roleName, vocabulary } from "@/vocabulary"
import { EmptyNote, ErrorNote, StatusBadge } from "@/components/docbase/page"
import { formatDate, plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { WikiReader } from "@/components/editor/wiki-reader"
import { articleBody, certaintyInWords, decisionInWords, documentsByMatter, resolvedByName, reviewInWords, rolesInWords } from "./knowledge"
import { Quote, Section, useAction, useArchivePage, useOpenDocument, useOpenMatter } from "./shared"

/**
 * The one page of a person or organization (D-376, D-389): the article
 * on top, as its page of the wiki reads (what the documents say of it,
 * with quotes and how sure each thing is, its documents by matter, who
 * else it is linked to), written by the projection from the record and
 * the synthesis it keeps, without asking the model; and in a tab its data
 * and corrections: how the documents write it, its identifiers, every
 * claim and mention with what a person can do about it, and the
 * decisions, to undo (R-08, R-09). Never an edit of the page.
 */
export function EntityDetailView({ id, onBack }: { id: string; onBack: () => void }) {
  const words = vocabulary()
  const entity = useDocbaseQuery(`entity:${id}`, () => getEntity(id))
  const article = useDocbaseQuery(`entity-article:${id}`, () => getEntityArticle(id))
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  const archivePage = useArchivePage()
  const action = useAction()
  const [joining, setJoining] = useState(false)
  const [tab, setTab] = useState<"article" | "data" | null>(null)
  const detail = entity.data
  // Without a page (only named in passing) there is no article to read.
  const shown = tab ?? (!article.loading && !article.data ? "data" : "article")
  const pending = detail ? detail.claims.filter((c) => c.state === "conflict").length + detail.doubts.length : 0

  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b px-8 pt-4">
        <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground" onClick={onBack}>
          <ArrowLeft className="size-4" /> {words.knowledge.entities}
        </button>
        <div className="mt-2 flex items-end justify-between gap-4 pb-2">
          <div className="min-w-0">
            <h1 className="font-serif text-2xl break-words">{detail?.name ?? "…"}</h1>
            {detail && <p className="text-sm text-muted-foreground">{entityTypeName(detail.kind, false, words)}</p>}
          </div>
          {detail && detail.state === "active" && (
            <div className="flex shrink-0 gap-2">
              {shown === "article" && detail.pagePath && (
                <Button
                  variant="outline"
                  disabled={!archivePage}
                  title={archivePage ? detail.pagePath : `Se abre con el ${words.archive.wiki.toLowerCase()} abierto`}
                  onClick={() => archivePage?.(detail.pagePath as string)}
                >
                  <NotebookPen /> Escribir notas
                </Button>
              )}
              {shown === "data" && (
                <Button variant="outline" onClick={() => setJoining(!joining)}>
                  <Link2 /> Unir con otra
                </Button>
              )}
            </div>
          )}
        </div>
        <nav className="-mb-px flex gap-1 text-sm">
          <Tab active={shown === "article"} onClick={() => setTab("article")}>
            Artículo
          </Tab>
          <Tab active={shown === "data"} onClick={() => setTab("data")}>
            Datos y correcciones
            {pending > 0 && (
              <span className="ml-1.5 rounded-full bg-amber-500/20 px-1.5 text-xs text-amber-700 dark:text-amber-400">{pending}</span>
            )}
          </Tab>
        </nav>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-6">
        {(entity.error || action.problem) && (
          <div className="mb-4">
            <ErrorNote>{entity.error ?? action.problem}</ErrorNote>
          </div>
        )}
        {!entity.loading && !detail && !entity.error && <EmptyNote>Esta entidad ya no está en el registro.</EmptyNote>}
        {detail && detail.state !== "active" && (
          <div className="mx-auto mb-6 max-w-6xl rounded-md border border-dashed px-4 py-3 text-sm">
            {detail.state === "merged" && detail.mergedInto ? (
              <>
                Se unió con otra entidad.{" "}
                <button className="text-primary hover:underline" onClick={() => openEntity(detail.mergedInto)}>
                  Ver la entidad que quedó
                </button>
              </>
            ) : (
              "Ya no la nombra ningún documento que cuente."
            )}
          </div>
        )}
        {detail && shown === "article" && <Article entity={detail} article={article.data} loading={article.loading} />}
        {detail && shown === "data" && joining && <JoinPicker entity={detail} onDone={() => setJoining(false)} run={action.run} />}
        {detail && shown === "data" && (
          <div className="mx-auto grid max-w-6xl gap-8 lg:grid-cols-[minmax(0,1fr)_20rem]">
            <main className="min-w-0">
              <Claims entity={detail} run={action.run} busy={action.busy} />
              <Documents entity={detail} run={action.run} busy={action.busy} />
              <Incoming entity={detail} />
              <Decisions entity={detail} run={action.run} busy={action.busy} />
            </main>
            <aside>
              <FactBox entity={detail} />
            </aside>
          </div>
        )}
      </div>
    </div>
  )
}

function Tab({ active, onClick, children }: { active: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      className={`flex items-center border-b-2 px-3 py-1.5 ${
        active ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"
      }`}
      onClick={onClick}
    >
      {children}
    </button>
  )
}

/**
 * The article: its page of the wiki as the projection wrote it, with the
 * notes of a person; the title is the page's own heading.
 */
function Article({ entity, article, loading }: { entity: EntityDetail; article: EntityArticle | null; loading: boolean }) {
  const words = vocabulary()
  if (loading && !article) return null
  if (!article) {
    return (
      <EmptyNote>
        {entity.pagePath
          ? `Su página todavía no está escrita: la escribe la próxima pasada del ${words.archive.wiki.toLowerCase()}.`
          : "Los documentos la nombran de paso: no tiene artículo. Lo que se sabe de ella está en «Datos y correcciones»."}
      </EmptyNote>
    )
  }
  return (
    <article className="mx-auto max-w-4xl">
      <WikiReader body={articleBody(article.text)} filePath={article.path} />
    </article>
  )
}

type Run = (action: () => Promise<unknown>) => Promise<void>

/** What each document says of the entity, the data in which they differ first. */
function Claims({ entity, run, busy }: { entity: EntityDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  const openDocument = useOpenDocument()
  const live = entity.claims.filter((claim) => claim.state !== "retired")
  const conflicts = live.filter((claim) => claim.state === "conflict")
  const supported = live.filter((claim) => claim.state !== "conflict")

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
        <button className="text-primary hover:underline" onClick={() => openEntity(claim.objectEntity)}>
          {claim.objectName ?? claim.objectEntity}
        </button>
        {/* An amount between two parties: the other party and the amount. */}
        {claim.objectValue ? <span> · {claim.objectValue}</span> : null}
      </>
    ) : (
      <span>{claim.objectValue}</span>
    )

  const item = (claim: ClaimView) => (
    <li key={claim.id} className="py-2">
      <div className="flex flex-wrap items-baseline gap-2">
        <span className="text-xs font-medium text-muted-foreground">{predicateName(claim.predicate, false, words)}</span>
        <span className="font-medium">{value(claim)}</span>
        {claim.validFrom && <span className="text-xs text-muted-foreground">desde {claim.validFrom}</span>}
        <Certainty claim={claim} />
        {claim.certainty !== "confirmed" && (
          <button
            className="ml-auto text-xs text-muted-foreground hover:text-foreground"
            disabled={busy}
            onClick={() => void confirm(claim, `«${predicateName(claim.predicate, false, words)}: ${claim.objectName ?? claim.objectValue ?? ""}»`)}
          >
            Confirmar
          </button>
        )}
        <button
          className={`${claim.certainty === "confirmed" ? "ml-auto " : ""}text-xs text-muted-foreground hover:text-destructive`}
          disabled={busy}
          onClick={() => void markWrong(`claim:${claim.id}`, `«${predicateName(claim.predicate, false, words)}: ${claim.objectName ?? claim.objectValue ?? ""}»`)}
        >
          Marcar como incorrecto
        </button>
      </div>
      <ul className="mt-1 flex flex-col gap-1 pl-3">
        {claim.evidence.map((evidence) => (
          <li key={evidence.evidenceId} className="text-sm">
            <Quote>{evidence.quote}</Quote>{" "}
            <span className="text-xs text-muted-foreground">
              —{" "}
              <button className="hover:underline" onClick={() => openDocument(locationOf(entity, evidence.contentId))}>
                {evidence.document}
              </button>
              {evidence.page !== null && `, p. ${evidence.page}`}
              {evidence.factTime && ` · ${evidence.factTime}`}
            </span>
            {claim.evidence.length > 1 && (
              <button
                className="ml-2 text-xs text-muted-foreground hover:text-destructive"
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

  return (
    <>
      {conflicts.length > 0 && (
        <Section title="Datos en los que los documentos difieren">
          <p className="mb-2 text-sm text-muted-foreground">
            Dos documentos dicen cosas distintas de lo mismo. La app no elige: si uno está mal, marcalo.
          </p>
          <ul className="divide-y rounded-md border border-amber-500/40 px-3">{conflicts.map(item)}</ul>
        </Section>
      )}
      <Section title="Lo que dicen los documentos">
        {supported.length === 0 ? (
          <p className="text-sm text-muted-foreground">Los documentos no dicen de esta entidad nada más que su nombre y su rol.</p>
        ) : (
          <ul className="divide-y">{supported.map(item)}</ul>
        )}
      </Section>
    </>
  )
}

/** How sure the record is of a claim: doubtful in amber, with why; a person's word named. */
function Certainty({ claim }: { claim: ClaimView }) {
  const said = certaintyInWords(claim)
  const tone =
    said.tone === "doubt"
      ? "border-amber-500/50 text-amber-700 dark:text-amber-400"
      : said.tone === "person"
        ? "border-primary/40 text-primary"
        : "border-transparent text-muted-foreground"
  return (
    <span className={`rounded border px-1.5 text-xs ${tone}`} title={said.why || undefined}>
      {said.label}
      {said.why && `: ${said.why}`}
    </span>
  )
}

function locationOf(entity: EntityDetail, contentId: number): number | null {
  return entity.mentions.find((m) => m.contentId === contentId)?.locationId ?? null
}

/** Its documents, by matter, with its role and how each one writes it. */
function Documents({ entity, run, busy }: { entity: EntityDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const openDocument = useOpenDocument()
  const openMatter = useOpenMatter()
  const groups = documentsByMatter(entity.mentions, entity.matters, words.knowledge.withoutMatter)
  const documents = new Set(entity.mentions.map((m) => m.contentId)).size

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
    <Section title={`Documentos (${documents})`}>
      <div className="flex flex-col gap-4">
        {groups.map((group) => (
          <div key={group.matterId ?? "none"}>
            <h3 className="text-sm font-semibold">
              {group.matterId ? (
                <button className="text-left hover:underline" onClick={() => openMatter(group.matterId as string)}>
                  {group.name}
                </button>
              ) : (
                group.name
              )}
            </h3>
            <ul className="mt-1 flex flex-col divide-y rounded-md border">
              {group.documents.map((doc) => (
                <li key={`${group.matterId}-${doc.contentId}`} className="px-3 py-2 text-sm">
                  <div className="flex flex-wrap items-baseline gap-2">
                    <button className="text-left text-primary hover:underline" onClick={() => openDocument(doc.locationId)}>
                      {doc.name || `Documento ${doc.contentId}`}
                    </button>
                    <span className="text-xs text-muted-foreground">{rolesInWords(doc.roles, words)}</span>
                    {doc.proposed && <StatusBadge tone="neutral">propuesto por el modelo</StatusBadge>}
                  </div>
                  {!doc.proposed && (
                    <ul className="mt-1 flex flex-col gap-0.5 pl-3">
                      {doc.mentions.map((mention) => (
                        <li key={mention.mentionId} className="flex flex-wrap items-baseline gap-2 text-xs text-muted-foreground">
                          <span className="text-foreground">«{mention.written}»</span>
                          <span>{roleName(mention.role, words)}</span>
                          {mention.page !== null && <span>p. {mention.page}</span>}
                          {resolvedByName(mention.resolvedBy) && <span>· {resolvedByName(mention.resolvedBy)}</span>}
                          <span className="ml-auto flex gap-2">
                            <button
                              className="hover:text-foreground"
                              disabled={busy}
                              onClick={() => void separate(mention.mentionId, mention.written, doc.name)}
                            >
                              Separar
                            </button>
                            <button
                              className="hover:text-destructive"
                              disabled={busy}
                              onClick={() => void notAnEntity(mention.mentionId, mention.written, doc.name)}
                            >
                              No es una entidad
                            </button>
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Section>
  )
}

/** What other entities' claims say of this one ("represented by"). */
function Incoming({ entity }: { entity: EntityDetail }) {
  const words = vocabulary()
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  if (entity.incoming.length === 0) return null
  return (
    <Section title="Vínculos desde otras entidades">
      <ul className="flex flex-col gap-1 text-sm">
        {entity.incoming.map((claim) => (
          <li key={claim.claimId} className="flex flex-wrap items-baseline gap-2">
            <span className="text-xs font-medium text-muted-foreground">{predicateName(claim.predicate, true, words)}</span>
            <button className="text-primary hover:underline" onClick={() => openEntity(claim.subjectId)}>
              {claim.subjectName}
            </button>
            <span className="text-xs text-muted-foreground">
              {entityTypeName(claim.subjectKind, false, words).toLowerCase()} · {plural(claim.documents, "documento", "documentos")}
            </span>
          </li>
        ))}
      </ul>
    </Section>
  )
}

/** What a person decided about it, to undo. */
function Decisions({ entity, run, busy }: { entity: EntityDetail; run: Run; busy: boolean }) {
  if (entity.decisions.length === 0) return null
  return (
    <Section title="Decisiones">
      <ul className="flex flex-col divide-y rounded-md border text-sm">
        {entity.decisions.map((decision) => (
          <li key={decision.id} className="flex items-center gap-3 px-3 py-2">
            <span className="min-w-0 flex-1">
              <span className={decision.state === "undone" ? "text-muted-foreground line-through" : ""}>
                {decisionInWords(decision, entity)}
              </span>
              <span className="block text-xs text-muted-foreground">
                {formatDate(decision.decidedAt)}
                {decision.note && ` · ${decision.note}`}
                {decision.state === "undone" && decision.undoneAt && ` · deshecha el ${formatDate(decision.undoneAt)}`}
              </span>
              {decision.state === "active" && decision.review && (
                <span className="block text-xs text-amber-700 dark:text-amber-400">{reviewInWords(decision.review)}</span>
              )}
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

function FactBox({ entity }: { entity: EntityDetail }) {
  const words = vocabulary()
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const focusDoubt = useKnowledgeStore((s) => s.focusDoubt)
  return (
    <div className="rounded-md border bg-muted/30 text-sm">
      <div className="border-b bg-muted px-3 py-2 text-center font-semibold break-words">{entity.name}</div>
      <Heading>Se escribe</Heading>
      <ul className="flex flex-col gap-0.5 px-3 py-2">
        {entity.aliases.map((alias) => (
          <li key={alias.written} className="flex justify-between gap-2">
            <span className="break-words">{alias.written}</span>
            <span className="shrink-0 text-xs text-muted-foreground">{alias.mentions}</span>
          </li>
        ))}
      </ul>
      {entity.identifiers.length > 0 && (
        <>
          <Heading>Identificadores</Heading>
          <dl className="divide-y">
            {entity.identifiers.map((identifier) => (
              <div key={`${identifier.kind}:${identifier.value}`} className="grid grid-cols-[38%_minmax(0,1fr)] gap-2 px-3 py-1.5">
                <dt className="text-xs font-medium text-muted-foreground">{identifierName(identifier.kind, words)}</dt>
                <dd className="min-w-0 break-words">
                  {identifier.value}{" "}
                  <span className="text-xs text-muted-foreground">({plural(identifier.documents, "documento", "documentos")})</span>
                </dd>
              </div>
            ))}
          </dl>
        </>
      )}
      {entity.doubts.length > 0 && (
        <>
          <Heading>Dudas</Heading>
          <div className="px-3 py-2">
            <p>{plural(entity.doubts.length, "posible igual espera", "posibles iguales esperan")} una decisión.</p>
            <button
              className="mt-1 text-primary hover:underline"
              onClick={() => {
                focusDoubt(entity.id)
                setActiveView("identity-doubts")
              }}
            >
              Resolver
            </button>
          </div>
        </>
      )}
    </div>
  )
}

function Heading({ children }: { children: ReactNode }) {
  return <div className="border-y bg-muted/60 px-3 py-1 text-center text-xs font-semibold text-muted-foreground">{children}</div>
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
    <div className="mx-auto mb-6 max-w-6xl rounded-md border px-4 py-3">
      <p className="text-sm">
        ¿Con quién es la misma entidad? Buscá entre {entityTypeName(entity.kind, true, words).toLowerCase()} por nombre.
      </p>
      <Input className="mt-2 h-8 w-72" autoFocus value={typed} onChange={(event) => setTyped(event.target.value)} />
      {found.length > 0 && (
        <ul className="mt-2 flex flex-col divide-y rounded-md border text-sm">
          {found.map((other) => (
            <li key={other.id} className="flex items-center gap-3 px-3 py-1.5">
              <span className="min-w-0 flex-1 break-words">
                {other.name}
                <span className="ml-2 text-xs text-muted-foreground">{plural(other.documents, "documento", "documentos")}</span>
              </span>
              <Button size="sm" variant="outline" onClick={() => void join(other)}>
                Unir
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
