import { useEffect, useState } from "react"
import { BookOpen, Pause, Play } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getBackgroundReading, setAskEngine, setBackgroundReading } from "@/commands/docbase"
import { getAskConfig } from "@/lib/document-asking"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useWikiStore } from "@/stores/wiki-store"
import { vocabulary } from "@/vocabulary"
import { ErrorNote, Panel, StatusBadge } from "./page"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * The automatic reading of the base (`background-reading`): with it on,
 * the base reads by itself, with the model configured, every document
 * that is added or changes in the folders. It does not start by itself,
 * because it uses the model. It replaces the switch of the old wiki.
 */
export function BackgroundReadingPanel() {
  const reading = useDocbaseQuery("background-reading", getBackgroundReading)
  const llmConfig = useWikiStore((s) => s.llmConfig)
  const dialog = useAppDialog()
  const words = vocabulary()
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const on = reading.data === true

  // While it is on, the base asks with the model configured now; with no
  // model that can answer, nothing is asked.
  useEffect(() => {
    if (!on) return
    setAskEngine(getAskConfig()).catch((cause) => console.error("[docbase] failed to set the engine for questions:", cause))
  }, [on, llmConfig])

  async function toggle(next: boolean) {
    const confirmed = await dialog.confirm(
      next
        ? {
            title: "Activar la lectura automática",
            message:
              "Desde ahora la base lee sola lo que se agregue o cambie en las carpetas incorporadas: " +
              `el tipo de cada documento, sus ${words.knowledge.entities.toLowerCase()} y sus ${words.knowledge.matters.toLowerCase()}. ` +
              "Cada lectura pasa por el modelo configurado, así que consume cupo o costo de ese proveedor. " +
              "Las copias de un mismo documento se leen una sola vez.",
            confirmLabel: "Activar",
            cancelLabel: "Cancelar",
          }
        : {
            title: "Pausar la lectura automática",
            message:
              "No se manda nada más al modelo: lo nuevo queda sin leer hasta que la actives de nuevo. " +
              "Lo que ya se leyó queda como está.",
            confirmLabel: "Pausar",
            cancelLabel: "Cancelar",
          },
    )
    if (!confirmed) return
    setBusy(true)
    setProblem(null)
    try {
      if (next) await setAskEngine(getAskConfig())
      await setBackgroundReading(next)
      reading.reload()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  if (reading.data === null) return reading.error ? <ErrorNote>{reading.error}</ErrorNote> : null

  return (
    <Panel
      title={
        <>
          <BookOpen className="size-4" /> Lectura automática
          {on ? <StatusBadge tone="ok">Activa</StatusBadge> : <StatusBadge tone="neutral">En pausa</StatusBadge>}
        </>
      }
      action={
        on ? (
          <Button variant="ghost" size="sm" onClick={() => toggle(false)} disabled={busy}>
            <Pause /> Pausar
          </Button>
        ) : (
          <Button size="sm" onClick={() => toggle(true)} disabled={busy}>
            <Play /> Activar
          </Button>
        )
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        {problem && <ErrorNote>{problem}</ErrorNote>}
        <p className="text-muted-foreground">
          {on
            ? "La base lee sola, con el modelo configurado, lo que se agrega o cambia en las carpetas."
            : "La base no le manda nada al modelo por su cuenta. Activarla usa el modelo configurado, por eso no arranca sola."}
        </p>
      </div>
    </Panel>
  )
}
