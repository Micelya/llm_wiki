import type { ModelError, Progress, ProgressStage, StageId, StageState } from "@/commands/docbase"
import { formatCount, plural } from "@/components/docbase/format"
import { vocabulary, type Vocabulary } from "@/vocabulary"

/**
 * How «En curso» tells the stages of the processing (D-424): what each
 * one is called and does, how it stands, its figures and the last error
 * of the model, in words; and the one line at the foot of the window.
 * The figures come from the base (`docbase_progress`); the domain words,
 * from the vocabulary of the vertical.
 */

export interface StageText {
  title: string
  explanation: string
}

/** What a stage is called and what it does. */
export function stageText(id: StageId, words: Vocabulary = vocabulary()): StageText {
  const terms = words.knowledge
  const who = terms.entities.toLowerCase()
  switch (id) {
    case "origins":
      return {
        title: "Recorrido de las carpetas",
        explanation:
          "La app recorre las carpetas incorporadas para ver qué archivos hay, cuáles cambiaron y cuáles ya no están. Lo hace al abrirse y cada pocos minutos.",
      }
    case "text":
      return {
        title: "Texto de cada documento",
        explanation: "Identifica cada archivo, encuentra sus copias y obtiene su texto, una sola vez por documento.",
      }
    case "recognition":
      return {
        title: "Texto de los escaneos",
        explanation:
          "Los escaneos y las fotos tienen el texto en una imagen y hay que reconocerlo. La base todavía no lo hace sola: esperan reconocimiento.",
      }
    case "similar":
      return {
        title: "Copias, versiones y plantillas",
        explanation: "Compara cada documento con los demás para encontrar copias, versiones y documentos de la misma plantilla.",
      }
    case "types":
      return {
        title: "Tipo y datos de cada documento",
        explanation: "El modelo dice qué es cada documento, en qué etapa está y cuáles son sus datos principales.",
      }
    case "reading":
      return {
        title: "Lectura con el modelo: a quiénes nombran",
        explanation: `El modelo lee cada documento una vez y dice qué ${who} nombra, con qué rol y qué dice de cada uno, con la cita que lo respalda.`,
      }
    case "resolve":
      return {
        title: "Quién es quién",
        explanation:
          "Junta los nombres de cada documento con los de los demás: por su identificador, por su nombre y, cuando hace falta, preguntándole al modelo.",
      }
    case "matters":
      return {
        title: terms.matters,
        explanation: `Agrupa los documentos de un mismo ${terms.matter.toLowerCase()} por lo que comparten; para los que quedan sueltos, el modelo propone a cuál pertenecen.`,
      }
    case "pages":
      return {
        title: "Páginas (artículos)",
        explanation: `Arma la página de cada documento, de las ${who} y de los ${terms.matters.toLowerCase()}; el modelo escribe lo que dicen los documentos.`,
      }
  }
}

/** How a stage stands, in one or two words. */
export function stateLabel(stage: Pick<ProgressStage, "id" | "state">): string {
  if (stage.id === "recognition" && stage.state === "waiting") return "Esperan reconocimiento"
  const labels: Record<StageState, string> = {
    done: "Hecha",
    running: "En curso",
    waiting: "Esperando",
    paused: "En pausa",
    error: "Con errores",
  }
  return labels[stage.state]
}

const UNITS: Record<ProgressStage["unit"], [string, string]> = {
  origins: ["carpeta", "carpetas"],
  files: ["archivo", "archivos"],
  documents: ["documento", "documentos"],
  pages: ["página", "páginas"],
}

/** "12 de 68 documentos"; "Ningún documento todavía" when it has nothing. */
export function stageFigures(stage: ProgressStage): string {
  const [one, many] = UNITS[stage.unit]
  if (stage.total === 0) return `${stage.unit === "pages" || stage.unit === "origins" ? "Ninguna" : "Ningún"} ${one} todavía`
  return `${formatCount(stage.done)} de ${plural(stage.total, one, many)}`
}

