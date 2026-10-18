import type { AttentionTarget } from "@/lib/attention/types"
import { normalizePath } from "@/lib/path-utils"
import { DEFAULT_DOCUMENT_FILTER, useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { useAttentionViewStore } from "@/stores/attention-view-store"

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
    case "wiki-page": {
      const project = wiki.project
      if (!project) return
      const path = target.path.startsWith("wiki/") ? target.path : `wiki/${target.path}`
      wiki.openPathInPreview(`${normalizePath(project.path)}/${path}`)
      return
    }
    default:
      wiki.setActiveView(target.view)
  }
}
