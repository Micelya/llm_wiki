import { describe, expect, it } from "vitest"
import type { Health, OriginSummary } from "@/commands/docbase"
import { buildAttention, pendingCount, processedPercent } from "./attention"

function health(byState: Health["byState"], locations: number): Health {
  return {
    origins: 1,
    unreachableOrigins: 0,
    locations,
    documents: locations,
    byState,
    copies: 0,
    words: 0,
    lastScanAt: null,
  }
}

function origin(patch: Partial<OriginSummary>): OriginSummary {
  return {
    id: "o1",
    name: "Casos",
    provider: "local-folder",
    location: "C:/casos",
    convention: [],
    addedAt: 0,
    lastScanAt: 0,
    reachable: true,
    files: 0,
    pending: 0,
    errors: 0,
    links: 0,
    ...patch,
  }
}

describe("what needs attention", () => {
  it("is empty when everything is in order", () => {
    expect(buildAttention(health({ processed: 10 }, 10), [origin({})], 0)).toEqual([])
  })

  it("puts errors first and says where to go for each", () => {
    const items = buildAttention(
      health({ processed: 5, link: 74, "needs-recognition": 3, unreadable: 1 }, 83),
      [origin({ reachable: false })],
      2,
    )
    expect(items.map((item) => item.id)).toEqual([
      "unreadable",
      "origin:o1",
      "needs-recognition",
      "review",
      "link",
    ])
    expect(items[0].title).toBe("1 archivo no se pudo leer")
    expect(items[0].target).toEqual({ view: "documents", state: "unreadable" })
    expect(items[2].title).toBe("3 documentos tienen el texto en imagen")
    expect(items[3].target).toEqual({ view: "review" })
    expect(items[4].title).toBe("74 documentos de un disco en la nube quedaron afuera")
  })

  it("does not count an origin never swept as unreachable", () => {
    expect(buildAttention(health({}, 0), [origin({ reachable: null })], 0)).toEqual([])
  })
})

describe("archive health figures", () => {
  it("measures progress over what can be processed", () => {
    // 183 files: 74 shortcuts and 2 without text cannot be processed.
    const h = health({ processed: 104, link: 74, unsupported: 2, "needs-recognition": 3 }, 183)
    expect(processedPercent(h)).toBe(97.2)
    expect(processedPercent(health({ link: 4 }, 4))).toBeNull()
  })

  it("counts as pending what was not read or has no text yet", () => {
    expect(pendingCount(health({ unread: 7, unprocessed: 2, processed: 1 }, 10))).toBe(9)
  })
})
