import { useState } from "react"
import { Loader2, ScanText } from "lucide-react"
import { Button } from "@/components/ui/button"
import { recognizeText, type DocumentDetail } from "@/commands/docbase"
import {
  engineName,
  illegibleParts,
  isRecognitionEngine,
  MISSING_ENGINE,
  pageList,
  RECOGNITION_COST,
  recognitionEngineId,
  recognitionFailureText,
} from "@/lib/text-recognition"
import { useProgressStore } from "@/stores/progress-store"
import { recognitionConfigOfServices } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { vocabulary } from "@/vocabulary"
import { ErrorNote } from "./page"
import { formatDate } from "./format"

/** The engine of «Reconocer escaneos» chosen now, and its name in the base. */
export function useRecognitionEngine(): { id: string | null; name: string | null } {
  const services = useAiServicesStore((s) => s.services)
  const id = recognitionEngineId(recognitionConfigOfServices(services))
  return { id, name: id ? engineName(id) : null }
}

export interface Recognize {
  busy: boolean
  problem: string | null
  /** How many were asked for in the last request; null before any. */
  asked: number | null
  /** These documents, every one waiting (`null`), or the failed ones. */
  run: (contentIds: number[] | null, onlyFailed?: boolean) => Promise<void>
}

/** Asks the base to recognize scans in the background, and tells how it went. */
export function useRecognize(): Recognize {
  const refresh = useProgressStore((s) => s.refresh)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [asked, setAsked] = useState<number | null>(null)
  async function run(contentIds: number[] | null, onlyFailed = false) {
    setBusy(true)
    setProblem(null)
    try {
      setAsked(await recognizeText(contentIds, onlyFailed))
      refresh()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }
  return { busy, problem, asked, run }
}

/**
 * What the screen of a document says of the recognition of its text
 * («Datos y correcciones»): for a scan, a button «Reconocer el texto»
 * (with the automatic reading on or off) and why the last try failed;
 * for a recognized one, which engine read it and when, the pages it
 * could not read, and reading it again when another engine is chosen.
 */
export function DocumentRecognition({ document }: { document: DocumentDetail }) {
  const engine = useRecognitionEngine()
  const recognize = useRecognize()
  const where = `${vocabulary().attention.name} → En curso`
  if (document.contentId === null) return null
  const contentId = document.contentId

  if (document.state === "needs-recognition") {
    const partial = !!document.text
    const failed = document.error && recognize.asked === null
    return (
      <section data-recognition="waiting" className="mb-8 rounded-md border border-warn-line bg-warn-bg px-4 py-3 text-sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="flex items-center gap-2 font-medium">
              <ScanText className="size-4 shrink-0" /> El texto de este documento está en imagen
            </p>
            <p className="mt-1 text-muted-foreground">
              {partial
                ? "Parte de sus páginas son un escaneo: falta reconocer el texto de esas páginas."
                : "Es un escaneo o una foto: hay que reconocer su texto para leerlo, compararlo con los demás y buscarlo."}{" "}
              {RECOGNITION_COST}
            </p>
            {failed && <p className="mt-1 text-destructive">El último intento falló: {recognitionFailureText(document.error ?? "")}</p>}
            {recognize.asked !== null && (
              <p className="mt-1">
                {recognize.asked > 0
                  ? `Pedido: se reconoce en segundo plano, una página a la vez. El avance está en ${where}.`
                  : "No había nada que reconocer."}
              </p>
            )}
            {!engine.id && <p className="mt-1 text-warn">{MISSING_ENGINE}</p>}
          </div>
          <Button
            size="sm"
            className="shrink-0"
            disabled={!engine.id || recognize.busy || (recognize.asked ?? 0) > 0}
            onClick={() => void recognize.run([contentId])}
          >
            {recognize.busy && <Loader2 className="animate-spin" />}
            {failed ? "Reintentar" : "Reconocer el texto"}
          </Button>
        </div>
        {recognize.problem && (
          <div className="mt-2">
            <ErrorNote>{recognize.problem}</ErrorNote>
          </div>
        )}
      </section>
    )
  }

  if (!isRecognitionEngine(document.textEngine)) return null
  const illegible = illegibleParts(document.text)
  const other = engine.id && engine.id !== document.textEngine
  return (
    <section data-recognition="done" className="mb-8 rounded-md border bg-muted/20 px-4 py-3 text-sm">
      <p className="flex items-center gap-2">
        <ScanText className="size-4 shrink-0 text-muted-foreground" />
        <span>
          Texto reconocido por {engineName(document.textEngine)}
          {document.textAt ? ` el ${formatDate(document.textAt)}` : ""}.
        </span>
      </p>
      {illegible && (
        <p className="mt-1 text-muted-foreground">
          {illegible.pages.length > 0
            ? `El motor no pudo leer partes de la ${pageList(illegible.pages)}: están marcadas «[ilegible]». Conviene compararlas con el original.`
            : "El motor no pudo leer algunas partes: están marcadas «[ilegible]». Conviene compararlas con el original."}
        </p>
      )}
      {other && (
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <Button size="sm" variant="outline" disabled={recognize.busy || (recognize.asked ?? 0) > 0} onClick={() => void recognize.run([contentId])}>
            {recognize.busy && <Loader2 className="animate-spin" />}
            Reconocer de nuevo con {engine.name}
          </Button>
          <span className="text-xs text-muted-foreground">
            {recognize.asked ? `Pedido: el avance está en ${where}.` : RECOGNITION_COST}
          </span>
        </div>
      )}
      {recognize.problem && (
        <div className="mt-2">
          <ErrorNote>{recognize.problem}</ErrorNote>
        </div>
      )}
    </section>
  )
}
