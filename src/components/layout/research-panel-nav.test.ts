import { describe, expect, it } from "vitest"
import {
  isResearchPanelVisible,
  nextResearchPanelNavState,
} from "./research-panel-nav"

describe("research panel nav state", () => {
  it("switches standalone views to Inicio and opens the panel", () => {
    expect(nextResearchPanelNavState("chat", false)).toEqual({
      activeView: "home",
      researchPanelOpen: true,
    })
    expect(nextResearchPanelNavState("settings", true)).toEqual({
      activeView: "home",
      researchPanelOpen: true,
    })
    expect(nextResearchPanelNavState("skills", false)).toEqual({
      activeView: "home",
      researchPanelOpen: true,
    })
  })

  it("toggles the panel beside the other screens", () => {
    expect(nextResearchPanelNavState("search", true)).toEqual({
      activeView: "search",
      researchPanelOpen: false,
    })
    expect(nextResearchPanelNavState("graph", false)).toEqual({
      activeView: "graph",
      researchPanelOpen: true,
    })
    expect(nextResearchPanelNavState("entities", false)).toEqual({
      activeView: "entities",
      researchPanelOpen: true,
    })
  })

  it("only marks the panel visible outside standalone views", () => {
    expect(isResearchPanelVisible("chat", true)).toBe(false)
    expect(isResearchPanelVisible("skills", true)).toBe(false)
    expect(isResearchPanelVisible("settings", true)).toBe(false)
    expect(isResearchPanelVisible("home", true)).toBe(true)
    expect(isResearchPanelVisible("home", false)).toBe(false)
  })
})
