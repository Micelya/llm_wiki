import { describe, expect, it } from "vitest"
import {
  cleanAssistantContentForSaving,
  titleFromCleanAssistantContent,
} from "./saved-query-text"

describe("the text of a saved query", () => {
  it("uses the first visible heading as the saved query title", () => {
    const content = [
      "<!-- sources: wiki/concepts/old.md -->",
      "<think>draft reasoning</think>",
      "# 煤矿安全治理建议",
      "",
      "正文内容。",
    ].join("\n")

    const clean = cleanAssistantContentForSaving(content)
    expect(titleFromCleanAssistantContent(clean)).toBe("煤矿安全治理建议")
  })

  it("removes hidden metadata and thinking blocks from saved content", () => {
    const content = [
      "<!-- save-worthy: yes -->",
      "<!-- sources: wiki/concepts/old.md -->",
      "<thinking>private chain</thinking>",
      "Visible answer",
    ].join("\n")

    expect(cleanAssistantContentForSaving(content)).toBe("Visible answer")
  })

  it("removes think tags", () => {
    expect(cleanAssistantContentForSaving("<think>draft</think>\nBody")).toBe("Body")
  })

  it("removes unclosed thinking blocks", () => {
    expect(cleanAssistantContentForSaving("<thinking>partial")).toBe("")
  })

  it("falls back for empty visible content", () => {
    expect(titleFromCleanAssistantContent(cleanAssistantContentForSaving("<thinking>all</thinking>"))).toBe("Saved Query")
    expect(titleFromCleanAssistantContent(cleanAssistantContentForSaving(""))).toBe("Saved Query")
  })

  it("truncates titles over 60 characters and keeps exactly-60-character titles", () => {
    expect(titleFromCleanAssistantContent("# " + "x".repeat(80))).toBe("x".repeat(60))
    expect(titleFromCleanAssistantContent("# " + "y".repeat(60))).toBe("y".repeat(60))
  })

  it("does not treat multiline HTML comments as hidden save metadata", () => {
    const content = "<!-- sources:\n  wiki/foo.md\n-->\nBody"
    expect(cleanAssistantContentForSaving(content)).toBe(content)
  })
})
