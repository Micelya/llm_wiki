import type { ProcessView, StepState } from "@/commands/docbase"
import { formatCount, plural } from "@/components/docbase/format"

/**
 * «En curso» with the 18 steps of the process (A3 of 2 nov; point 8 of the
 * redesign): the line of how the loop stands, why it went back the last
 * time, and each step in words. Pure: the screen draws what these say.
 */

export type StepMark = "complete" | "working" | "pending" | "aside" | "person" | "unknown"

export interface StepLine {
  mark: StepMark
  /** How it stands, in words: «completo», «trabajando · 3 pendientes»… */
  text: string
  /** Something of it waits in Atención: a failure, a person, a type of low confidence. */
  attention: boolean
}

/** The line above: the state of the loop and what it does or waits for. */
export function statusLine(view: ProcessView): { state: string; detail: string } {
  const status = view.status
  switch (status.state) {
    case "working":
      return {
        state: "Procesando",
        detail: [status.operation, status.scope && status.scope !== "todo" ? status.scope : null].filter(Boolean).join(" · "),
      }
    case "stopped": {
      // What waits for a setting (the automatic reading off, no engine) is
      // not «todo completo»: it is said.
      const held = waitsFor(view)
      return {
        state: "Detenido",
        detail: held
          ? `lo que se hace sin IA está completo · lo demás espera: ${held}`
          : "todo completo · sigue solo cuando aparezca un archivo nuevo o si pedís volver a procesar",
      }
    }
    case "waiting-person":
      return { state: "Esperando a una persona", detail: status.waitingFor ?? "" }
    case "waiting":
      return { state: "Esperando", detail: status.waitingFor ?? waitsFor(view) ?? "un reintento o el motor" }
    default:
      return { state: "Empezando", detail: "el proceso todavía no hizo su primera vuelta" }
  }
}

/** What the steps wait for when the loop says only that it waits. */
function waitsFor(view: ProcessView): string | null {
  return view.state.steps.find((step) => step.held && step.count.setAside > 0)?.held ?? null
}

/** Why it went back to an earlier step the last time, if it did. */
export function backLine(view: ProcessView): string | null {
  return view.status.back ? `La última vez ${view.status.back}.` : null
}

/** One step, in words. */
export function stepLine(step: StepState, view: ProcessView): StepLine {
  const count = step.count
  const reason = view.aside[String(step.about.number)] ?? null
  const working = view.status.state === "working" && view.status.step === step.about.number
  const parts: string[] = []
  if (count.pending > 0) parts.push(plural(count.pending, "pendiente", "pendientes"))
  const failed = Math.max(0, count.setAside - count.person - count.low)
  const byPerson = count.person + (reason?.includes("persona") ? failed : 0)
  if (byPerson > 0) parts.push(`${formatCount(byPerson)} esperando a una persona`)
  if (count.low > 0) parts.push(`${formatCount(count.low)} ${count.low === 1 ? "apartado" : "apartados"} por confianza baja`)
  const rest = reason?.includes("persona") ? 0 : failed
  if (rest > 0) {
    const why = step.held ?? reason
    parts.push(
      why && why !== "apartado por ahora"
        ? `${formatCount(rest)} esperando: ${why}`
        : `${formatCount(rest)} ${rest === 1 ? "apartado" : "apartados"} por falla`,
    )
  }
  const attention = byPerson > 0 || count.low > 0 || (rest > 0 && !step.held)
  if (working) return { mark: "working", text: ["trabajando", ...parts].join(" · "), attention }
  if (count.pending > 0) return { mark: "pending", text: parts.join(" · "), attention }
  if (byPerson > 0) return { mark: "person", text: parts.join(" · "), attention }
  if (count.setAside > 0) return { mark: "aside", text: parts.join(" · "), attention }
  if (!count.known) return { mark: "unknown", text: "completo hasta ahora · se revisa al final de cada vuelta", attention }
  return { mark: "complete", text: "completo", attention }
}

/** The two groups, «Cada archivo» and «Cada documento», with what they count. */
export function stepGroups(view: ProcessView): { title: string; counts: string; steps: StepState[] }[] {
  const totals = view.totals
  return [
    {
      title: "Cada archivo",
      counts: `${plural(totals.files, "archivo distinto", "archivos distintos")} en ${plural(totals.places, "lugar", "lugares")}`,
      steps: view.state.steps.filter((step) => step.about.level === "file"),
    },
    {
      title: "Cada documento",
      counts: `${plural(totals.documents, "documento", "documentos")} (cada parte de un archivo compuesto cuenta como uno)`,
      steps: view.state.steps.filter((step) => step.about.level === "document"),
    },
  ]
}
