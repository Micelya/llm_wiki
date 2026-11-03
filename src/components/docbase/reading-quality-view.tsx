import { useState, type ReactNode } from "react"
import { ExternalLink, SlidersHorizontal } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getReadingQuality, type QualityDocument, type ReadingQuality } from "@/commands/docbase"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { DocbasePage, ErrorNote } from "./page"
import { ReadWithAi } from "./recognition"
import { useDocbaseQuery } from "./use-docbase-query"
import { below, pagesText, percent, verdictText, wordsText } from "./reading-quality"
import { formatCount, plural } from "./format"

/**
 * «Calidad de la lectura» (E2 of 2 nov, P-544): the documents whose
 * reading has few safe words, to choose the thresholds with data. It is
 * reached from Atención and from Documentos. Looking writes nothing; «Leer
 * con IA» says its cost first and is a request like any other.
 */
export function ReadingQualityView() {
  const quality = useDocbaseQuery("reading-quality", getReadingQuality)
  const [value, setValue] = useState<number | null>(null)
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  function open(document: QualityDocument) {
    if (document.locationId === null) return
    setActiveView("documents")
    openDocument(document.locationId, document.firstPage !== null ? document.versionId : null)
  }
  return (
    <DocbasePage
      title="Calidad de la lectura"
      subtitle="Los documentos que el lector leyó con pocas palabras seguras, para elegir los umbrales mirando los documentos."
      actions={
        <Button variant="outline" onClick={() => setActiveView("settings")}>
          <SlidersHorizontal /> Umbrales en Ajustes
        </Button>
      }
    >
      {quality.error && <ErrorNote>{quality.error}</ErrorNote>}
      {quality.data && (
        <ReadingQualityBody
          data={quality.data}
          value={value ?? quality.data.quality}
          onValue={setValue}
          onOpen={open}
          action={(document) => <ReadWithAi contentId={document.versionId} textEngine={null} />}
        />
      )}
    </DocbasePage>
  )
}

interface BodyProps {
  data: ReadingQuality
  /** The value the rule is on, 0..1. */
  value: number
  onValue: (value: number) => void
  onOpen: (document: QualityDocument) => void
  /** What each document offers besides opening it: «Leer con IA». */
  action: (document: QualityDocument) => ReactNode
}

