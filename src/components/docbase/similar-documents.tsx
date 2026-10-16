import { useState } from "react"
import { Check, Columns2, Undo2, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  decideSimilar,
  listSimilarDocuments,
  listSimilarRelations,
  type RelationDef,
  type RelationId,
  type SimilarDecision,
  type SimilarDocument,
} from "@/commands/docbase"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { relationName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatDate } from "./format"
import { StatusBadge } from "./page"
import {
  currentRelation,
  makesOneDocument,
  measuresInWords,
  relationChoices,
  relationSentence,
  suggestedWikiSource,
} from "./similar"
import { useDocbaseQuery } from "./use-docbase-query"

const SELECT_CLASS =
  "h-7 rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"

/**
 * "Documentos parecidos", below the locations of a document: each
 * similar document with the relation proposed, what they have in common
 * and the buttons to join, choose another relation, discard or undo.
 * Nothing is joined without a person (design, section 3).
 */
export function SimilarDocuments({
  locationId,
  name,
  onProblem,
}: {
  locationId: number
  /** The name of the document shown. */
  name: string
  onProblem: (problem: string | null) => void
}) {
  const similar = useDocbaseQuery(`similar:${locationId}`, () => listSimilarDocuments(locationId))
  const relations = useDocbaseQuery("similar-relations", listSimilarRelations)
  const openDocument = useDocbaseStore((s) => s.openDocument)

  async function decide(item: SimilarDocument, decision: SimilarDecision) {
    await decideAndFollow(item, decision, onProblem, similar.reload)
  }

  if (!similar.data) return null
  return (
    <SimilarList
      items={similar.data}
      relations={relations.data ?? []}
      name={name}
      onOpen={openDocument}
      onDecide={decide}
    />
  )
}

/**
 * Records what a person decided about a pair, and has the wiki follow:
 * the one not chosen stops feeding it, and what is a source now and was
 * never ingested is queued. Problems go to `onProblem`.
 */
export async function decideAndFollow(
  item: SimilarDocument,
  decision: SimilarDecision,
  onProblem: (problem: string | null) => void,
  onDecided: () => void = () => {},
): Promise<void> {
  onProblem(null)
  try {
    await decideSimilar(item.contentId, item.other.contentId, decision, item.evidenceHash)
    onDecided()
  } catch (cause) {
    onProblem(String(cause))
    return
  }
  try {
    const { followSimilarDecision } = await import("@/lib/docbase-wiki-bridge")
    await followSimilarDecision(useWikiStore.getState().project, [item.contentId, item.other.contentId])
  } catch (cause) {
    onProblem(`La decisión quedó guardada, pero el wiki no se pudo actualizar: ${String(cause)}`)
  }
}

/** The section itself, apart from where its data comes from. */
export function SimilarList({
  items,
  relations,
  name,
  onOpen,
  onDecide,
  words = vocabulary(),
}: {
  items: SimilarDocument[]
  relations: RelationDef[]
  name: string
  onOpen: (locationId: number) => void
  onDecide: (item: SimilarDocument, decision: SimilarDecision) => Promise<void> | void
  words?: Vocabulary
}) {
  const waiting = items.filter((item) => item.state === "proposed").length
  return (
    <section className="mt-8">
      <h2 className="border-b pb-1 font-serif text-xl">
        Documentos parecidos ({items.length})
      </h2>
      <div className="mt-3">
        {items.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Ningún otro documento de la base comparte buena parte de su texto con este.
          </p>
        ) : (
          <>
            {waiting > 0 && (
              <p className="mb-3 text-sm text-muted-foreground">
                Nada se une solo: cada parecido espera que alguien lo una con la relación que corresponda o lo
                descarte.
              </p>
            )}
            <ul className="flex flex-col gap-3">
              {items.map((item) => (
                <SimilarCard
                  key={item.other.contentId}
                  item={item}
                  relations={relations}
                  name={name}
                  words={words}
                  onOpen={onOpen}
                  onDecide={onDecide}
                />
              ))}
            </ul>
          </>
        )}
      </div>
    </section>
  )
}

/** A join being prepared: the relation, and the wiki's source when it makes them one document. */
interface Joining {
  relation: RelationId
  /** True while the person picks the relation from the list. */
  choosing: boolean
  source: number
}

