import { objectOfPage, type PageObject } from "@/commands/docbase"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"

/**
 * Follows a link to a page: a logical path (`wiki/entities/…`), the old
 * wiki's path of a file, or the name of a link (`[[name|text]]`). Every
 * page is the screen of what it names, read from the base (D-407): a
 * person or organization, a matter, a document, or Inicio for the
 * overview. Never a file. `false` when the page names nothing the base
 * knows.
 */
export async function openWikiPath(reference: string): Promise<boolean> {
  let found: PageObject | null = null
  try {
    found = await objectOfPage(reference)
  } catch {
    found = null
  }
  return found ? openPageObject(found) : false
}

/** Opens the screen of what a page names. */
export function openPageObject(found: PageObject): boolean {
  const wiki = useWikiStore.getState()
  switch (found.kind) {
    case "entity":
      useKnowledgeStore.getState().openEntity(found.objectId)
      wiki.setActiveView("entities")
      return true
    case "matter":
      useKnowledgeStore.getState().openMatter(found.objectId)
      wiki.setActiveView("matters")
      return true
    case "document":
      if (found.locationId === null) return false
      wiki.setActiveView("documents")
      useDocbaseStore.getState().openDocument(found.locationId)
      return true
    case "overview":
      wiki.setActiveView("home")
      return true
    default:
      return false
  }
}
