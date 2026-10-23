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
          "Los escaneos y las fotos tienen el texto en una imagen. La base lo reconoce con el modelo, de a un documento, mientras la lectura automática está activa o cuando lo pedís; cada página es una pregunta al modelo.",
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
    case "templates":
      return {
        title: `${words.templates.name}: nombres de los campos`,
        explanation: `El modelo les pone nombre a los campos de cada ${words.templates.one} que su texto no nombra, y dice qué es cada uno (una persona, una fecha, un importe…). Una pregunta por ${words.templates.one}; lo que escribe una persona nunca se pisa.`,
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
  const labels: Record<StageState, string> = {
    done: "Hecha",
    running: "En curso",
    waiting: "Esperando",
    paused: "En pausa",
    error: "Con errores",
  }
  return labels[stage.state]
}

const UNITS: Record<Exclude<ProgressStage["unit"], "templates">, [string, string]> = {
  origins: ["carpeta", "carpetas"],
  files: ["archivo", "archivos"],
  documents: ["documento", "documentos"],
  pages: ["página", "páginas"],
}

/** "12 de 68 documentos"; "Ningún documento todavía" when it has nothing. */
export function stageFigures(stage: ProgressStage, words: Vocabulary = vocabulary()): string {
  // The templates are counted with the profession's word for them.
  if (stage.unit === "templates") {
    if (stage.total === 0) return words.templates.noneYet
    return `${formatCount(stage.done)} de ${plural(stage.total, words.templates.one, words.templates.many)}`
  }
  const [one, many] = UNITS[stage.unit]
  if (stage.total === 0) return `${stage.unit === "pages" || stage.unit === "origins" ? "Ninguna" : "Ningún"} ${one} todavía`
  return `${formatCount(stage.done)} de ${plural(stage.total, one, many)}`
}

/** Whether a stage waits for the engine's first pass to know what it has left (D-452). */
export function waitsFirstLap(stage: ProgressStage): boolean {
  return (stage.extra.firstLap ?? 0) > 0
}

/** What is left, failed or out of a stage: "faltan 56 · 2 fallaron". */
export function stageDetails(stage: ProgressStage, words: Vocabulary = vocabulary()): string[] {
  const out: string[] = []
  if (waitsFirstLap(stage)) {
    out.push("esperando la primera vuelta del motor para saber si queda algo por hacer")
  }
  if (stage.pending > 0) {
    out.push(
      stage.id === "recognition"
        ? `${formatCount(stage.pending)} ${stage.pending === 1 ? "espera" : "esperan"}`
        : `${stage.pending === 1 ? "falta" : "faltan"} ${formatCount(stage.pending)}`,
    )
  }
  if (stage.failed > 0) out.push(stage.failed === 1 ? "1 falló" : `${formatCount(stage.failed)} fallaron`)
  if (stage.id === "recognition") out.push(...scanPages(stage))
  if (stage.id === "matters") {
    const grouped = stage.extra.grouped ?? 0
    const loose = stage.extra.loose ?? 0
    if (grouped > 0) out.push(`${formatCount(grouped)} en un ${words.knowledge.matter.toLowerCase()}`)
    if (loose > 0) out.push(`${formatCount(loose)} ${loose === 1 ? "suelto" : "sueltos"}`)
  } else if (stage.out > 0) {
    out.push(stage.out === 1 ? "1 queda afuera" : `${formatCount(stage.out)} quedan afuera`)
  }
  if (stage.id === "reading") {
    const requested = stage.extra.requested ?? 0
    const outdated = stage.extra.outdated ?? 0
    if (requested > 0) out.push(plural(requested, "pedido para volver a leer", "pedidos para volver a leer"))
    if (outdated > 0) {
      out.push(
        plural(outdated, "leído con otra versión de la pregunta o con otro modelo", "leídos con otra versión de la pregunta o con otro modelo"),
      )
    }
  }
  const questions = stage.extra.questions ?? 0
  if (questions > 0 && stage.state !== "done") {
    out.push(plural(questions, "pregunta pendiente al modelo", "preguntas pendientes al modelo"))
  }
  if (stage.otherWork && stage.pending === 0 && stage.state !== "done" && !waitsFirstLap(stage)) out.push("rehaciendo con lo nuevo")
  return out
}

/**
 * The size of the recognition still to do (P-242): the pages that wait,
 * each a question to the model, and the documents whose pages are still
 * being counted.
 */
function scanPages(stage: ProgressStage): string[] {
  const pages = stage.extra.pages ?? 0
  const uncounted = stage.extra.uncounted ?? 0
  const out: string[] = []
  if (pages > 0) out.push(`${plural(pages, "página", "páginas")} por reconocer`)
  if (uncounted > 0) out.push(`contando las páginas de ${plural(uncounted, "documento", "documentos")}`)
  return out
}

/** "página 2 de 8" of the scan being recognized, when the engine said it. */
export function pageOf(item: { page?: number | null; pages?: number | null } | null): string | null {
  if (!item?.page) return null
  return item.pages ? `página ${formatCount(item.page)} de ${formatCount(item.pages)}` : `página ${formatCount(item.page)}`
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
  const working = progress.stages
  const failing = working.find((stage) => stage.state === "error" && stage.error)
  if (failing?.error) {
    const { what } = modelErrorText(failing.error)
    const retry = failing.error.retryAt ? ` Vuelve a intentar a las ${clockTime(failing.error.retryAt)}.` : ""
    return { tone: "error", text: `${stageText(failing.id, words).title}: ${what}${retry}` }
  }
  const running = working.find((stage) => stage.state === "running") ?? working.find((stage) => stage.state === "waiting")
  if (running) {
    const figures = running.total > 0 ? `, ${formatCount(running.done)} de ${formatCount(running.total)}` : ""
    const page = pageOf(running.current)
    return { tone: "busy", text: `Procesando: ${stageText(running.id, words).title}${figures}${page ? ` (${page})` : ""}` }
  }
  // The questions that go by themselves passed their cap (D-484).
  if (progress.autoQuestions?.held && working.some((stage) => (stage.extra.held ?? 0) > 0)) {
    return {
      tone: "paused",
      text: `Esperando tu permiso para ${plural(progress.autoQuestions.pending, "pregunta", "preguntas")} al modelo`,
    }
  }
  const paused = working.filter((stage) => stage.state === "paused")
  if (paused.length > 0) {
    const waiting = progress.paused > 0 ? `: ${plural(progress.paused, "documento espera", "documentos esperan")}` : ""
    // Only the scans wait, for want of an engine that reads them.
    if (paused.every((stage) => stage.id === "recognition") && !progress.recognitionEngine) {
      return { tone: "paused", text: `Falta elegir el motor que reconoce escaneos${waiting}` }
    }
    return {
      tone: "paused",
      text: progress.engineChosen ? `Lectura automática en pausa${waiting}` : `Falta elegir un modelo${waiting}`,
    }
  }
  return { tone: "ok", text: "Todo al día" }
}