export function SimilarCard({
  item,
  relations,
  name,
  words,
  onOpen,
  onDecide,
  initialJoining = null,
}: {
  item: SimilarDocument
  relations: RelationDef[]
  name: string
  words: Vocabulary
  onOpen: (locationId: number) => void
  onDecide: (item: SimilarDocument, decision: SimilarDecision) => Promise<void> | void
  /** Only for tests and previews: the card opened on a join. */
  initialJoining?: Joining | null
}) {
  const [joining, setJoining] = useState<Joining | null>(initialJoining)
  const [busy, setBusy] = useState(false)
  const relation = currentRelation(item)
  const proposedNothing = item.relation === "similar"
  const sentence = relationSentence(item, relation, words)
  const suggested = suggestedWikiSource(item)
  const choices = relationChoices(relations, words)

  async function run(decision: SimilarDecision) {
    setBusy(true)
    try {
      await onDecide(item, decision)
      setJoining(null)
    } finally {
      setBusy(false)
    }
  }

  function startJoin(target: RelationId, choosing: boolean) {
    if (!choosing && !makesOneDocument(target, relations)) {
      void run({ kind: "join", relation: target, wikiSource: null })
      return
    }
    setJoining({ relation: target, choosing, source: suggested })
  }

  function confirmJoin() {
    if (!joining) return
    const one = makesOneDocument(joining.relation, relations)
    void run({ kind: "join", relation: joining.relation, wikiSource: one ? joining.source : null })
  }

  const sourceName = (id: number | null) =>
    id === item.contentId ? `este documento (${name})` : id === item.other.contentId ? `«${item.other.name}»` : "—"

  return (
    <li
      className={`rounded-md border px-4 py-3 text-sm ${item.state === "discarded" ? "opacity-60" : ""}`}
      data-state={item.state}
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <button
            className="text-left font-medium text-primary break-words hover:underline"
            onClick={() => onOpen(item.other.locationId)}
          >
            {item.other.name}
          </button>
          <div className="text-xs break-words text-muted-foreground">
            {item.other.originName} › {item.other.relPath}
            {item.other.copies > 1 && ` · ${item.other.copies} ubicaciones`}
          </div>
        </div>
        <StateBadge item={item} />
      </div>

      <div className="mt-2">
        <span className="text-muted-foreground">
          {item.state === "joined" ? "Unidos como " : proposedNothing ? "" : "Relación propuesta: "}
        </span>
        {proposedNothing && item.state !== "joined" ? (
          <span className="font-medium">Sin relación sugerida</span>
        ) : (
          <span className="font-medium">{relationName(relation, words)}</span>
        )}
        {item.certainty === "low" && item.state === "proposed" && !proposedNothing && (
          <span className="ml-2 text-xs text-muted-foreground">(propuesta con menos certeza)</span>
        )}
        {sentence && <div className="text-muted-foreground">{sentence}</div>}
      </div>

      <p className="mt-1 font-serif italic">{item.description}</p>

      <ul className="mt-1 flex flex-col text-xs text-muted-foreground">
        {measuresInWords(item).map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>

      {item.state === "joined" && item.wikiSource !== null && (
        <p className="mt-2 text-xs">
          Queda como fuente del wiki: <span className="font-medium">{sourceName(item.wikiSource)}</span>.
        </p>
      )}

      {joining && (
        <JoinPanel
          item={item}
          joining={joining}
          choices={choices}
          oneDocument={makesOneDocument(joining.relation, relations)}
          suggested={suggested}
          sourceName={sourceName}
          busy={busy}
          onChange={setJoining}
          onConfirm={confirmJoin}
          onCancel={() => setJoining(null)}
        />
      )}

      {!joining && (
        <div className="mt-3 flex flex-wrap items-center gap-2">
          {item.state === "proposed" ? (
            <>
              {!proposedNothing && (
                <Button size="sm" disabled={busy} onClick={() => startJoin(item.relation, false)}>
                  <Check /> Unir como «{relationName(item.relation, words)}»
                </Button>
              )}
              <Button
                size="sm"
                variant="outline"
                disabled={busy || choices.measured.length + choices.inferred.length === 0}
                onClick={() => startJoin(proposedNothing ? choices.measured[0]?.id ?? item.relation : item.relation, true)}
              >
                {proposedNothing ? "Elegir una relación" : "Elegir otra relación"}
              </Button>
              <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run({ kind: "discard" })}>
                <X /> Descartar
              </Button>
            </>
          ) : (
            <Button size="sm" variant="outline" disabled={busy} onClick={() => void run({ kind: "undo" })}>
              <Undo2 /> Deshacer
            </Button>
          )}
          <Button
            size="sm"
            variant="ghost"
            disabled
            title="Todavía no está disponible: los dos textos lado a lado, con lo que cambia resaltado."
          >
            <Columns2 /> Ver diferencias
          </Button>
        </div>
      )}
    </li>
  )
}

