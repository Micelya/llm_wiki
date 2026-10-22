import { useState } from "react"
import { Loader2, RotateCcw } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getRereadPlan, requestReread, type RereadPlan, type RereadTarget } from "@/commands/docbase"
import { plural } from "@/components/docbase/format"
import { ErrorNote } from "@/components/docbase/page"
import { useAppDialog } from "@/stores/app-dialog-store"

/**
 * The notice before reading again with the model (D-432): how many
 * documents, how many questions, and what happens to what is there.
 */
export function rereadConfirmMessage(plan: RereadPlan): string {
  const documents = plural(plan.documents, "documento", "documentos")
  const questions = plural(plan.questions, "pregunta al modelo", "preguntas al modelo")
  const already = plan.requested > 0 ? ` ${plural(plan.requested, "ya estaba pedido", "ya estaban pedidos")}.` : ""
  return (
    `Se van a volver a leer ${documents}: son ${questions}, que usan la cuota de tu proveedor.${already} ` +
    "Se leen de nuevo, sin usar las respuestas de antes, aunque la lectura automática esté en pausa. " +
    "La lectura que tiene cada uno queda hasta que termina la nueva; lo que decidiste sobre personas y datos se conserva."
  )
}

/** What a request left, in words. */
export function rereadNotice(count: number): string {
  if (count === 0) return "No hay documentos leídos para volver a leer."
  return `${plural(count, "documento pedido", "documentos pedidos")} para volver a leer: se leen de a uno, primero que el resto.`
}

/**
 * The button that asks to read again with the model a document, the
 * documents of an origin or folder, or the whole archive (D-432). It says
 * first what it costs; the request is kept in the base until it is done.
 */
export function ReadAgainButton({
  target,
  label = "Volver a leer con el modelo",
  onDone,
}: {
  target: RereadTarget
  label?: string
  onDone?: () => void
}) {
  const dialog = useAppDialog()
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  async function run() {
    setBusy(true)
    setProblem(null)
    setNotice(null)
    try {
      const plan = await getRereadPlan(target)
      if (plan.documents === 0) {
        setNotice(rereadNotice(0))
        return
      }
      if (!plan.engine) {
        setProblem("Falta elegir el motor de «Preguntar al modelo» en Ajustes → Servicios de IA: sin él no se puede volver a leer.")
        return
      }
      const confirmed = await dialog.confirm({
        title: "Volver a leer con el modelo",
        message: rereadConfirmMessage(plan),
        confirmLabel: "Volver a leer",
        cancelLabel: "Cancelar",
      })
      if (!confirmed) return
      setNotice(rereadNotice(await requestReread(target)))
      onDone?.()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }
  return (
    <div className="flex shrink-0 flex-col items-end gap-1">
      <Button size="sm" variant="outline" disabled={busy} onClick={() => void run()}>
        {busy ? <Loader2 className="animate-spin" /> : <RotateCcw />}
        {label}
      </Button>
      {notice && <p className="max-w-72 text-right text-xs text-muted-foreground">{notice}</p>}
      {problem && <ErrorNote>{problem}</ErrorNote>}
    </div>
  )
}
