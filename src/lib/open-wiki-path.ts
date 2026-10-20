import { entityOfPage } from "@/commands/docbase"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"

/**
 * Follows a link to a page of the wiki. The page of a person or an
 * organization has one place, its page in "Personas y organizaciones"
 * (the article, with its data and corrections in a tab: D-376, D-389);
 * any other page opens in the wiki. Opening a file from the tree still
 * opens the file, to write notes in it.
 */
export async function openWikiPath(path: string): Promise<void> {
  let entity: string | null = null
  try {
    entity = await entityOfPage(path)
  } catch {
    entity = null
  }
  const wiki = useWikiStore.getState()
  if (entity) {
    useKnowledgeStore.getState().openEntity(entity)
    wiki.setActiveView("entities")
    return
  }
  wiki.openPathInPreview(path)
}
