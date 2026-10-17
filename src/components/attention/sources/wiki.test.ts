import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }))

import type { LintItem } from "@/stores/lint-store"
import type { ReviewItem } from "@/stores/review-store"
import { vocabularyFor } from "@/vocabulary"
import { archiveWikiNotices, lintNotices, projectScope, reviewKindsText, reviewNotices } from "./wiki"

function review(id: string, type: ReviewItem["type"], resolved = false): ReviewItem {
  return { id, type, title: `Propuesta ${id}`, description: "", options: [], resolved, createdAt: 0 }
}

function lint(id: string, type: LintItem["type"], brokenTarget?: string): LintItem {
  return { id, type, severity: "warning", page: `entities/${id}.md`, detail: "", brokenTarget, createdAt: 0 }
}

describe("the wiki's review in the attention view", () => {
  it("counts what is not resolved and says what kinds there are", () => {
    const items = [review("1", "contradiction"), review("2", "missing-page"), review("3", "missing-page"), review("4", "duplicate", true)]
    const [line] = reviewNotices(items, undefined)
    expect(line).toMatchObject({ kind: "decision", count: 3, title: "3 propuestas del wiki esperan tu revisión" })
    expect(line.explanation.startsWith("1 contradicción entre páginas y 2 páginas que faltan.")).toBe(true)
    expect(line.actions[0]).toMatchObject({ kind: "go", target: { view: "review" } })
  })

  it("goes away when everything is resolved", () => {
    expect(reviewNotices([review("1", "suggestion", true)], undefined)).toEqual([])
  })

  it("names three kinds and more with commas", () => {
    expect(reviewKindsText([review("1", "contradiction"), review("2", "duplicate"), review("3", "suggestion")])).toBe(
      "1 contradicción entre páginas, 1 página duplicada y 1 sugerencia",
    )
  })
})

describe("the wiki's broken links in the attention view", () => {
  it("counts the broken links the structural check found, as a decision that can wait", () => {
    const [line] = lintNotices([lint("a", "broken-link", "Contrato"), lint("b", "orphan"), lint("c", "broken-link")], undefined)
    expect(line).toMatchObject({ kind: "decision", priority: "low", count: 2 })
    expect(line.cases?.[0]).toMatchObject({ title: "entities/a.md", detail: "apunta a «Contrato»" })
  })

  it("raises nothing for orphan pages alone", () => {
    expect(lintNotices([lint("b", "orphan")], undefined)).toEqual([])
  })
})

describe("whose the notices are", () => {
  it("says nothing for the archive and names any other project", () => {
    expect(projectScope({ name: "Mi archivo", path: "C:\\Wikis\\archivo" }, "C:/Wikis/archivo")).toBeUndefined()
    expect(projectScope({ name: "Pericias", path: "C:/Wikis/pericias" }, "C:/Wikis/archivo")).toBe("Proyecto «Pericias»")
  })

  it("shows the archive's wiki while another project is open, without buttons", () => {
    const lines = archiveWikiNotices([review("1", "contradiction")], [lint("a", "broken-link")], vocabularyFor("legal", "es"))
    expect(lines.map((line) => line.id)).toEqual(["archive-wiki:proposals", "archive-wiki:broken-links"])
    expect(lines.every((line) => line.scope === "Wiki del archivo" && line.actions.length === 0)).toBe(true)
  })
})
