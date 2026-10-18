import { runStructuralLint, type LintResult } from "@/lib/lint"
import { loadLintConfig } from "@/lib/lint-config"
import { normalizePath } from "@/lib/path-utils"
import { useLintStore, type LintItem } from "@/stores/lint-store"
import { useWikiStore } from "@/stores/wiki-store"

/**
 * The structural review of the wiki's links, run by itself every time the
 * wiki's queue empties, so the broken links the attention view counts
 * are always the ones the link review shows. Only the structural part:
 * it reads the pages and needs no model. What the model found on a run
 * started by a person (`semantic`) stays as it was.
 */
export async function refreshStructuralLint(projectPath: string, signal?: AbortSignal): Promise<number | null> {
  const path = normalizePath(projectPath)
  const config = await loadLintConfig(path)
  let results: LintResult[]
  try {
    results = await runStructuralLint(path, { signal, config })
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return null
    throw error
  }
  // The project may have changed while the pages were read.
  const open = useWikiStore.getState().project
  if (signal?.aborted || !open || normalizePath(open.path) !== path) return null
  replaceStructuralItems(results)
  return results.length
}

/** Puts these structural results in place of the previous ones, keeping what the model found. */
export function replaceStructuralItems(results: LintResult[]): void {
  const store = useLintStore.getState()
  const kept: LintItem[] = store.items.filter((item) => item.type === "semantic")
  store.setItems(kept)
  store.addItems(results)
}
