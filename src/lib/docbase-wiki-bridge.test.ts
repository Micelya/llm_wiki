import { describe, expect, it } from "vitest"
import type { WikiSource } from "@/commands/docbase"
import { isSameProjectPath, selectPendingSources } from "./docbase-wiki-bridge"

function source(identity: string): WikiSource {
  return {
    originId: "o1",
    identity,
    sourcePath: `raw/sources/${identity}`,
    ext: identity.split(".").pop() ?? "",
    contentId: null,
  }
}

describe("sources of the base still pending in the wiki", () => {
  const sources = [
    source("Casos/BGH/carta.pdf"),
    source("Casos/BGH/contrato.docx"),
    source("Casos/BGH/foto.jpg"),
    source("Casos/BGH/audiencia.mp3"),
    source("Casos/BGH/plano.dwg"),
  ]

  it("leaves out what the wiki cannot ingest", () => {
    const pending = selectPendingSources(sources, [], [])
    expect(pending.map((s) => s.identity)).toEqual([
      "Casos/BGH/carta.pdf",
      "Casos/BGH/contrato.docx",
      "Casos/BGH/foto.jpg",
    ])
  })

  it("leaves out what is already ingested or waiting in the queue", () => {
    const pending = selectPendingSources(
      sources,
      ["casos/bgh/CARTA.pdf"],
      ["raw\\sources\\Casos\\BGH\\contrato.docx"],
    )
    expect(pending.map((s) => s.identity)).toEqual(["Casos/BGH/foto.jpg"])
  })
})

describe("project folder comparison", () => {
  it("ignores separators, case and a trailing slash", () => {
    expect(isSameProjectPath("C:\\Wikis\\General", "c:/wikis/general/")).toBe(true)
    expect(isSameProjectPath("C:/Wikis/General", "C:/Wikis/Otro")).toBe(false)
    expect(isSameProjectPath(null, "C:/Wikis/General")).toBe(false)
  })
})
