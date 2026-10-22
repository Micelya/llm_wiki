/**
 * Text recognition of scans and photos (P-242): which engine reads them,
 * and how what it read, or why it could not, is told on screen.
 *
 * The reading happens in the base (Rust, `docbase/recognize.rs`), behind
 * an engine-neutral contract (`TextRecognizer`); this module only says
 * which engine the configured model is, so callers never name one. The
 * retired ingest took it from the image-description setting when it used
 * the main model and that model was the Codex CLI; that setting no longer
 * exists, so it is the main model alone (D-429). Today only the Codex CLI
 * can read images from Rust.
 */
import type { RecognitionConfig } from "@/commands/docbase"
import { useWikiStore } from "@/stores/wiki-store"

/** The engine that recognizes scans with the model chosen now; null when that model cannot. */
export function getRecognitionConfig(): RecognitionConfig | null {
  return recognitionConfigOf(useWikiStore.getState().llmConfig)
}

export function recognitionConfigOf(llmConfig: { provider: string; model: string }): RecognitionConfig | null {
  if (llmConfig.provider !== "codex-cli") return null
  return { engine: "codex-cli", model: llmConfig.model }
}

/** The name the base saves the text of that engine with (as `recognition::engine_id` in Rust). */
export function recognitionEngineId(config: RecognitionConfig | null): string | null {
  if (!config) return null
  const model = config.model.trim()
  return model ? `${config.engine}:${model}` : config.engine
}

/** What a stage or a document says when no engine can recognize scans. */
export const MISSING_ENGINE =
  "Falta elegir el motor que reconoce escaneos: en Ajustes → LLM Models, elegí «Codex CLI (local)»."

/** What it costs, in words. */
export const RECOGNITION_COST = "Cada página es una pregunta al modelo: consume cupo o costo de ese proveedor."

const ENGINE_NAMES: Record<string, string> = { "codex-cli": "Codex" }

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
  if (said.includes("cannot be reached")) return "No se encuentra el archivo: revisá que su carpeta esté conectada."
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
