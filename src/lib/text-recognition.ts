/**
 * Text recognition for sources whose text is only pixels: scanned PDF
 * pages and photos of documents.
 *
 * The reading itself happens in the Rust `preprocess_file` command,
 * behind an engine-neutral contract; this module only decides whether
 * recognition is switched on and with which engine, so callers never
 * name an engine themselves.
 *
 * For now it is switched on by the existing "image captioning" setting
 * when that setting uses the main model and the main model is the Codex
 * CLI — the one provider that cannot caption images through the regular
 * chat path but can read them through `codex exec`.
 */
import { preprocessFile } from "@/commands/fs"
import { getFileName } from "@/lib/path-utils"
import { useWikiStore } from "@/stores/wiki-store"

export interface RecognitionConfig {
  engine: "codex-cli"
  model: string
}

const RECOGNIZABLE_EXTENSIONS = new Set(["pdf", "png", "jpg", "jpeg"])

export function getRecognitionConfig(): RecognitionConfig | null {
  const { multimodalConfig, llmConfig } = useWikiStore.getState()
  if (!multimodalConfig.enabled || !multimodalConfig.useMainLlm) return null
  if (llmConfig.provider !== "codex-cli") return null
  return { engine: "codex-cli", model: llmConfig.model }
}

export function isRecognizableSource(sourcePath: string): boolean {
  const ext = getFileName(sourcePath).split(".").pop()?.toLowerCase() ?? ""
  return RECOGNIZABLE_EXTENSIONS.has(ext)
}

/**
 * Make sure a recognizable source has had its pixels read before its
 * text is used. Errors propagate on purpose: carrying on would ingest a
 * scanned page as if it were blank and record that as a success.
 */
export async function ensureRecognizedText(sourcePath: string): Promise<void> {
  const config = getRecognitionConfig()
  if (!config || !isRecognizableSource(sourcePath)) return
  await preprocessFile(sourcePath, config)
}
