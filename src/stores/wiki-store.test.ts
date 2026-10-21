import { describe, expect, it } from "vitest"
import { useWikiStore } from "./wiki-store"

describe("wiki store", () => {
  it("opens on Inicio", () => {
    expect(useWikiStore.getInitialState().activeView).toBe("home")
  })

  it("keeps graph ui state until explicitly reset", () => {
    useWikiStore.getState().resetGraphUiState()

    useWikiStore.getState().setGraphUiState((current) => ({
      ...current,
      colorMode: "community",
      filters: {
        ...current.filters,
        minLinks: 2,
        hiddenTypes: new Set(["source"]),
      },
      nodeScale: 1.25,
      graphSpacingDraft: 1.4,
    }))
    useWikiStore.getState().setActiveView("chat")
    useWikiStore.getState().setActiveView("graph")

    const preserved = useWikiStore.getState().graphUiState
    expect(preserved.colorMode).toBe("community")
    expect(preserved.filters.minLinks).toBe(2)
    expect(preserved.filters.hiddenTypes.has("source")).toBe(true)
    expect(preserved.nodeScale).toBe(1.25)
    expect(preserved.graphSpacingDraft).toBe(1.4)

    useWikiStore.getState().resetGraphUiState()
    const reset = useWikiStore.getState().graphUiState
    expect(reset.colorMode).toBe("type")
    expect(reset.filters.minLinks).toBeUndefined()
    expect(reset.filters.hiddenTypes.size).toBe(0)
    expect(reset.nodeScale).toBe(1)
    expect(reset.graphSpacingDraft).toBe(1)
  })
})
