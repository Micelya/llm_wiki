import { describe, expect, it } from "vitest"
import {
  MAX_FORCED_NEIGHBOR_LABELS,
  MAX_LABEL_CHARS,
  focusLabel,
  hubLabelThreshold,
  resolveFocus,
  shortenLabel,
} from "./graph-labels"

describe("shortenLabel", () => {
  it("leaves short titles alone", () => {
    expect(shortenLabel("Factura A 00121")).toBe("Factura A 00121")
  })

  it("cuts long titles to the limit and marks the cut", () => {
    const short = shortenLabel("Carta documento de Extrimian a BGH Tech Partner del 31/07/2026")
    expect(Array.from(short).length).toBeLessThanOrEqual(MAX_LABEL_CHARS)
    expect(short.startsWith("Carta documento de Extrimian")).toBe(true)
    expect(short.endsWith("…")).toBe(true)
  })

  it("does not leave a space before the ellipsis", () => {
    expect(shortenLabel("abcd efgh", 6)).toBe("abcd…")
  })

  it("counts characters, not UTF-16 units", () => {
    expect(shortenLabel("😀😀😀😀", 3)).toBe("😀😀…")
  })
})

describe("hubLabelThreshold", () => {
  it("hides labels of nodes smaller than a hub", () => {
    expect(hubLabelThreshold(6, 15.7)).toBe(15.7)
  })

  it("keeps the stricter limit of large graphs", () => {
    expect(hubLabelThreshold(18, 6)).toBe(18)
  })
})

describe("resolveFocus", () => {
  const hover = { node: "a", neighbors: new Set(["b"]) }
  const pinned = { node: "c", neighbors: new Set(["d"]) }
  const all = () => true

  it("prefers the hovered node over the pinned one", () => {
    expect(resolveFocus(hover, pinned, all)).toBe(hover)
  })

  it("falls back to the pinned node", () => {
    expect(resolveFocus(null, pinned, all)).toBe(pinned)
  })

  it("ignores a pinned node that is no longer in the graph", () => {
    expect(resolveFocus(null, pinned, (node) => node !== "c")).toBeNull()
  })

  it("is null when nothing is in focus", () => {
    expect(resolveFocus(null, null, all)).toBeNull()
  })
})

describe("focusLabel", () => {
  const focus = { node: "a", neighbors: new Set(["b"]) }

  it("shows the full title on the focused node", () => {
    expect(focusLabel(focus, "a", "short…", "the full title")).toEqual({
      label: "the full title",
      forceLabel: true,
    })
  })

  it("keeps the short label on neighbours", () => {
    expect(focusLabel(focus, "b", "short…", "the full title")).toEqual({
      label: "short…",
      forceLabel: true,
    })
  })

  it("does not force neighbour labels when there are too many", () => {
    const many = new Set(Array.from({ length: MAX_FORCED_NEIGHBOR_LABELS + 1 }, (_, i) => `n${i}`))
    expect(focusLabel({ node: "a", neighbors: many }, "n0", "s", "f").forceLabel).toBe(false)
  })

  it("removes the label of every other node", () => {
    expect(focusLabel(focus, "z", "short…", "full")).toEqual({ label: "", forceLabel: false })
  })
})
