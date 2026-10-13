/**
 * The project the app opens by itself: the general project of the
 * space, kept in the folder `default` of the app's workspace.
 */
import { documentDir, homeDir, join } from "@tauri-apps/api/path"
import { ensureDefaultProject as ensureDefaultProjectCommand, getGeneralProject } from "@/commands/docbase"
import { fileExists, openProject } from "@/commands/fs"
import { normalizePath } from "@/lib/path-utils"
import {
  loadOutputLanguage,
  loadSourceWatchConfig,
  loadWorkspaceDir,
  saveOutputLanguage,
  saveSourceWatchConfig,
} from "@/lib/project-store"
import type { OutputLanguage } from "@/lib/output-language-options"
import { vocabulary } from "@/vocabulary"
import { normalizeSourceWatchConfig } from "@/lib/source-watch-config"
import type { WikiProject } from "@/types/wiki"

/** Folder name earlier versions of the app led users to create by hand. */
const LEGACY_WORKSPACE = "llm-wiki-workspace"

/** A sensible workspace to offer the first time. */
export async function suggestWorkspaceDir(): Promise<string> {
  const legacy = await join(await homeDir(), LEGACY_WORKSPACE)
  if (await fileExists(legacy).catch(() => false)) return normalizePath(legacy)
  return normalizePath(await join(await documentDir(), "Micelya"))
}

/**
 * Opens the default project, creating it the first time. Returns `null`
 * when the workspace is not chosen yet.
 *
 * While the wiki is not active the project never ingests on its own:
 * its sources are whole folders, and sending them to the model has to
 * be something the user asked for.
 */
export async function openDefaultProject(workspaceDir?: string | null): Promise<WikiProject | null> {
  const workspace = workspaceDir ?? (await loadWorkspaceDir())
  if (!workspace) return null
  const general = await ensureDefaultProjectCommand(workspace)
  if (!general.path) return null
  const project = await openProject(general.path)
  await syncAutoIngest(project.id, general.wikiActive)
  await ensureWikiLanguage(project.id)
  return project
}

/**
 * Gives the default project a language for its wiki when it has none.
 *
 * A project made by hand gets one in its creation dialog; this one is
 * made by the app. Without it the language is guessed from each
 * document, so a firm with documents in several languages would get a
 * wiki in several languages. The app's own preference is used, or the
 * language of the vocabulary.
 */
export async function ensureWikiLanguage(projectId: string): Promise<void> {
  if (await loadOutputLanguage(projectId)) return
  const general = await loadOutputLanguage()
  const language = general && general !== "auto" ? general : (vocabulary().wikiLanguage as OutputLanguage)
  // Only the project's own entry: the app-wide preference stays as it is.
  await saveOutputLanguage(language, projectId, { projectOnly: true })
}

/**
 * Keeps the queue of the default project still while its wiki is off.
 * A restored queue resumes by itself; for this project that would send
 * documents to the model while the screen says the wiki is paused.
 */
export async function holdQueueWhileWikiIsOff(project: WikiProject): Promise<void> {
  const general = await getGeneralProject()
  const clean = (path: string) => normalizePath(path).replace(/\/+$/, "").toLowerCase()
  if (!general?.path || clean(general.path) !== clean(project.path) || general.wikiActive) return
  const { pauseProcessing } = await import("@/lib/ingest-queue")
  pauseProcessing()
}

/** Whether the open project is the default one. */
export async function isDefaultProject(project: WikiProject | null): Promise<boolean> {
  if (!project) return false
  const general = await getGeneralProject()
  const clean = (path: string) => normalizePath(path).replace(/\/+$/, "").toLowerCase()
  return !!general?.path && clean(general.path) === clean(project.path)
}

/** Makes the project's own watcher ingest new files only while the wiki is active. */
export async function syncAutoIngest(projectId: string, wikiActive: boolean): Promise<void> {
  const config = normalizeSourceWatchConfig(await loadSourceWatchConfig(projectId))
  if (config.autoIngest === wikiActive) return
  await saveSourceWatchConfig({ ...config, autoIngest: wikiActive }, projectId)
}