/** What the screen shows, from what the base says. */
export function ReadingQualityBody({ data, value, onValue, onOpen, action }: BodyProps) {
  const notLegible = data.belowOne.filter((d) => d.verdict === "no-legible")
  const lowConfidence = data.belowOne.filter((d) => d.verdict === "low-confidence")
  const used = data.belowOne.filter((d) => d.verdict === null)
  const atValue = below(data.belowOne, value)
  const limit = Math.max(data.quality, data.lowConfidenceType)
  return (
    <div className="max-w-4xl space-y-8 text-sm" data-quality="body">
      <section className="space-y-2">
        <p>
          La <strong>proporción de palabras seguras</strong> de un documento es lo que el lector leyó con confianza alta
          sobre todo lo que leyó. Por ejemplo, un documento con 9 palabras seguras, 18 aproximadas y ninguna descartada
          tiene 9 de 27: {percent(9 / 27)}.
        </p>
        <p>
          Debajo de <strong>«no legible»</strong> ({percent(data.quality)}) o del <strong>tipo de confianza baja</strong>{" "}
          ({percent(data.lowConfidenceType)}), un documento sirve solo para consultas: se busca, se abre y se lee, pero no
          da datos, no une personas y no tiene página en la wiki. Los dos valores son provisorios: se eligen con esta lista.
        </p>
        <p className="text-muted-foreground" data-quality="totals">
          {plural(data.documents, "documento", "documentos")} en la base; {formatCount(data.measured)} con su proporción;{" "}
          {plural(data.belowOne.length, "tiene", "tienen")} alguna palabra que no es segura.
        </p>
      </section>

      <section className="space-y-3 rounded-md border p-4" data-quality="rule">
        <h2 className="font-ui text-[0.9375rem] font-bold text-strong">La regla para elegir el umbral</h2>
        <label className="flex flex-wrap items-center gap-3">
          <span>Mover el valor:</span>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={Math.round(value * 100)}
            onChange={(event) => onValue(Number(event.target.value) / 100)}
            className="w-64"
            aria-label="Valor del umbral"
          />
          <strong>{percent(value)}</strong>
        </label>
        <p data-quality="at-value">
          Con {percent(value)}, quedan debajo {plural(atValue.documents, "documento", "documentos")} (
          {plural(atValue.pages, "página", "páginas")}). Hoy, con {percent(limit)}, quedan{" "}
          {formatCount(below(data.belowOne, limit).documents)}.
        </p>
        <table className="w-full max-w-md border-collapse text-left">
          <thead>
            <tr className="border-b text-muted-foreground">
              <th className="py-1 pr-4 font-medium">Valor</th>
              <th className="py-1 pr-4 font-medium">Documentos debajo</th>
              <th className="py-1 font-medium">Páginas debajo</th>
            </tr>
          </thead>
          <tbody>
            {data.steps.map((step) => (
              <tr key={step.value} className="border-b last:border-0" data-step={Math.round(step.value * 100)}>
                <td className="py-1 pr-4">
                  {percent(step.value)}
                  {step.value === data.quality && <span className="ml-2 text-muted-foreground">«no legible» hoy</span>}
                  {step.value === data.lowConfidenceType && <span className="ml-2 text-muted-foreground">confianza baja hoy</span>}
                </td>
                <td className="py-1 pr-4">{formatCount(step.documents)}</td>
                <td className="py-1">{formatCount(step.pages)}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="text-muted-foreground">
          Los umbrales se cambian en Ajustes → Umbrales, que dice antes de guardar qué documentos cambian de lado y si se
          vuelve a preguntar algo.
        </p>
      </section>

      <DocumentList
        title={`No legibles (${formatCount(notLegible.length)})`}
        empty="Ningún documento queda debajo de «no legible»."
        list={notLegible}
        onOpen={onOpen}
        action={action}
        tag="no-legible"
      />
      <DocumentList
        title={`De tipo de confianza baja (${formatCount(lowConfidence.length)})`}
        empty="Ningún documento queda entre «no legible» y el tipo de confianza baja."
        list={lowConfidence}
        onOpen={onOpen}
        action={action}
        tag="low-confidence"
      />
      <DocumentList
        title={`Con alguna palabra dudosa, que se usan (${formatCount(used.length)})`}
        empty="Ninguno."
        list={used}
        onOpen={onOpen}
        action={action}
        tag="used"
      />
      {(data.unmeasured.length > 0 || data.withoutText.length > 0) && (
        <section className="space-y-2" data-quality="apart">
          <h2 className="font-ui text-[0.9375rem] font-bold text-strong">Aparte</h2>
          {data.unmeasured.length > 0 && (
            <p>
              {plural(data.unmeasured.length, "documento todavía no tiene", "documentos todavía no tienen")} su proporción:
              el proceso la calcula en el paso de la firma de parecidos, sin preguntar nada.
            </p>
          )}
          {data.withoutText.length > 0 && (
            <DocumentList
              title={`Sin texto (${formatCount(data.withoutText.length)})`}
              empty=""
              list={data.withoutText}
              onOpen={onOpen}
              action={action}
              tag="without-text"
              withShare={false}
            />
          )}
        </section>
      )}
    </div>
  )
}

function DocumentList({
  title,
  empty,
  list,
  onOpen,
  action,
  tag,
  withShare = true,
}: {
  title: string
  empty: string
  list: QualityDocument[]
  onOpen: (document: QualityDocument) => void
  action: (document: QualityDocument) => ReactNode
  tag: string
  withShare?: boolean
}) {
  return (
    <section className="space-y-2" data-quality-list={tag}>
      <h2 className="font-ui text-[0.9375rem] font-bold text-strong">{title}</h2>
      {list.length === 0 && empty && <p className="text-muted-foreground">{empty}</p>}
      <ul className="divide-y rounded-md border">
        {list.map((document) => (
          <li key={document.versionId} className="flex flex-wrap items-start gap-x-6 gap-y-2 px-4 py-3" data-document={document.versionId}>
            <div className="min-w-[16rem] flex-1 break-words">
              <p className="font-medium">{document.name || "Sin lugar visible"}</p>
              <p className="text-muted-foreground">
                {document.path} · {pagesText(document)}
              </p>
              {withShare && (
                <p>
                  <strong>{percent(document.share)}</strong> de palabras seguras ({wordsText(document)}) ·{" "}
                  {verdictText(document.verdict)}
                </p>
              )}
            </div>
            <div className="flex flex-wrap items-start gap-2">
              <Button size="sm" variant="secondary" disabled={document.locationId === null} onClick={() => onOpen(document)}>
                <ExternalLink className="size-4" /> Abrir
              </Button>
              {action(document)}
            </div>
          </li>
        ))}
      </ul>
    </section>
  )
}
