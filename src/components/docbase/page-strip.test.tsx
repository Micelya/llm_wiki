import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { IncludedPart } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"

const cutSplit = vi.fn<(hash: string, page: number) => Promise<IncludedPart[]>>()
const cutJoin = vi.fn<(hash: string, page: number) => Promise<IncludedPart[]>>()
const cutConfirm = vi.fn<(hash: string) => Promise<IncludedPart[]>>()
vi.mock("@/commands/docbase", () => ({
  cutSplit: (hash: string, page: number) => cutSplit(hash, page),
  cutJoin: (hash: string, page: number) => cutJoin(hash, page),
  cutConfirm: (hash: string) => cutConfirm(hash),
  includedParts: () => Promise.resolve([]),
  onDocbaseChanged: () => Promise.resolve(() => {}),
}))
vi.mock("@/commands/fs", () => ({ getFileSize: () => Promise.resolve(0), readFileAsBase64: () => Promise.reject(new Error("none")) }))
vi.mock("@/components/editor/file-preview", () => ({ decodeBase64: () => new Uint8Array() }))
// What the base answers for each file, at once: a file of one document has no parts.
const answers: Record<string, IncludedPart[]> = {}
vi.mock("./use-docbase-query", () => ({
  useDocbaseQuery: (key: string) => ({ data: answers[key.replace("included:", "")] ?? null, error: null, loading: false, reload: () => {} }),
}))

const { CompoundStrip, IncludedNote, PageStrip, runCutAction } = await import("./page-strip")
const { PartNotice } = await import("./document-detail")

const words = vocabularyFor("legal", "es")

function text(html: string): string {
  return html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ").trim()
}

function part(firstPage: number, lastPage: number, patch: Partial<IncludedPart> = {}): IncludedPart {
  return {
    position: firstPage,
    versionId: 100 + firstPage,
    documentId: 200 + firstPage,
    firstPage,
    lastPage,
    cutBy: "rule",
    cutState: "proposed",
    doubtfulPages: [],
    uncertain: false,
    whole: false,
    ...patch,
  }
}

// An invented writ with its attachments (R-24): pages 1-2 confirmed by a
// person, 3-5 proposed by the rules with page 4 uncertain, 6 backed by the model.
const PARTS = [
  part(1, 2, { cutBy: "person", cutState: "confirmed" }),
  part(3, 5, { doubtfulPages: [4], uncertain: true }),
  part(6, 6, { cutBy: "model" }),
]

function strip(parts: IncludedPart[], onAction = () => {}): string {
  return renderToStaticMarkup(<PageStrip parts={parts} words={words} onAction={onAction} onOpen={() => {}} />)
}

/** The attributes of the element that has `marker`. */
function tagWith(html: string, marker: string): string[] {
  return html.match(new RegExp(`<[^>]*${marker}[^>]*>`, "g")) ?? []
}