/** What is left, failed or out of a stage: "faltan 56 · 2 fallaron". */
export function stageDetails(stage: ProgressStage, words: Vocabulary = vocabulary()): string[] {
  const out: string[] = []
  if (stage.pending > 0) {
    out.push(stage.id === "recognition" ? `${formatCount(stage.pending)} esperan` : `${stage.pending === 1 ? "falta" : "faltan"} ${formatCount(stage.pending)}`)
  }
  if (stage.failed > 0) out.push(stage.failed === 1 ? "1 falló" : `${formatCount(stage.failed)} fallaron`)
  if (stage.id === "matters") {
    const grouped = stage.extra.grouped ?? 0
    const loose = stage.extra.loose ?? 0
    if (grouped > 0) out.push(`${formatCount(grouped)} en un ${words.knowledge.matter.toLowerCase()}`)
    if (loose > 0) out.push(`${formatCount(loose)} ${loose === 1 ? "suelto" : "sueltos"}`)
  } else if (stage.out > 0) {
    out.push(stage.out === 1 ? "1 queda afuera" : `${formatCount(stage.out)} quedan afuera`)
  }
  const questions = stage.extra.questions ?? 0
  if (questions > 0 && stage.state !== "done") {
    out.push(plural(questions, "pregunta pendiente al modelo", "preguntas pendientes al modelo"))
  }
  if (stage.otherWork && stage.pending === 0 && stage.state !== "done") out.push("rehaciendo con lo nuevo")
  return out
}

/** Share of the stage that is done, 0 to 1; null when it has nothing. */
export function stageShare(stage: ProgressStage): number | null {
  return stage.total > 0 ? Math.min(1, stage.done / stage.total) : null
}

/** "14:05". */
export function clockTime(ms: number): string {
  return new Date(ms).toLocaleTimeString("es-AR", { hour: "2-digit", minute: "2-digit", hourCycle: "h23" })
}

/** The last error of the model, in words: what happened, when and when it tries again. */
export function modelErrorText(error: ModelError): { what: string; when: string } {
  const what: Record<ModelError["kind"], string> = {
    limit: "El proveedor del modelo llegó a su límite de uso.",
    key: "El proveedor del modelo rechazó la clave o la sesión.",
    connection: "No hay conexión con el proveedor del modelo, o no respondió a tiempo.",
    other: "El modelo falló.",
  }
  const when = [`Pasó a las ${clockTime(error.at)}.`]
  if (error.retryAt) when.push(`Vuelve a intentar a las ${clockTime(error.retryAt)}.`)
  return { what: what[error.kind] ?? what.other, when: when.join(" ") }
}

/** How many elements the tab «En curso» has: the stages not done. */
export function ongoingCount(progress: Progress | null): number {
  return progress ? progress.stages.filter((stage) => stage.state !== "done").length : 0
}

export type ActivityTone = "busy" | "error" | "paused" | "ok"

/**
 * The one line at the foot of the window: what is being processed now and
 * how far it went, or why it stopped, or that everything is up to date.
 * `null` while there is nothing to tell (no folder incorporated yet).
 */
export function activityLine(progress: Progress | null, words: Vocabulary = vocabulary()): { tone: ActivityTone; text: string } | null {
  if (!progress) return null
  const origins = progress.stages.find((stage) => stage.id === "origins")
  if (!origins || origins.total === 0) return null
  const working = progress.stages.filter((stage) => stage.id !== "recognition")
  const failing = working.find((stage) => stage.state === "error" && stage.error)
  if (failing?.error) {
    const { what } = modelErrorText(failing.error)
    const retry = failing.error.retryAt ? ` Vuelve a intentar a las ${clockTime(failing.error.retryAt)}.` : ""
    return { tone: "error", text: `${stageText(failing.id, words).title}: ${what}${retry}` }
  }
  const running = working.find((stage) => stage.state === "running") ?? working.find((stage) => stage.state === "waiting")
  if (running) {
    const figures = running.total > 0 ? `, ${formatCount(running.done)} de ${formatCount(running.total)}` : ""
    return { tone: "busy", text: `Procesando: ${stageText(running.id, words).title}${figures}` }
  }
  if (working.some((stage) => stage.state === "paused")) {
    const waiting = progress.paused > 0 ? `: ${plural(progress.paused, "documento espera", "documentos esperan")}` : ""
    return {
      tone: "paused",
      text: progress.engineChosen ? `Lectura automática en pausa${waiting}` : `Falta elegir un modelo${waiting}`,
    }
  }
  return { tone: "ok", text: "Todo al día" }
}
