import { describe, expect, it } from "vitest"
import {
  finishLegalPage,
  linkResolves,
  titleFromHeading,
  wikiPageKeys,
  withMdExtension,
  withoutAcademicFields,
  withoutSelfReference,
  withValidLinks,
} from "./ingest-legal-finish"

describe("the last touch on a law firm's page", () => {
  it("always gives a page its .md", () => {
    expect(withMdExtension("wiki/concepts/condicion-de-pago")).toBe("wiki/concepts/condicion-de-pago.md")
    expect(withMdExtension("wiki/concepts/mora.md")).toBe("wiki/concepts/mora.md")
    expect(withMdExtension("wiki/concepts/art.")).toBe("wiki/concepts/art.md")
  })

  it("takes the title from the first heading, never 'Source: …'", () => {
    const page = "---\ntype: source\ntitle: \"Source: Carpeta/Contrato de obra.pdf\"\n---\n\n# Contrato de obra con Ejemplo Uno S.A.\n\nTexto."
    const out = titleFromHeading(page)
    expect(out).toContain('title: "Contrato de obra con Ejemplo Uno S.A."')
    expect(out).not.toContain("Source:")
    const english = "---\ntype: concept\ntitle: Late payment interest\n---\n# Intereses por mora\n"
    expect(titleFromHeading(english)).toContain('title: "Intereses por mora"')
  })

  it("turns a 'Source: …' heading into the document's name", () => {
    const page = "---\ntype: source\ntitle: \"Source: Carpeta/Boleta 123.pdf\"\n---\n\n# Source: Carpeta/Boleta 123.pdf\n"
    const out = titleFromHeading(page)
    expect(out).toContain("# Boleta 123\n")
    expect(out).toContain('title: "Boleta 123"')
  })

  it("leaves a page whose title is its heading as it is", () => {
    const page = "---\ntype: concept\ntitle: \"Fuerza mayor\"\n---\n# Fuerza mayor\n"
    expect(titleFromHeading(page)).toBe(page)
  })

  it("drops the fields of an academic reference", () => {
    const page = "---\ntype: source\ntitle: \"Carta\"\nauthors: []\nyear: 2026\nurl: \"\"\nvenue: \"\"\ntags: [carta]\n---\n# Carta\n"
    const out = withoutAcademicFields(page)
    expect(out).not.toMatch(/authors|year|url|venue/)
    expect(out).toContain("tags: [carta]")
    const block = "---\ntitle: X\nauthors:\n  - Alguien\ntags: []\n---\n# X\n"
    expect(withoutAcademicFields(block)).toBe("---\ntitle: X\ntags: []\n---\n# X\n")
  })

  it("drops the sentences that speak of the wiki or of its material", () => {
    const page = [
      "---",
      "title: \"Carta documento\"",
      "---",
      "# Carta documento",
      "",
      "La carta intima el pago de $ 1.000. Esta figura ya figura en el wiki como concepto.",
      "- La respuesta no está incluida en el material proporcionado.",
      "- El contrato prevé multas por defectos en el material entregado al cliente.",
      "| Concepto | wiki |",
    ].join("\n")
    const { content, removed } = withoutSelfReference(page)
    expect(removed).toHaveLength(2)
    expect(content).toContain("La carta intima el pago de $ 1.000.")
    expect(content).not.toContain("ya figura en el wiki")
    expect(content).not.toContain("no está incluida")
    // A list item left empty goes; a table stays as it is.
    expect(content).not.toMatch(/^- $/m)
    expect(content).toContain("| Concepto | wiki |")
    expect(content).toContain("# Carta documento")
  })

  it("cuts sentences where they end, not after an abbreviation or an initial", () => {
    const page = "# X\n\nLa carta la firma el Dr. J. Pérez por el art. 5 de Ejemplo Uno S.A. El wiki ya tiene la carta.\nSigue en otra línea."
    const { content, removed } = withoutSelfReference(page)
    expect(removed).toEqual(["El wiki ya tiene la carta."])
    expect(content).toBe("# X\n\nLa carta la firma el Dr. J. Pérez por el art. 5 de Ejemplo Uno S.A. Sigue en otra línea.")
  })

  it("keeps what a sentence says of the documents when only its tail speaks of the wiki", () => {
    const page = "# X\n\nLa factura se suma a las facturas impagas de mayo y junio ya registradas en el wiki. Otra oración."
    const { content, removed } = withoutSelfReference(page)
    expect(removed).toHaveLength(1)
    expect(content).toBe("# X\n\nLa factura se suma a las facturas impagas de mayo y junio. Otra oración.")
    const middle = "# X\n\nSe relaciona con las cartas incluidas en el wiki, pero la correspondencia debe cotejarse."
    expect(withoutSelfReference(middle).content).toBe("# X\n\nSe relaciona con las cartas, pero la correspondencia debe cotejarse.")
    // The whole sentence is about the wiki: it goes.
    expect(withoutSelfReference("# X\n\nEl proyecto ya figura en el wiki.").content).toBe("# X\n")
  })

  it("keeps a sentence about the goods of a contract", () => {
    for (const line of [
      "El proveedor responde por los vicios en el material suministrado por terceros.",
      "El contrato regula la entrega de material.",
      "Se reclaman defectos en el material entregado.",
    ]) {
      expect(withoutSelfReference(`# X\n\n${line}`).removed).toEqual([])
    }
  })

  it("leaves a link to no page as its text", () => {
    const keys = wikiPageKeys(["wiki/concepts/mora.md", "wiki/sources/carta-1.md", "wiki/entities/persona-dos-ent12345.md"])
    expect(linkResolves(keys, "mora")).toBe(true)
    expect(linkResolves(keys, "concepts/mora")).toBe(true)
    expect(linkResolves(keys, "wiki/sources/carta-1.md")).toBe(true)
    expect(linkResolves(keys, "persona-dos-ent12345#Notas")).toBe(true)
    const { content, unlinked } = withValidLinks(
      "Ver [[mora]], [[fuerza-mayor|la fuerza mayor]], [[concepts/plazo-de-gracia]] y ![[media/a.png]].",
      keys,
    )
    expect(content).toBe("Ver [[mora]], la fuerza mayor, plazo-de-gracia y ![[media/a.png]].")
    expect(unlinked).toEqual(["fuerza-mayor", "concepts/plazo-de-gracia"])
  })

  it("does everything at once and says what it did", () => {
    const keys = wikiPageKeys(["wiki/concepts/mora.md"])
    const page = "---\ntype: concept\ntitle: Interest\nvenue: \"\"\n---\n# Intereses\n\nSe calculan según [[mora]] y [[tasa-activa]]. Ya está en el wiki."
    const { content, warnings } = finishLegalPage("wiki/concepts/intereses.md", page, keys)
    expect(content).toBe("---\ntype: concept\ntitle: \"Intereses\"\n---\n# Intereses\n\nSe calculan según [[mora]] y tasa-activa.")
    expect(warnings).toHaveLength(2)
  })
})
