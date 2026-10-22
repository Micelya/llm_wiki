import { describe, expect, it } from "vitest"
import { NOTE_HREF, noteLabel, splitNotes } from "./article-notes"

// Every page below is made up.
const PAGE = [
  "## Lo que dicen los documentos",
  "",
  "Vive en Calle Inventada 100.[^1] Es socia de Ficticia SA.[^2][^1]",
  "",
  "## Documentos (1)",
  "",
  "- [[contrato-de-prueba|Contrato de prueba.pdf]] — parte",
  "",
  "[^1]: Domicilio: Calle Inventada 100 — «con domicilio en Calle Inventada 100» ([[contrato-de-prueba|Contrato de prueba.pdf]], p. 2)",
  "[^2]: Socio de: Ficticia SA — «socia de Ficticia SA» ([[contrato-de-prueba|Contrato de prueba.pdf]])",
  "",
].join("\n")

describe("the notes of an article (D-441)", () => {
  it("takes the notes out of the text and turns each mark into a link to its note", () => {
    const { body, notes } = splitNotes(PAGE)
    expect(notes).toEqual({
      "1": "Domicilio: Calle Inventada 100 — «con domicilio en Calle Inventada 100» ([[contrato-de-prueba|Contrato de prueba.pdf]], p. 2)",
      "2": "Socio de: Ficticia SA — «socia de Ficticia SA» ([[contrato-de-prueba|Contrato de prueba.pdf]])",
    })
    expect(body).toContain(`Vive en Calle Inventada 100.[1](${NOTE_HREF}1) Es socia de Ficticia SA.[2](${NOTE_HREF}2)[1](${NOTE_HREF}1)`)
    expect(body).not.toContain("[^")
    expect(body).not.toContain("«con domicilio")
    // The list of documents stays as it was.
    expect(body).toContain("- [[contrato-de-prueba|Contrato de prueba.pdf]] — parte")
  })

  it("leaves a page without notes, a mark without its note and code as they are", () => {
    const plain = "Un texto sin notas.\n"
    expect(splitNotes(plain)).toEqual({ body: plain, notes: {} })
    const { body } = splitNotes("Uno.[^1] Dos.[^9] `[^1]`\n\n[^1]: «uno»\n")
    expect(body).toBe(`Uno.[1](${NOTE_HREF}1) Dos.[^9] \`[^1]\`\n`)
  })

  it("knows the label of a note from its link, and no other link", () => {
    expect(noteLabel(`${NOTE_HREF}12`)).toBe("12")
    expect(noteLabel("#contrato-de-prueba")).toBeNull()
  })
})
