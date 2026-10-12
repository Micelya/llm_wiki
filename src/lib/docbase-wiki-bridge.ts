/**
 * The wiki side of the bridge with the document base.
 *
 * The general project's wiki takes its sources from the base: every
 * origin is mounted in it. The wiki does not start by itself, because
 * ingesting uses the model: the user turns it on once, and from then on
 * it is fed by itself with whatever is added, changed or removed.
 */
import { listWikiSources, setWikiActive, type WikiSource } from "@/commands/docbase"
import { listIngestedSourceIdentities } from "@/lib/ingest-cache"
import { getQueue } from "@/lib/ingest-queue"
import { normalizePath } from "@/lib/path-utils"
import { loadSourceWatchConfig, saveSourceWatchConfig } from "@/lib/project-store"
import { enqueueSourceIngest, isIngestableSourcePath } from "@/lib/source-lifecycle"
import { normalizeSourceWatchConfig } from "@/lib/source-watch-config"
import { useWikiStore } from "@/stores/wiki-store"
import type { WikiProject } from "@/types/wiki"

/** Whether two project folders are the same one, the way Windows compares paths. */
export function isSameProjectPath(a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b) return false
  const clean = (path: string) => normalizePath(path).replace(/\/+$/, "").toLowerCase()
  return clean(a) === clean(b)
}

/**
 * The sources of the base the wiki has not ingested and is not about to:
 * ingestable files that are neither in the ingest record nor waiting in
 * the queue.
 */
export function selectPendingSources(
  sources: WikiSource[],
  ingestedIdentities: string[],
  queuedSourcePaths: string[],
): WikiSource[] {
  const ingested = new Set(ingestedIdentities.map((identity) => normalizePath(identity).toLowerCase()))
  const queued = new Set(queuedSourcePaths.map((path) => normalizePath(path).toLowerCase()))
  return sources.filter(
    (source) =>
      isIngestableSourcePath(source.sourcePath) &&
      !ingested.has(source.identity.toLowerCase()) &&
      !queued.has(source.sourcePath.toLowerCase()),
  )
}

/** Pending sources of the general project's wiki, by origin id. */
export async function pendingWikiSourcesByOrigin(project: WikiProject): Promise<Map<string, WikiSource[]>> {
  const [sources, ingested] = await Promise.all([
    listWikiSources(),
    listIngestedSourceIdentities(project.path),
  ])
  const queued = getQueue()
    .filter((task) => task.projectId === project.id)
    .filter((task) => task.status === "pending" || task.status === "processing")
    .map((task) => task.sourcePath)
  const byOrigin = new Map<string, WikiSource[]>()
  for (const source of selectPendingSources(sources, ingested, queued)) {
    const list = byOrigin.get(source.originId) ?? []
    list.push(source)
    byOrigin.set(source.originId, list)
  }
  return byOrigin
}

/**
 * Turns the wiki of the general project on or off. `project` is that
 * project, which has to be the open one.
 *
 * On: the project's watcher starts ingesting what appears or changes in
 * the origins, and what is already there and was never ingested is
 * queued now. Off: nothing more is sent to the model; pages stay.
 * Returns how many documents were queued.
 */
export async function setWikiFeeding(project: WikiProject, active: boolean): Promise<number> {
  const { startProjectFileSync, stopProjectFileSync } = await import("@/lib/project-file-sync")
  await stopProjectFileSync()
  const config = normalizeSourceWatchConfig({
    ...(await loadSourceWatchConfig(project.id)),
    autoIngest: active,
  })
  await saveSourceWatchConfig(config, project.id)
  useWikiStore.getState().setSourceWatchConfig(config)
  await setWikiActive(active)
  // Restarting applies the setting and covers folders mounted since.
  if (config.enabled) {
    await startProjectFileSync(project, config).catch((error) =>
      console.error("[docbase] failed to restart the project file watcher:", error),
    )
  }
  if (!active) return 0
  const pending = [...(await pendingWikiSourcesByOrigin(project)).values()].flat()
  return pending.length > 0 ? ingestWikiSources(project, pending) : 0
}

/** Queues the given sources for ingest. Returns how many were queued. */
export async function ingestWikiSources(project: WikiProject, sources: WikiSource[]): Promise<number> {
  const queued = await enqueueSourceIngest(
    project,
    sources.map((source) => source.sourcePath),
    useWikiStore.getState().llmConfig,
  )
  return queued.length
}
