import { describe, expect, it } from "vitest"
import { formatAgo, formatSize, parseConvention, plural } from "./format"

describe("document base formatting", () => {
  it("reads folder levels however the user separates them", () => {
    expect(parseConvention("cliente, caso")).toEqual(["cliente", "caso"])
    expect(parseConvention(" cliente / caso / ")).toEqual(["cliente", "caso"])
    expect(parseConvention("cliente > caso")).toEqual(["cliente", "caso"])
    expect(parseConvention("")).toEqual([])
  })

  it("agrees the noun with the count", () => {
    expect(plural(1, "archivo", "archivos")).toBe("1 archivo")
    expect(plural(0, "archivo", "archivos")).toBe("0 archivos")
    expect(plural(3, "archivo", "archivos")).toBe("3 archivos")
  })

  it("tells how long ago something happened", () => {
    const now = Date.UTC(2026, 9, 11, 12, 0, 0)
    expect(formatAgo(now - 20_000, now)).toBe("hace un momento")
    expect(formatAgo(now - 4 * 60_000, now)).toBe("hace 4 min")
    expect(formatAgo(now - 3 * 3_600_000, now)).toBe("hace 3 h")
    expect(formatAgo(now - 26 * 3_600_000, now)).toBe("ayer")
    expect(formatAgo(now - 3 * 86_400_000, now)).toBe("hace 3 días")
  })

  it("shows sizes in the largest sensible unit", () => {
    expect(formatSize(190)).toBe("190 B")
    expect(formatSize(2048)).toBe("2 KB")
    expect(formatSize(5 * 1024 * 1024)).toBe("5 MB")
  })
})
