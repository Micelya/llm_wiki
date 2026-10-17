import { describe, expect, it } from "vitest"
import { buildWikiIndex } from "./wiki-index"
import { vocabularyFor, wikiVocabulary } from "@/vocabulary"
import { isEntityPageBlock } from "./knowledge-layer"
import { isProjectedPage } from "./dedup-runner"

const words = vocabularyFor("legal", "es").wikiIndex

describe("the wiki's index", () => {
  it("lists every page under its type, people, organizations and matters apart, nothing left out", () => {
    const pages = Array.from({ length: 250 }, (_, i) => ({ path: `wiki/sources/doc-${i}.md`, title: `Documento ${i}`, type: "source" }))
    pages.push(
      { path: "wiki/entities/b-1.md", title: "Ejemplo Dos S.A.", type: "entity", entityKind: "organizacion" } as never,
      { path: "wiki/entities/a-1.md", title: "Persona Uno", type: "entity", entityKind: "persona" } as never,
      { path: "wiki/concepts/locacion.md", title: "Locación", type: "concept" },
      { path: "wiki/goals/x.md", title: "Algo propio", type: "goal" },
      { path: "wiki/index.md", title: "Índice", type: "" },
    )
    const index = buildWikiIndex(pages, words)
    expect(index.match(/^- \[\[sources\//gm)).toHaveLength(250)
    expect(index.indexOf("## Documentos (250)")).toBeLessThan(index.indexOf("## Personas, organizaciones y asuntos (2)"))
    expect(index).toContain("### Personas (1)\n\n- [[entities/a-1|Persona Uno]]")
    expect(index).toContain("### Organizaciones (1)\n\n- [[entities/b-1|Ejemplo Dos S.A.]]")
    expect(index).toContain("## Conceptos (1)")
    expect(index).toContain("## Otras páginas (1)\n\n- [[goals/x|Algo propio]]")
    expect(index).not.toContain("[[index")
    expect(index).not.toMatch(/## [^\n]+\n\n## /)
  })

  it("takes its words from the vocabulary of the wiki's language", () => {
    expect(wikiVocabulary("English").wikiIndex.title).toBe("Wiki index")
    expect(wikiVocabulary("Spanish").wikiIndex.title).toBe("Índice del wiki")
    expect(wikiVocabulary("Klingon").wikiIndex.title).toBe("Índice del wiki")
  })
})

describe("pages of the knowledge layer", () => {
  it("tells an entity page the model wrote by its folder or its type", () => {
    expect(isEntityPageBlock("wiki/entities/x.md", "")).toBe(true)
    expect(isEntityPageBlock("wiki/people/x.md", "---\ntype: entity\ntitle: X\n---\n")).toBe(true)
    expect(isEntityPageBlock("wiki/concepts/x.md", "---\ntype: concept\ntitle: X\n---\n")).toBe(false)
  })

  it("tells a projected page from one a person or the model wrote", () => {
    expect(isProjectedPage("---\r\ntype: entity\r\ngenerated_by: micelya-conocimiento\r\n---\r\n\r\n# X\r\n")).toBe(true)
    expect(isProjectedPage("---\ntype: entity\n---\n\ngenerated_by: micelya-conocimiento\n")).toBe(false)
    expect(isProjectedPage("# X\n")).toBe(false)
  })
})
