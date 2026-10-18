import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import { vocabularyFor } from "@/vocabulary"
import { SimilarCard, SimilarList } from "./similar-documents"
import { docRef, RELATIONS, similarDocument } from "./similar.fixtures"

// The section talks to the base only through its props here.
vi.mock("@/commands/docbase", () => ({}))

const words = vocabularyFor("legal", "es")

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/\s+/g, " ")
    .trim()
}

function card(props: Partial<Parameters<typeof SimilarCard>[0]> = {}): string {
  return renderToStaticMarkup(
    <SimilarCard
      item={similarDocument()}
      relations={RELATIONS}
      name="Contrato de servicios.pdf"
      words={words}
      onOpen={() => {}}
      onDecide={() => {}}
      {...props}
    />,
  )
}

describe("the similar documents section", () => {
  it("says so when there are none", () => {
    const html = renderToStaticMarkup(
      <SimilarList items={[]} relations={RELATIONS} name="x" words={words} onOpen={() => {}} onDecide={() => {}} />,
    )
    expect(text(html)).toContain("Documentos parecidos (0)")
    expect(text(html)).toContain("Ningún otro documento de la base comparte buena parte de su texto con este.")
  })

  it("shows each similar document with its relation, description, measures and buttons", () => {
    const shown = text(card())
    expect(shown).toContain("Contrato de servicios firmado.pdf")
    expect(shown).toContain("Carpeta de ejemplo › Contratos/Contrato de servicios firmado.pdf")
    expect(shown).toContain("Relación propuesta: Ejemplar firmado")
    expect(shown).toContain("«Contrato de servicios firmado.pdf» es el ejemplar firmado de este documento.")
    expect(shown).toContain("El mismo texto, y uno de los dos tiene la firma verificable de una parte.")
    expect(shown).toContain("Comparten el 92 % del texto.")
    expect(shown).toContain("Espera una decisión")
    expect(shown).toContain("Unir como «Ejemplar firmado»")
    expect(shown).toContain("Elegir otra relación")
    expect(shown).toContain("Descartar")
    expect(shown).not.toContain("Deshacer")
  })

  it("offers the side by side differences when it knows where this document is", () => {
    expect(card()).toMatch(/<button[^>]*disabled=""[^>]*>(?:(?!<\/button>).)*Ver diferencias/)
    expect(card({ selfLocationId: 10 })).not.toMatch(/<button[^>]*disabled=""[^>]*>(?:(?!<\/button>).)*Ver diferencias/)
  })

  it("asks which one stays as the wiki's source when the relation makes them one document, suggesting one", () => {
    const html = card({ initialJoining: { relation: "signed-copy", choosing: false, source: 2 } })
    const shown = text(html)
    expect(shown).toContain("¿cuál queda como fuente del wiki?")
    expect(shown).toContain("este documento (Contrato de servicios.pdf)")
    expect(shown).toMatch(/«Contrato de servicios firmado.pdf» \(sugerido\)/)
    expect(html).toMatch(/<input type="radio"[^>]*checked=""/)
  })

  it("offers the configured relations when choosing another, and keeps both in the wiki for distinct documents", () => {
    const html = card({ initialJoining: { relation: "same-template", choosing: true, source: 2 } })
    const shown = text(html)
    expect(html).toContain('<optgroup label="Se miden comparando textos">')
    expect(html).toContain('<optgroup label="Se infieren">')
    expect(shown).toContain("Misma plantilla, otros datos")
    expect(shown).toContain("Dice lo mismo con otras palabras")
    expect(shown).not.toContain("Mismo documento ")
    expect(shown).toContain("Son documentos distintos: los dos siguen yendo al wiki.")
  })

  it("shows a decision and lets it be undone", () => {
    const joined = text(
      card({
        item: similarDocument({
          state: "joined",
          chosenRelation: "version",
          wikiSource: 1,
          decidedAt: Date.UTC(2026, 9, 10, 12),
        }),
      }),
    )
    expect(joined).toContain("Unidos como Versión")
    expect(joined).toContain("Unidos el 10/10/2026")
    expect(joined).toContain("Queda como fuente del wiki: este documento (Contrato de servicios.pdf)")
    expect(joined).toContain("Deshacer")
    expect(joined).not.toContain("Descartar")

    const discarded = card({ item: similarDocument({ state: "discarded", decidedAt: Date.UTC(2026, 9, 10, 12) }) })
    expect(discarded).toContain('data-state="discarded"')
    expect(text(discarded)).toContain("Descartado el 10/10/2026")
    expect(text(discarded)).toContain("Deshacer")
  })

  it("proposes nothing to join when the engine found no clear relation", () => {
    const shown = text(
      card({
        item: similarDocument({ relation: "similar", subjectContentId: null, certainty: "low", suggestedSource: null }),
      }),
    )
    expect(shown).toContain("Sin relación sugerida")
    expect(shown).toContain("Elegir una relación")
    expect(shown).not.toContain("Unir como")
  })

  it("marks a proposal made on text read from images as less certain", () => {
    expect(text(card({ item: similarDocument({ certainty: "low" }) }))).toContain("(propuesta con menos certeza)")
  })

  it("lists every similar document of the one shown", () => {
    const items = [
      similarDocument(),
      similarDocument({ other: docRef(3, "Contrato borrador.docx"), relation: "version", subjectContentId: null }),
    ]
    const html = renderToStaticMarkup(
      <SimilarList items={items} relations={RELATIONS} name="x" words={words} onOpen={() => {}} onDecide={() => {}} />,
    )
    expect(text(html)).toContain("Documentos parecidos (2)")
    expect(text(html)).toContain("Nada se une solo")
    expect(text(html)).toContain("Unir como «Versión»")
  })
})