describe("the strip of pages of a joined file (D-560)", () => {
  beforeEach(() => {
    cutSplit.mockReset()
    cutJoin.mockReset()
    cutConfirm.mockReset()
  })

  it("shows where each document starts, how its start stands and who said it, in the vertical's words", () => {
    const html = strip(PARTS)
    const shown = text(html)
    expect(shown).toMatch(/^Documentos que incluye 3 /)
    expect(shown).toContain("Documento 1 páginas 1 a 2 Confirmado · lo decidió una persona")
    expect(shown).toContain("Documento 2 páginas 3 a 5 Incierto · lo propuso la app por el texto")
    expect(shown).toContain("Documento 3 páginas 6 a 6 Respaldado · lo propuso el modelo mirando las páginas")
    // Only the uncertain one is marked (D-603).
    expect(tagWith(html, "data-uncertain").map((tag) => tag.match(/data-part="(\d+)"/)?.[1])).toEqual(["2"])
    // Every page has its place in the strip.
    expect(tagWith(html, "data-page=").length).toBe(6)
    // No word of the inside reaches the screen (D-512).
    expect(shown).not.toMatch(/\b(cut|appearance|version|rule|model|person|proposed|confirmed)\b/i)
  })

  it("marks the doubtful pages to look at, and only them", () => {
    const html = strip(PARTS)
    expect(tagWith(html, "data-doubtful").map((tag) => tag.match(/data-page="(\d+)"/)?.[1])).toEqual(["4"])
    expect(text(html).match(/Para mirar/g)?.length).toBe(1)
  })

  it("offers joining on the page a document starts on and splitting on the others; never on the first page", () => {
    const html = strip(PARTS)
    const actions = html.match(/data-page="(\d+)"[\s\S]*?(?=data-page=|<\/ol>)/g)!.map((tile) => [
      Number(tile.match(/data-page="(\d+)"/)![1]),
      tile.match(/data-action="(\w+)"/)?.[1] ?? null,
    ])
    expect(actions).toEqual([
      [1, null],
      [2, "split"],
      [3, "join"],
      [4, "split"],
      [5, "split"],
      [6, "join"],
    ])
  })

  it("splitting and joining call their own command with the page", async () => {
    cutSplit.mockResolvedValue(PARTS)
    cutJoin.mockResolvedValue(PARTS)
    await runCutAction("h-1", { kind: "split", page: 4 })
    expect(cutSplit).toHaveBeenCalledWith("h-1", 4)
    expect(cutJoin).not.toHaveBeenCalled()
    await runCutAction("h-1", { kind: "join", page: 6 })
    expect(cutJoin).toHaveBeenCalledWith("h-1", 6)
    expect(cutSplit).toHaveBeenCalledTimes(1)
    expect(cutConfirm).not.toHaveBeenCalled()
  })

  it("confirming leaves every start confirmed by a person, nothing to look at and nothing more to confirm", async () => {
    cutConfirm.mockResolvedValue(PARTS.map((p) => ({ ...p, cutBy: "person", cutState: "confirmed", doubtfulPages: [], uncertain: false })))
    const before = strip(PARTS)
    expect(text(before)).toContain("Confirmar dónde empieza cada documento")
    const after = strip(await runCutAction("h-1", { kind: "confirm" }))
    expect(cutConfirm).toHaveBeenCalledWith("h-1")
    expect(tagWith(after, "data-cut-state=").every((tag) => tag.includes('data-cut-state="confirmed"'))).toBe(true)
    expect(text(after)).not.toContain("Incierto")
    expect(text(after)).not.toContain("Para mirar")
    expect(tagWith(after, "disabled").some((tag) => tag.startsWith("<button"))).toBe(true)
    expect(text(after)).toContain("Todo confirmado")
  })

  it("a file of one document shows no strip; a joined one does", () => {
    answers["h-single"] = []
    answers["h-joined"] = PARTS
    const of = (hash: string) => renderToStaticMarkup(<CompoundStrip hash={hash} path="C:/x.pdf" isPdf onOpen={() => {}} />)
    expect(of("h-single")).toBe("")
    expect(of("h-joined")).toContain("data-strip")
  })

  it("a PDF of one document with an uncertain step shows the whole file to look at, with nothing else to open", () => {
    answers["h-uncertain"] = [part(1, 3, { whole: true, uncertain: true, cutState: "proposed", doubtfulPages: [3] })]
    const html = renderToStaticMarkup(<CompoundStrip hash="h-uncertain" path="C:/x.pdf" isPdf onOpen={() => {}} />)
    const shown = text(html)
    expect(shown).toContain("Dónde podría empezar otro documento")
    expect(shown).toContain("Documento 1 páginas 1 a 3 Incierto · lo propuso la app por el texto")
    expect(tagWith(html, "data-doubtful").map((tag) => tag.match(/data-page="(\d+)"/)?.[1])).toEqual(["3"])
    expect(shown).not.toContain("Abrir")
    // The whole file stays confirmed in the base: the mark decides how it is shown.
    expect(text(strip([part(1, 3, { whole: true, uncertain: true, cutState: "confirmed" })]))).not.toContain("Confirmado")
  })
})

describe("a joined file in the list and its documents opened", () => {
  it("the list says how many documents a file includes and how many are uncertain", () => {
    expect(text(renderToStaticMarkup(<IncludedNote included={5} unconfirmed={3} words={words} />))).toBe(
      "incluye 5 documentos · 3 inciertos",
    )
    expect(text(renderToStaticMarkup(<IncludedNote included={5} unconfirmed={0} words={words} />))).toBe("incluye 5 documentos")
    expect(renderToStaticMarkup(<IncludedNote included={0} unconfirmed={0} words={words} />)).toBe("")
    // A PDF of one document with an uncertain step says so too.
    expect(text(renderToStaticMarkup(<IncludedNote included={0} unconfirmed={1} words={words} />))).toBe("un documento · incierto")
  })

  it("a document a file includes says which one it is, its pages and the way back to the whole file", () => {
    const html = renderToStaticMarkup(
      <PartNotice
        part={{ ordinal: 2, count: 3, firstPage: 3, lastPage: 5, cutBy: "rule", cutState: "proposed", uncertain: false }}
        file="escrito con anexos.pdf"
        onFile={() => {}}
      />,
    )
    expect(text(html)).toContain("Documento 2 de 3 incluidos en escrito con anexos.pdf, páginas 3 a 5")
    expect(text(html)).toContain("Ver el archivo completo")
  })
})
