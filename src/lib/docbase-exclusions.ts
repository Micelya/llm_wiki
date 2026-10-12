import { excludedSources } from "@/commands/docbase"
import { normalizePath } from "@/lib/path-utils"

/**
 * Drops from an ingest batch what the user excluded in the document
 * base. Whoever asks for the ingest (the user, the watcher), an excluded
 * file never reaches the model. If the base cannot answer, nothing is
 * dropped: it is not there to block the wiki.
 */
export async function leaveOutExcludedSources<T extends { sourcePath: string }>(
  projectPath: string,
  files: T[],
): Promise<T[]> {
  if (files.length === 0) return files
  try {
    const excluded = new Set(
      (await excludedSources(normalizePath(projectPath), files.map((file) => normalizePath(file.sourcePath)))).map(
        (path) => path.toLowerCase(),
      ),
    )
    return excluded.size === 0
      ? files
      : files.filter((file) => !excluded.has(normalizePath(file.sourcePath).toLowerCase()))
  } catch {
    return files
  }
}
