import { useState, type ReactNode } from "react"
import { Shapes } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getDocumentType, getTypesPlan, runTypes } from "@/commands/docbase"
import { MISSING_ASK_ENGINE, useAskConfig } from "@/lib/document-asking"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useWikiStore } from "@/stores/wiki-store"
import { documentTypeName, stageName, vocabulary } from "@/vocabulary"
import {
  confidenceText,
  dataRows,
  questionsText,
  runConfirmMessage,
  runReportText,
  typeLabel,
  typeSourceText,
} from "./document-types"
import { formatCount, plural } from "./format"
import { ErrorNote, Panel, StatusBadge } from "./page"
import { useDocbaseQuery } from "./use-docbase-query"

/** Questions a run started here asks at most; the rest wait for another run. */
export const RUN_LIMIT = 60

/**
 * The types of the documents of the base: how many have one, what is
 * left to ask the model, and the types it proposed. Asking costs the
 * provider's quota, so it only happens from the button, after a notice
 * that says how many questions it will make, or by itself while the
 * automatic reading is on.
 */
export function DocumentTypesPanel() {
  const plan = useDocbaseQuery("types-plan", getTypesPlan)
  const dialog = useAppDialog()
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const words = vocabulary()
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  // Every hook goes before the first return: React needs the same ones in
  // every render, loading or loaded.
  const engine = useAskConfig()

  const data = plan.data
  if (!data) return plan.error ? <ErrorNote>{plan.error}</ErrorNote> : null
  if (data.documents === 0) return null

  const questions = questionsText(data)

  async function run() {
    if (!data || !engine) return
    const confirmed = await dialog.confirm({
      title: "Reconocer los tipos de documento",
      message: runConfirmMessage(data, RUN_LIMIT),
      confirmLabel: "Preguntar",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    setBusy(true)
    setProblem(null)
    setNotice(null)
    try {
      setNotice(runReportText(await runTypes(engine, RUN_LIMIT)))
      plan.reload()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Panel
      title={
        <>
          <Shapes className="size-4" /> Tipos de documento
          {questions ? (
            <StatusBadge tone="neutral">{questions} sin hacer</StatusBadge>
          ) : (
            <StatusBadge tone="ok">Al día</StatusBadge>
          )}
        </>
      }
      action={
        questions ? (
          <Button
            size="sm"
            onClick={run}
            disabled={busy || !engine}
            title={engine ? undefined : MISSING_ASK_ENGINE}
          >
            {busy ? "Preguntando…" : "Reconocer tipos"}
          </Button>
        ) : undefined
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        {problem && <ErrorNote>{problem}</ErrorNote>}
        {notice && <p className="rounded-md bg-muted px-3 py-2">{notice}</p>}
        <p className="text-muted-foreground">
          {formatCount(data.typed)} de {plural(data.documents, "documento tiene", "documentos tienen")} tipo y etapa.
          Las marcas que se verifican sin modelo (una marca de agua, una firma, un formulario) deciden lo que pueden;
          lo demás se le pregunta al modelo, una vez por familia de documentos parecidos.
          {data.engine ? " Mientras la lectura automática esté activa, las preguntas se hacen solas." : ""}
        </p>
        {!engine && questions && (
          <p className="text-xs text-muted-foreground">
            {MISSING_ASK_ENGINE}
          </p>
        )}
        {data.proposedTypes.length > 0 && (
          <div>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="font-medium">Tipos que propuso el modelo y nadie confirmó todavía</p>
              <Button size="sm" variant="outline" onClick={() => setActiveView("document-types")}>
                Confirmar o rechazar
              </Button>
            </div>
            <ul className="mt-1 flex flex-col gap-1">
              {data.proposedTypes.map((proposed) => (
                <li key={proposed.id} className="text-xs">
                  <span className="font-medium">{documentTypeName(proposed.id, proposed.name, words)}</span>
                  {" · "}
                  {plural(proposed.documents, "documento", "documentos")}
                  {proposed.description && <span className="text-muted-foreground"> — {proposed.description}</span>}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </Panel>
  )
}

/** The type, the stage and the data of one document, for its fact box. */
export function DocumentTypeFacts({
  locationId,
  heading,
  row,
}: {
  locationId: number
  heading: (children: ReactNode) => ReactNode
  row: (label: string, children: ReactNode) => ReactNode
}) {
  const view = useDocbaseQuery(`document-type:${locationId}`, () => getDocumentType(locationId))
  const words = vocabulary()
  const found = view.data?.docType ?? null
  const data = dataRows(view.data?.data ?? [], words)
  if (!view.data) return null
  return (
    <>
      {heading("Tipo de documento")}
      <dl className="divide-y">
        {found ? (
          <>
            {row(
              "Tipo",
              <span title={found.proposedDescription ?? undefined}>
                {typeLabel(found.docType, null, found.proposed, found.proposedName, words)}
              </span>,
            )}
            {found.stage && row("Etapa", stageName(found.stage, words))}
            {row(
              "Según",
              <span className="text-xs">
                {typeSourceText(found)}
                {found.reason && <span className="block text-muted-foreground">«{found.reason}»</span>}
                {confidenceText(found) && <span className="block text-muted-foreground">{confidenceText(found)}</span>}
              </span>,
            )}
            {data.map((datum) =>
              row(
                datum.name,
                <span title={datum.byModel ? "Lo encontró el modelo" : "Lo encontraron las reglas del tipo"}>
                  {datum.values.join(" · ")}
                </span>,
              ),
            )}
          </>
        ) : (
          row("Tipo", <span className="text-xs text-muted-foreground">Todavía sin reconocer</span>)
        )}
      </dl>
    </>
  )
}
