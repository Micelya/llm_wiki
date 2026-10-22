/**
 * Text recognition of scans and photos (P-242): which engine reads them,
 * and how what it read, or why it could not, is told on screen.
 *
 * The reading happens in the base (Rust, `docbase/recognize.rs`), behind
 * an engine-neutral contract (`TextRecognizer`); this module only says
 * which engine is chosen, so callers never name one. It is the engine of
 * «Reconocer escaneos» in the registry of AI services (Ajustes → Servicios
 * de IA, D-443), when the base can run it: today Codex or Micelya Core.
 */
import type { RecognitionConfig } from "@/commands/docbase"
import { recognitionConfigOfServices } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"

/** The engine that recognizes scans; null when none the base can run is chosen. */
export function getRecognitionConfig(): RecognitionConfig | null {
  return recognitionConfigOfServices(useAiServicesStore.getState().services)
}

/** The name the base saves the text of that engine with (as `recognition::engine_id` in Rust). */
export function recognitionEngineId(config: RecognitionConfig | null): string | null {
  if (!config) return null
  const model = config.model.trim()
  return model ? `${config.engine}:${model}` : config.engine
}

/** What a stage or a document says when no engine can recognize scans. */
export const MISSING_ENGINE =
  "Falta elegir el motor que reconoce escaneos: en Ajustes → Servicios de IA, elegí uno para «Reconocer escaneos»."

/** What it costs, in words. */
export const RECOGNITION_COST = "Cada página es una pregunta al modelo: consume cupo o costo de ese proveedor."

const ENGINE_NAMES: Record<string, string> = { "codex-cli": "Codex", core: "Micelya Core" }

/** Whether a text was produced by a recognition engine, not by plain extraction. */
export function isRecognitionEngine(engine: string | null | undefined): engine is string {
  return !!engine && engine !== "extraction"
}

/** "Codex (gpt-6-luna)" for "codex-cli:gpt-6-luna"; "Codex" for "codex-cli". */
export function engineName(engineId: string): string {
  const [engine, ...rest] = engineId.split(":")
  const model = rest.join(":")
  const name = ENGINE_NAMES[engine] ?? engine
  return model ? `${name} (${model})` : name
}

/** Why a recognition failed, in words, from what the engine or the base said. */
export function recognitionFailureText(message: string): string {
  const said = message.toLowerCase()
  const page = /\bpage (\d+)/.exec(said)
  const where = page ? ` (página ${page[1]})` : ""
  if (said.includes("no connection with micelya core")) {
    return "Falta la conexión con Micelya Core: cargala en Ajustes → Conexiones."
  }
  if (said.includes("rejected the credential")) return "Micelya Core rechazó la credencial: revisala en Ajustes → Conexiones."
  const core = /micelya core could not read the page: ([\s\S]+)$/i.exec(message)
  if (core) return `Micelya Core no pudo leer ${page ? `la página ${page[1]}` : "la página"}: ${core[1].trim()}`
  if (said.includes("could not be deleted in micelya core")) {
    return `Micelya Core leyó la página pero no se pudo borrar su lectura allí${where}: se vuelve a intentar más tarde, sin pagarla de nuevo.`
  }
  if (said.includes("cannot be reached: its folder")) return "No se encuentra el archivo: revisá que su carpeta esté conectada."
  if (/usage limit|hit your limit|rate limit|429|quota|too many requests/.test(said)) {
    return `El proveedor del modelo llegó a su límite de uso${where}.`
  }
  if (/not found on path|failed to start/.test(said)) return "No se encontró el programa del motor (Codex) en esta computadora."
  if (/did not answer within|timed out|timeout|connection|network|offline/.test(said)) {
    return `El motor no respondió a tiempo, o no hay conexión${where}.`
  }
  if (/unauthorized|\b401\b|\b403\b|api key|not logged in|log in|login/.test(said)) return "El proveedor del modelo rechazó la clave o la sesión."
  if (/failed to open pdf|could not be rendered|could not be converted|could not be saved|not found:/.test(said)) {
    return `No se pudo convertir la página en imagen: el PDF puede estar dañado o protegido${where}.`
  }
  if (/empty answer|did not write an answer/.test(said)) return `El motor no devolvió texto${where}.`
  if (said.includes("not an image whose text")) return "Este formato no es una imagen que se pueda reconocer."
  return `El motor que reconoce el texto falló${where}.`
}

/**
 * The parts the engine said it could not read: Codex writes «[ilegible]»
 * for each character it cannot tell apart. The pages that have one (by
 * their «## Page N» section); `pages` is empty for a photo, which has no
 * pages. Null when it read everything.
 */
export function illegibleParts(text: string | null): { pages: number[] } | null {
  if (!text || !text.includes("[ilegible]")) return null
  const parts = text.split(/^## Page (\d+)[ \t]*$/m)
  const pages: number[] = []
  for (let index = 1; index + 1 < parts.length; index += 2) {
    if (parts[index + 1].includes("[ilegible]")) pages.push(Number(parts[index]))
  }
  return { pages }
}

/** "páginas 2, 5 y 7", "página 3". */
export function pageList(pages: number[]): string {
  if (pages.length === 1) return `página ${pages[0]}`
  return `páginas ${pages.slice(0, -1).join(", ")} y ${pages[pages.length - 1]}`
}

const MARKER = /^<!-- texto-reconocido motor="[^"]*" -->[ \t]*\r?\n?/gm

/** The text without the marks that say which engine read each page (the screen says it once). */
export function withoutRecognitionMarkers(text: string): string {
  return text.replace(MARKER, "")
}
