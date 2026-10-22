import { useState } from "react"
import { BookOpen, Pause, Play } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getBackgroundReading, setBackgroundReading, type Progress } from "@/commands/docbase"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useProgressStore } from "@/stores/progress-store"
import { vocabulary } from "@/vocabulary"
import { ErrorNote, Panel, StatusBadge } from "./page"
import { plural } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

export interface BackgroundReading {
  /** `null` while it is not known yet. */
  on: boolean | null
  error: string | null
  busy: boolean
  problem: string | null
  /** Asks for a confirmation that says what it means, then turns it on or off. */
  toggle: (next: boolean) => Promise<void>
}

/**
 * The automatic reading of the base (`background-reading`): with it on,
 * the base reads by itself, with the engines of Ajustes → Servicios de IA
 * (the base reads them from its registry at every request, D-443), every
 * document that is added or changes in the folders. It does not start by
 * itself, because it uses the model. The one switch, shared by Orígenes
 * and «En curso» (D-424).
 */
export function useBackgroundReading(): BackgroundReading {
  const reading = useDocbaseQuery("background-reading", getBackgroundReading)
  const refreshProgress = useProgressStore((s) => s.refresh)
  const progress = useProgressStore((s) => s.progress)
  const dialog = useAppDialog()
  const words = vocabulary()
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  async function toggle(next: boolean) {
    const confirmed = await dialog.confirm(
      next
        ? {
            title: "Activar la lectura automática",
            message:
              "Desde ahora la base lee sola lo que se agregue o cambie en las carpetas incorporadas: " +
              `el texto de los escaneos, el tipo de cada documento, sus ${words.knowledge.entities.toLowerCase()} y sus ${words.knowledge.matters.toLowerCase()}. ` +
              "Cada lectura pasa por el modelo configurado, así que consume cupo o costo de ese proveedor. " +
              scansWaiting(progress) +
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
      await setBackgroundReading(next)
      reading.reload()
      refreshProgress()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  return { on: reading.data, error: reading.error, busy, problem, toggle }
}

/** How many scans wait for recognition and their pages, to say before turning the reading on (P-242). */
export function scansWaiting(progress: Progress | null): string {
  const scans = progress?.stages.find((stage) => stage.id === "recognition")
  const documents = (scans?.pending ?? 0) + (scans?.failed ?? 0)
  if (documents === 0) return ""
  const pages = scans?.extra.pages ?? 0
  const size = pages > 0 ? ` (${plural(pages, "página", "páginas")}, una pregunta al modelo por página)` : ""
  return `Hay ${plural(documents, "escaneo", "escaneos")} por reconocer${size}. `
}

/** The button that pauses or turns on the automatic reading. */
export function BackgroundReadingButton({ reading }: { reading: BackgroundReading }) {
  return reading.on ? (
    <Button variant="ghost" size="sm" onClick={() => void reading.toggle(false)} disabled={reading.busy}>
      <Pause /> Pausar
    </Button>
  ) : (
    <Button size="sm" onClick={() => void reading.toggle(true)} disabled={reading.busy}>
      <Play /> Activar
    </Button>
  )
}

/** «Activa» or «En pausa». */
export function BackgroundReadingBadge({ on }: { on: boolean }) {
  return on ? <StatusBadge tone="ok">Activa</StatusBadge> : <StatusBadge tone="neutral">En pausa</StatusBadge>
}

export function BackgroundReadingPanel() {
  const reading = useBackgroundReading()
  const on = reading.on === true

  if (reading.on === null) return reading.error ? <ErrorNote>{reading.error}</ErrorNote> : null

  return (
    <Panel
      title={
        <>
          <BookOpen className="size-4" /> Lectura automática
          <BackgroundReadingBadge on={on} />
        </>
      }
      action={<BackgroundReadingButton reading={reading} />}
    >
      <div className="flex flex-col gap-3 text-sm">
        {reading.problem && <ErrorNote>{reading.problem}</ErrorNote>}
        <p className="text-muted-foreground">
          {on
            ? "La base lee sola, con el modelo configurado, lo que se agrega o cambia en las carpetas."
            : "La base no le manda nada al modelo por su cuenta. Activarla usa el modelo configurado, por eso no arranca sola."}
        </p>
      </div>
    </Panel>
  )
}
