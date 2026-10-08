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
import { listen } from "@tauri-apps/api/event"
import { preprocessFile } from "@/commands/fs"
import { getFileName } from "@/lib/path-utils"
import { useWikiStore } from "@/stores/wiki-store"

export interface RecognitionConfig {
  engine: "codex-cli"
  model: string
}

/** Emitted by the Rust side before each page is handed to the engine. */
const PROGRESS_EVENT = "text-recognition://progress"

interface RecognitionProgress {
  path: string
  page: number
  total: number
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
 *
 * This is the only place recognition is started. It runs inside the
 * ingest task of each source, never while files are being imported, so
 * importing stays quick and the wait shows up against the right file.
 * `onStage` receives a short description of what is happening,
 * page by page for a scanned PDF.
 */
export async function ensureRecognizedText(
  sourcePath: string,
  onStage?: (stage: string) => void,
): Promise<void> {
  const config = getRecognitionConfig()
  if (!config || !isRecognizableSource(sourcePath)) return
  onStage?.("Recognizing text...")
  const stopListening = onStage
    ? await listen<RecognitionProgress>(PROGRESS_EVENT, ({ payload }) => {
        if (payload.path !== sourcePath) return
        onStage(`Recognizing text: page ${payload.page} of ${payload.total}`)
      }).catch(() => null)
    : null
  try {
    await preprocessFile(sourcePath, config)
  } finally {
    stopListening?.()
  }
}
