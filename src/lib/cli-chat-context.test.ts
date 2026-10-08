import { describe, expect, it, vi } from "vitest"
import {
  CLI_CONTEXT_MAX_PAGES,
  CLI_CONTEXT_PAGE_CHARS,
  CLI_CONTEXT_TOTAL_CHARS,
  buildCliRetrievedContext,
  cliRequestSkills,
} from "./cli-chat-context"

describe("cliRequestSkills", () => {
  it("sends no skills in auto mode so the backend fallback search still runs", () => {
    expect(cliRequestSkills("auto", ["pdf", "docx"])).toEqual([])
  })

  it("keeps the skills the user picked explicitly", () => {
    expect(cliRequestSkills("explicit", ["pdf"])).toEqual(["pdf"])
  })
})

describe("buildCliRetrievedContext", () => {
  it("appends the text of each retrieved wiki page after the summary", async () => {
    const readPage = vi.fn(async (path: string) => `body of ${path}`)
    const context = await buildCliRetrievedContext({
      summary: "found 2 pages",
      references: [
        { title: "Factura A", path: "wiki/sources/a.md", kind: "wiki" },
        { title: "Carta", path: "wiki/sources/b.md", kind: "wiki" },
      ],
      readPage,
    })

    expect(context.startsWith("found 2 pages")).toBe(true)
    expect(context).toContain('<page path="wiki/sources/a.md" title="Factura A">\nbody of wiki/sources/a.md\n</page>')
    expect(context).toContain("body of wiki/sources/b.md")
  })

  it("reads only wiki pages, once each", async () => {
    const readPage = vi.fn(async () => "text")
    await buildCliRetrievedContext({
      summary: "",
      references: [
        { title: "A", path: "wiki/a.md" },
        { title: "A again", path: "wiki/a.md" },
        { title: "Web", path: "https://example.com/x" },
        { title: "Raw", path: "raw/sources/doc.pdf" },
        { title: "Image", path: "wiki/media/x.png" },
      ],
      readPage,
    })

    expect(readPage).toHaveBeenCalledTimes(1)
    expect(readPage).toHaveBeenCalledWith("wiki/a.md")
  })

  it("returns the summary untouched when no page can be read", async () => {
    const context = await buildCliRetrievedContext({
      summary: "only snippets",
      references: [{ title: "A", path: "wiki/a.md" }],
      readPage: async () => {
        throw new Error("missing")
      },
    })

    expect(context).toBe("only snippets")
  })

  it("keeps the pages that could be read when another one fails", async () => {
    const context = await buildCliRetrievedContext({
      summary: "s",
      references: [
        { title: "A", path: "wiki/a.md" },
        { title: "B", path: "wiki/b.md" },
      ],
      readPage: async (path) => {
        if (path === "wiki/a.md") throw new Error("missing")
        return "b text"
      },
    })

    expect(context).not.toContain("wiki/a.md")
    expect(context).toContain("b text")
  })

  it("bounds the number of pages, each page and the total", async () => {
    const references = Array.from({ length: CLI_CONTEXT_MAX_PAGES + 4 }, (_, i) => ({
      title: `P${i}`,
      path: `wiki/p${i}.md`,
    }))
    const readPage = vi.fn(async () => "§".repeat(CLI_CONTEXT_PAGE_CHARS * 2))
    const context = await buildCliRetrievedContext({ summary: "", references, readPage })

    expect(readPage).toHaveBeenCalledTimes(CLI_CONTEXT_MAX_PAGES)
    expect(context).toContain("[page truncated]")
    const pageChars = (context.match(/§/g) ?? []).length
    expect(pageChars).toBeGreaterThan(CLI_CONTEXT_PAGE_CHARS)
    expect(pageChars).toBeLessThanOrEqual(CLI_CONTEXT_TOTAL_CHARS)
  })

  it("escapes quotes in titles so a title cannot close the page tag", async () => {
    const context = await buildCliRetrievedContext({
      summary: "",
      references: [{ title: 'Contrato "marco" <v2>', path: "wiki/c.md" }],
      readPage: async () => "text",
    })

    expect(context).toContain('title="Contrato &quot;marco&quot; &lt;v2>"')
  })
})
