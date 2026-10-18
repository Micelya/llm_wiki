import { describe, expect, it } from "vitest"
import type { Health } from "@/commands/docbase"
import { pendingCount, processedPercent } from "./attention"

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
