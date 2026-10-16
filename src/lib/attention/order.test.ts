import { describe, expect, it } from "vitest"
import { attentionCounts, navigationCount, noticesOf, orderNotices, summaryText } from "./order"
import type { AttentionNotice } from "./types"

function notice(id: string, patch: Partial<AttentionNotice> = {}): AttentionNotice {
  return {
    id,
    source: "test",
    kind: "decision",
    topic: id,
    priority: "normal",
    count: 1,
    title: id,
    explanation: "",
    todo: "",
    resolvedWhen: "",
    actions: [],
    ...patch,
  }
}

describe("the order of the attention view", () => {
  it("puts problems first, then decisions, then what is in progress", () => {
    const ordered = orderNotices([
      [notice("queue", { kind: "ongoing" }), notice("similar")],
      [notice("origin", { kind: "problem" })],
    ])
    expect(ordered.map((n) => n.id)).toEqual(["origin", "similar", "queue"])
  })

  it("orders by priority inside each class, then by source and by the order each source gave", () => {
    const ordered = orderNotices([
      [notice("a-low", { kind: "problem", priority: "low" }), notice("a-normal", { kind: "problem" })],
      [notice("b-high", { kind: "problem", priority: "high" }), notice("b-normal", { kind: "problem" })],
    ])
    expect(ordered.map((n) => n.id)).toEqual(["b-high", "a-normal", "b-normal", "a-low"])
  })

  it("splits the lines by class", () => {
    const all = [notice("p", { kind: "problem" }), notice("d"), notice("o", { kind: "ongoing" })]
    expect(noticesOf(all, "decision").map((n) => n.id)).toEqual(["d"])
  })
})

describe("the counters", () => {
  it("counts problem lines and decisions, not what is in progress nor what can wait", () => {
    const counts = attentionCounts([
      notice("origin", { kind: "problem", priority: "high", count: 3 }),
      notice("ingest", { kind: "problem", count: 5 }),
      notice("shortcuts", { kind: "problem", priority: "low", count: 74 }),
      notice("similar", { count: 4 }),
      notice("types", { count: 2 }),
      notice("links", { priority: "low", count: 12 }),
      notice("queue", { kind: "ongoing", count: 40 }),
    ])
    // A problem line is one thing that does not work; a family of similar
    // documents is one decision however many documents it has.
    expect(counts).toEqual({ problems: 2, decisions: 6, ongoing: 1 })
    expect(navigationCount(counts)).toBe(8)
  })

  it("sums up as Inicio says it", () => {
    expect(summaryText({ problems: 2, decisions: 9, ongoing: 1 })).toBe("2 problemas · 9 decisiones")
    expect(summaryText({ problems: 1, decisions: 0, ongoing: 0 })).toBe("1 problema")
    expect(summaryText({ problems: 0, decisions: 1, ongoing: 3 })).toBe("1 decisión")
    expect(summaryText({ problems: 0, decisions: 0, ongoing: 3 })).toBeNull()
  })
})
