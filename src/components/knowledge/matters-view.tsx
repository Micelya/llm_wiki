import { ArrowLeft, BookOpen, Undo2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  getMatter,
  joinMatter,
  leaveMatter,
  listMatters,
  undoIdentityDecision,
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
import { rolesInWords } from "./knowledge"
import { Section, useAction, useArchivePage, useOpenDocument, useOpenEntity } from "./shared"

/**
 * The matters of the archive: groups of documents joined by what is
 * specific to one matter (R-05), with the reasons each document is in it,
 * its parties, the documents the model proposes for it, and what a
 * person decided. A person puts a document in or takes it out; nothing
 * else is edited.
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
  const rows = matters.data ?? []
  return (
    <DocbasePage title={words.knowledge.matters} subtitle={words.knowledge.mattersDescription}>
      <div className="mx-auto flex max-w-5xl flex-col gap-4">
        {matters.error && <ErrorNote>{matters.error}</ErrorNote>}
        {matters.data && rows.length === 0 && (
          <EmptyNote>
            Todavía no hay documentos agrupados: hacen falta dos que compartan algo propio de un mismo{" "}
            {words.knowledge.matter.toLowerCase()}.
          </EmptyNote>
        )}
        {rows.length > 0 && (
          <ul className="divide-y rounded-lg border bg-card">
            {rows.map((matter) => (
              <li key={matter.id}>
                <button className="flex w-full items-start gap-3 px-4 py-2.5 text-left hover:bg-muted/50" onClick={() => openMatter(matter.id)}>
                  <span className="min-w-0 flex-1 font-medium break-words">{matter.name}</span>
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

function MatterDetailView({ id, onBack }: { id: string; onBack: () => void }) {
  const words = vocabulary()
  const matter = useDocbaseQuery(`matter:${id}`, () => getMatter(id))
  const archivePage = useArchivePage()
  const action = useAction()
  const detail = matter.data
  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b px-8 pt-4">
        <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground" onClick={onBack}>
          <ArrowLeft className="size-4" /> {words.knowledge.matters}
        </button>
        <div className="mt-2 flex items-end justify-between gap-4 pb-3">
          <h1 className="min-w-0 font-serif text-2xl break-words">{detail?.name ?? words.knowledge.matter}</h1>
          {detail?.pagePath && (
            <Button
              variant="outline"
              disabled={!archivePage}
              title={archivePage ? detail.pagePath : `Se abre con el ${words.archive.wiki.toLowerCase()} abierto`}
              onClick={() => archivePage?.(detail.pagePath as string)}
            >
              <BookOpen /> Su página del wiki
            </Button>
          )}
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-6">
        {(matter.error || action.problem) && (
          <div className="mb-4">
            <ErrorNote>{matter.error ?? action.problem}</ErrorNote>
          </div>
        )}
        {!matter.loading && !detail && !matter.error && (
          <EmptyNote>Este {words.knowledge.matter.toLowerCase()} ya no existe: sus documentos se agruparon de otra manera.</EmptyNote>
        )}
        {detail && (
          <div className="mx-auto max-w-5xl">
            {detail.state === "retired" && (
              <p className="mb-4 rounded-md border border-dashed px-3 py-2 text-sm">
                Ya no queda ningún grupo con estos documentos.
              </p>
            )}
            <Documents matter={detail} run={action.run} busy={action.busy} />
            <Parties matter={detail} />
            <Decisions matter={detail} run={action.run} busy={action.busy} />
          </div>
        )}
      </div>
    </div>
  )
}

type Run = (action: () => Promise<unknown>) => Promise<void>

function Documents({ matter, run, busy }: { matter: MatterDetail; run: Run; busy: boolean }) {
  const words = vocabulary()
  const dialog = useAppDialog()
  const openDocument = useOpenDocument()
  const firm = matter.documents.filter((d) => d.state === "firm")
  const proposed = matter.documents.filter((d) => d.state === "proposed")
  const name = words.knowledge.matter.toLowerCase()

  async function takeOut(doc: MatterDocument) {
    const confirmed = await dialog.confirm({
      title: `Sacar del ${name}`,
      message:
        `«${doc.document.name}» deja de ser de este ${name}: queda separado de cada uno de sus documentos, ` +
        "y ninguna pasada lo vuelve a poner. Se puede deshacer desde las decisiones.",
      confirmLabel: "Sacar",
      cancelLabel: "Cancelar",
    })
    if (confirmed) await run(() => leaveMatter(`content:${doc.document.contentId}`, matter.id, `fuera de «${matter.name}»`))
  }

  const row = (doc: MatterDocument) => (
    <li key={doc.document.contentId} className="px-3 py-2 text-sm">
      <div className="flex flex-wrap items-baseline gap-2">
        <button className="text-left text-primary hover:underline" onClick={() => openDocument(doc.document.locationId)}>
          {doc.document.name || `Documento ${doc.document.contentId}`}
        </button>
        <span className="ml-auto flex gap-2">
          {doc.state === "firm" ? (
            firm.length > 1 && (
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
                onClick={() => void run(() => joinMatter(`content:${doc.document.contentId}`, matter.id, "propuesta del modelo confirmada"))}
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
        <ul className="flex flex-col divide-y rounded-md border">{firm.map(row)}</ul>
      </Section>
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
