import type { WikiState } from "@/stores/wiki-store"

/** The screens that use the whole work area: no research panel beside them. */
export function isStandaloneView(view: WikiState["activeView"]): boolean {
  return view === "chat" || view === "skills" || view === "settings"
}

export function isResearchPanelVisible(
  activeView: WikiState["activeView"],
  researchPanelOpen: boolean,
): boolean {
  return researchPanelOpen && !isStandaloneView(activeView)
}

export function nextResearchPanelNavState(
  activeView: WikiState["activeView"],
  researchPanelOpen: boolean,
): { activeView: WikiState["activeView"]; researchPanelOpen: boolean } {
  if (isStandaloneView(activeView)) {
    return { activeView: "home", researchPanelOpen: true }
  }
  return { activeView, researchPanelOpen: !researchPanelOpen }
}
