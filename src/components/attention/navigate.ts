import type { AttentionTarget } from "@/lib/attention/types"
import { openWikiPath } from "@/lib/open-wiki-path"
import { DEFAULT_DOCUMENT_FILTER, useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { useAttentionViewStore } from "@/stores/attention-view-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"

/** Takes the user to the exact place a notice points at. */
export function goToTarget(target: AttentionTarget): void {
  const wiki = useWikiStore.getState()
  const docbase = useDocbaseStore.getState()
  switch (target.view) {
    case "documents":
      docbase.setFilter({ ...DEFAULT_DOCUMENT_FILTER, ...target.filter })
      wiki.setActiveView("documents")
      return
    case "document":
      wiki.setActiveView("documents")
      docbase.openDocument(target.locationId)
      return
    case "similar":
      useAttentionViewStore.getState().focusFamily(target.principal ?? null)
      wiki.setActiveView("similar")
      return
    case "entities":
      useKnowledgeStore.getState().openEntity(target.entityId ?? null)
      wiki.setActiveView("entities")
      return
    case "identity-doubts":
      useKnowledgeStore.getState().focusDoubt(target.group ?? null)
      wiki.setActiveView("identity-doubts")
      return
    case "matters":
      useKnowledgeStore.getState().openMatter(target.matterId ?? null)
      wiki.setActiveView("matters")
      return
    case "wiki-page":
      // The screen of what the page names, from the base (D-407).
      void openWikiPath(target.path.startsWith("wiki/") ? target.path : `wiki/${target.path}`)
      return
    default:
      wiki.setActiveView(target.view)
  }
}
