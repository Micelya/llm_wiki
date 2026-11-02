import { useState } from "react"
import { Loader2, ScanText, Sparkles } from "lucide-react"
import { Button } from "@/components/ui/button"
import { aiReadingCost, readWithAi, recognizeText, type AiReadingCost, type DocumentDetail } from "@/commands/docbase"
import {
  engineName,
  illegibleParts,
  isCloudReader,
  isRecognitionEngine,
  pageList,
  RECOGNITION_COST,
  recognitionEngineId,
  recognitionFailureText,
} from "@/lib/text-recognition"
import { useProgressStore } from "@/stores/progress-store"
import { aiReadingConfigOfServices, recognitionConfigOfServices } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { vocabulary } from "@/vocabulary"
import { ErrorNote } from "./page"
import { formatDate } from "./format"

/** The reader of «Reconocer escaneos» chosen now (the one of this computer by default), and its name in the base. */
export function useRecognitionEngine(): { id: string | null; name: string | null } {
  const services = useAiServicesStore((s) => s.services)
  const catalog = useAiServicesStore((s) => s.catalog)
  const id = recognitionEngineId(recognitionConfigOfServices(services, catalog))
  return { id, name: id ? engineName(id) : null }
}

/**
 * «Leer con IA» (E2 point 6; D-670, D-705): for a document the reader of
 * this computer did not read well, or an old manuscript. It says the cost
 * first (its reader and its pages, each a question); confirmed, it leaves a
 * request the process reads like any other, with the reader of AI chosen in
 * Ajustes. Without one, it says where to choose it.
 */
function ReadWithAi({ contentId, textEngine }: { contentId: number; textEngine: string | null }) {
  const services = useAiServicesStore((s) => s.services)
  const catalog = useAiServicesStore((s) => s.catalog)
  const refresh = useProgressStore((s) => s.refresh)
  const reader = recognitionEngineId(aiReadingConfigOfServices(services, catalog))
  const [cost, setCost] = useState<AiReadingCost | null>(null)
  const [asked, setAsked] = useState<number | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  if (reader && textEngine && textEngine === reader) return null
  async function look() {
    setProblem(null)
    try {
      setCost(await aiReadingCost(contentId))
    } catch (cause) {
      setProblem(String(cause))
    }
  }
  async function confirm() {
    setProblem(null)
    try {
      setAsked(await readWithAi(contentId))
      setCost(null)
      refresh()
    } catch (cause) {
      setProblem(String(cause))
    }
  }
  return (
    <div data-read-with-ai className="mt-2 space-y-1">
      {!reader ? (
        <p className="text-muted-foreground">
          Para leerlo con IA, elegí el lector de IA en Ajustes → Servicios de IA → Reconocer escaneos → «Lector de IA para
          «Leer con IA»».
        </p>
      ) : cost ? (
        <div className="flex flex-wrap items-center gap-2">
          <span>
            Leer con {engineName(cost.engine)}: {cost.pages} {cost.pages === 1 ? "página, 1 consulta" : `páginas, ${cost.pages} consultas`}{" "}
            a ese proveedor.
          </span>
          <Button size="sm" onClick={() => void confirm()}>
            Leer con IA
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setCost(null)}>
            Cancelar
          </Button>
        </div>
      ) : asked !== null ? (
        <p>{asked > 0 ? "Pedido: se lee con IA en segundo plano; el avance está en «En curso»." : "No había nada que leer con ese lector."}</p>
      ) : (
        <Button size="sm" variant="outline" onClick={() => void look()}>
          <Sparkles className="size-4" /> Leer con IA
        </Button>
      )}
      {problem && <ErrorNote>{problem}</ErrorNote>}
    </div>
  )
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
 * for a recognized one, which engine read it and when and the pages it
 * could not read. The engine is the one of the configuration (D-721): the
 * person does not choose it here.
 */
export function DocumentRecognition({ document }: { document: DocumentDetail }) {
  const recognize = useRecognize()
  const engine = useRecognitionEngine()
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
              {engine.name ? `Lo lee ${engine.name}, el lector elegido en Ajustes.` : ""}{" "}
              {isCloudReader(engine.id) ? RECOGNITION_COST : ""}
            </p>
            {failed && <p className="mt-1 text-destructive">El último intento falló: {recognitionFailureText(document.error ?? "")}</p>}
            {recognize.asked !== null && (
              <p className="mt-1">
                {recognize.asked > 0
                  ? `Pedido: se reconoce en segundo plano, una página a la vez. El avance está en ${where}.`
                  : "No había nada que reconocer."}
              </p>
            )}

          </div>
          <Button
            size="sm"
            className="shrink-0"
            disabled={recognize.busy || (recognize.asked ?? 0) > 0}
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
        <ReadWithAi contentId={contentId} textEngine={null} />
      </section>
    )
  }

  if (!isRecognitionEngine(document.textEngine)) return null
  const illegible = illegibleParts(document.text)
  return (
    <section data-recognition="done" className="mb-8 rounded-md border bg-muted/20 px-4 py-3 text-sm">
      <p className="flex items-center gap-2">
        <ScanText className="size-4 shrink-0 text-muted-foreground" />
        <span>
          Texto reconocido por {engineName(document.textEngine)}
          {document.textAt ? ` el ${formatDate(document.textAt)}` : ""}.
        </span>
      </p>
      {document.text?.includes("[aproximado]") && (
        <p className="mt-1 text-muted-foreground">
          Los renglones leídos con poca seguridad están marcados «[aproximado]»: sirven para entender el documento y no dan datos.
          Lo leído con menos seguridad todavía no está en el texto.
        </p>
      )}
      {illegible && (
        <p className="mt-1 text-muted-foreground">
          {illegible.pages.length > 0
            ? `El motor no pudo leer partes de la ${pageList(illegible.pages)}: están marcadas «[ilegible]». Conviene compararlas con el original.`
            : "El motor no pudo leer algunas partes: están marcadas «[ilegible]». Conviene compararlas con el original."}
        </p>
      )}
      {recognize.problem && (
        <div className="mt-2">
          <ErrorNote>{recognize.problem}</ErrorNote>
        </div>
      )}
      <ReadWithAi contentId={contentId} textEngine={document.textEngine ?? null} />
    </section>
  )
}
