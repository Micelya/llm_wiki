/**
 * The wiki side of the knowledge layer (D-200).
 *
 * In the archive's wiki (the general project) the pages of people,
 * organizations and matters are a projection of the layer's record: the
 * base writes them, with the block of entities at the end of each
 * document's page. Ingest keeps writing the page of each document and the
 * concepts, and no longer writes entity pages there. Every other project
 * works as before.
 */
import { getGeneralProject, onKnowledgePages, setWikiLanguage } from "@/commands/docbase"
import { parseFrontmatter } from "@/lib/frontmatter"
import { normalizePath } from "@/lib/path-utils"
import { withProjectLock } from "@/lib/project-mutex"
import { useWikiStore } from "@/stores/wiki-store"

/** How long the archive's folder is remembered between questions. */
const REMEMBER_MS = 15_000

let remembered: { at: number; path: string | null } | null = null

function clean(path: string): string {
  return normalizePath(path).replace(/\/+$/, "").toLowerCase()
}

/** The folder of the archive's wiki, or null (none, or no base in this run). */
export async function archiveWikiPath(): Promise<string | null> {
  if (remembered && Date.now() - remembered.at < REMEMBER_MS) return remembered.path
  let path: string | null = null
  try {
    path = (await getGeneralProject())?.path ?? null
  } catch {
    // No document base (tests, or a base that could not be opened).
    path = null
  }
  remembered = { at: Date.now(), path }
  return path
}

/** Forgets the folder remembered, after the archive's project changes. */
export function forgetArchiveWikiPath(): void {
  remembered = null
}

/** Whether the knowledge layer keeps the entities of the project at this folder. */
export async function usesKnowledgeLayer(projectPath: string): Promise<boolean> {
  const archive = await archiveWikiPath()
  return !!archive && clean(archive) === clean(projectPath)
}

/**
 * Whether a page the model wrote is the page of a person, an
 * organization or a matter: under wiki/entities/, or with that type.
 */
export function isEntityPageBlock(relativePath: string, content: string): boolean {
  const path = normalizePath(relativePath).toLowerCase()
  if (path.startsWith("wiki/entities/")) return true
  const type = parseFrontmatter(content).frontmatter?.type
  return typeof type === "string" && type.trim().toLowerCase() === "entity"
}

let toldLanguage: string | null = null

/** Tells the base the language the archive's wiki is written in, when it changes. */
export function tellWikiLanguage(language: string): void {
  if (!language || language === toldLanguage) return
  toldLanguage = language
  setWikiLanguage(language).catch(() => {
    toldLanguage = null
  })
}

/**
 * When the knowledge layer writes pages of the archive's wiki, the index
 * is written again and, if that wiki is open, its tree shows them.
 * `rebuild` writes the index of a folder (passed in so this module does
 * not load the ingest code).
 */
export function followKnowledgePages(rebuild: (projectPath: string) => Promise<unknown>): Promise<() => void> {
  return onKnowledgePages(() => {
    void (async () => {
      const path = await archiveWikiPath()
      if (!path) return
      try {
        await withProjectLock(normalizePath(path), () => rebuild(path))
      } catch (error) {
        console.warn("[knowledge] the wiki index could not be written again:", error)
      }
      const open = useWikiStore.getState().project
      if (open && clean(open.path) === clean(path)) {
        const { refreshProjectFileTree } = await import("@/lib/project-file-tree-refresh")
        await refreshProjectFileTree(open.path, { projectId: open.id }).catch(() => {})
      }
    })()
  })
}