function StateBadge({ item }: { item: SimilarDocument }) {
  if (item.state === "joined") {
    return (
      <StatusBadge tone="ok" title={item.decidedAt ? `Unidos el ${formatDate(item.decidedAt)}` : undefined}>
        Unidos{item.decidedAt ? ` el ${formatDate(item.decidedAt)}` : ""}
      </StatusBadge>
    )
  }
  if (item.state === "discarded") {
    return (
      <StatusBadge tone="neutral">
        Descartado{item.decidedAt ? ` el ${formatDate(item.decidedAt)}` : ""}
      </StatusBadge>
    )
  }
  return <StatusBadge tone="busy">Espera una decisión</StatusBadge>
}

function JoinPanel({
  item,
  joining,
  choices,
  oneDocument,
  suggested,
  sourceName,
  busy,
  onChange,
  onConfirm,
  onCancel,
}: {
  item: SimilarDocument
  joining: Joining
  choices: ReturnType<typeof relationChoices>
  oneDocument: boolean
  suggested: number
  sourceName: (id: number | null) => string
  busy: boolean
  onChange: (joining: Joining) => void
  onConfirm: () => void
  onCancel: () => void
}) {
  return (
    <div className="mt-3 flex flex-col gap-3 rounded-md border bg-muted/30 px-3 py-3">
      {joining.choosing && (
        <label className="flex flex-wrap items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground">Relación</span>
          <select
            className={SELECT_CLASS}
            value={joining.relation}
            onChange={(event) => onChange({ ...joining, relation: event.target.value })}
          >
            <optgroup label="Se miden comparando textos">
              {choices.measured.map((choice) => (
                <option key={choice.id} value={choice.id}>
                  {choice.name}
                </option>
              ))}
            </optgroup>
            {choices.inferred.length > 0 && (
              <optgroup label="Se infieren">
                {choices.inferred.map((choice) => (
                  <option key={choice.id} value={choice.id}>
                    {choice.name}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        </label>
      )}
      {oneDocument ? (
        <fieldset className="flex flex-col gap-1">
          <legend className="mb-1 text-xs font-medium text-muted-foreground">
            Con esta relación los dos son el mismo documento: ¿cuál queda como fuente del wiki?
          </legend>
          {[item.contentId, item.other.contentId].map((id) => (
            <label key={id} className="flex items-center gap-2">
              <input
                type="radio"
                name={`source-${item.contentId}-${item.other.contentId}`}
                checked={joining.source === id}
                onChange={() => onChange({ ...joining, source: id })}
              />
              <span>{sourceName(id)}</span>
              {id === suggested && <span className="text-xs text-muted-foreground">(sugerido)</span>}
            </label>
          ))}
          <p className="mt-1 text-xs text-muted-foreground">
            Los dos siguen en la base. Solo el elegido alimenta el wiki; lo que el otro ya aportó se queda.
          </p>
        </fieldset>
      ) : (
        <p className="text-xs text-muted-foreground">Son documentos distintos: los dos siguen yendo al wiki.</p>
      )}
      <div className="flex gap-2">
        <Button size="sm" disabled={busy} onClick={onConfirm}>
          <Check /> Unir
        </Button>
        <Button size="sm" variant="ghost" disabled={busy} onClick={onCancel}>
          Cancelar
        </Button>
      </div>
    </div>
  )
}
