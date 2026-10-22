import { describe, expect, it } from "vitest"
import { articlePreview, firstWikilink, plain } from "./article-preview"

// Every article below is made up.
const PERSON = `---
type: entity
title: "Persona de Ejemplo"
entity_kind: persona
entity_id: ent-1
generated_by: micelya-conocimiento
---

# Persona de Ejemplo

*Persona* · Alias: P. Ejemplo · DNI 11.111.111 (2 documentos)

## Lo que dicen los documentos

Vive en Calle Inventada 10.[^1] Firmó el contrato con [[organizacion-ficticia|Organización Ficticia S.A.]] en **marzo**.[^2]

- un dato suelto

[^1]: Domicilio: Calle Inventada 10 — «con domicilio en Calle Inventada 10» ([[doc-1|contrato.pdf]])
[^2]: Firma — «firma el presente» ([[doc-1|contrato.pdf]])
<!-- notas de la persona -->
`

const DOCUMENT = `---
type: source
title: "Contrato de ejemplo"
---

# Contrato de ejemplo

Contrato de locación de un local inventado, por dos años.

Tipo: Contrato · Fecha: 1 de enero de 2020

\u0001micelya-bloque\u0001
`

describe("the card of a link: type and short summary from the article (P-246)", () => {
  it("reads the kind under the title and the first paragraph of what the documents say", () => {
    const shown = articlePreview(PERSON)
    expect(shown.kind).toBe("persona")
    expect(shown.kindLabel).toBe("Persona")
    expect(shown.summary).toBe("Vive en Calle Inventada 10. Firmó el contrato con Organización Ficticia S.A. en marzo.")
  })

  it("a document's summary is its topic", () => {
    expect(articlePreview(DOCUMENT)).toEqual({
      kind: null,
      kindLabel: null,
      summary: "Contrato de locación de un local inventado, por dos años.",
    })
  })

  it("cuts a long summary at a word, and has none when the article has no text", () => {
    const long = `# Título\n\n${"palabra ".repeat(80)}\n`
    const summary = articlePreview(long, 60).summary ?? ""
    expect(summary.length).toBeLessThanOrEqual(61)
    expect(summary.endsWith("palabra…")).toBe(true)
    expect(articlePreview("# Solo un título\n\n## Y una sección\n").summary).toBeNull()
  })

  it("writes markdown as plain text, and finds the document of a note", () => {
    expect(plain("Ver [[a-b|el contrato]], [un enlace](#x), `código` y *énfasis*.[^3]")).toBe("Ver el contrato, un enlace, código y énfasis.")
    expect(firstWikilink("«cita» ([[doc-9|recibo.pdf]]); «otra» ([[doc-8|otro.pdf]])")).toEqual({ target: "doc-9", name: "recibo.pdf" })
    expect(firstWikilink("«cita» ([[doc-7]])")).toEqual({ target: "doc-7", name: "doc-7" })
    expect(firstWikilink("sin documento")).toBeNull()
  })
})
