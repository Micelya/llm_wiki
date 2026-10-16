import { describe, expect, it } from "vitest"
import { vocabularyFor } from "@/vocabulary"
import {
  currentRelation,
  makesOneDocument,
  measuresInWords,
  passedOverNotice,
  percent,
  relationChoices,
  relationSentence,
  suggestedWikiSource,
} from "./similar"
import { docRef, RELATIONS, similarDocument, similarGroup } from "./similar.fixtures"

const words = vocabularyFor("legal", "es")

describe("measures in plain words", () => {
  it("cuts shares instead of rounding them up", () => {
    expect(percent(0.925)).toBe("92 %")
    expect(percent(0.996)).toBe("99 %")
    expect(percent(1)).toBe("100 %")
  })

  it("says how much they share and how long each one is", () => {
    expect(measuresInWords(similarDocument())).toEqual([
      "Comparten el 92 % del texto.",
      "El 99 % del más corto está, palabra por palabra, en el otro.",
      "Este documento tiene 1.200 palabras; el otro, 1.340.",
    ])
  })

  it("reads the words of each side from the evidence, which names the smaller id `a`", () => {
    const seenFromTheOther = similarDocument({ contentId: 2, other: docRef(1, "Contrato de servicios.pdf") })
    expect(measuresInWords(seenFromTheOther)).toContain("Este documento tiene 1.340 palabras; el otro, 1.200.")
  })

  it("leaves out the share of the shorter one when it says nothing new, and warns about text read from images", () => {
    const base = similarDocument()
    const lines = measuresInWords(
      similarDocument({
        cover: 0.95,
        similarity: 0.93,
        evidence: { ...base.evidence, recognized: { a: false, b: true } },
      }),
    )
    expect(lines).toHaveLength(3)
    expect(lines[2]).toBe("Parte del texto se leyó de una imagen y puede tener errores de lectura.")
  })
})

describe("the relation in a sentence", () => {
  it("says which one is the relation of the other", () => {
    expect(relationSentence(similarDocument(), "signed-copy", words)).toBe(
      "«Contrato de servicios firmado.pdf» es el ejemplar firmado de este documento.",
    )
    expect(relationSentence(similarDocument({ subjectContentId: 1 }), "signed-copy", words)).toBe(
      "Este documento es el ejemplar firmado de «Contrato de servicios firmado.pdf».",
    )
  })

  it("says nothing for a relation without direction, or for a relation the engine did not propose", () => {
    expect(relationSentence(similarDocument({ relation: "version", subjectContentId: null }), "version", words)).toBeNull()
    expect(relationSentence(similarDocument(), "preview", words)).toBeNull()
  })

  it("shows the chosen relation once joined", () => {
    expect(currentRelation(similarDocument({ state: "joined", chosenRelation: "version" }))).toBe("version")
    expect(currentRelation(similarDocument({ chosenRelation: null }))).toBe("signed-copy")
  })
})

describe("relations a person can choose", () => {
  it("offers the catalog's relations by their names, measured first, without the automatic one nor plain similar", () => {
    const choices = relationChoices(RELATIONS, words)
    expect(choices.measured.map((choice) => choice.name)).toEqual([
      "Prácticamente el mismo",
      "Ejemplar firmado",
      "Vista previa",
      "Versión",
      "Contenido en",
      "Misma plantilla, otros datos",
    ])
    expect(choices.inferred.map((choice) => choice.name)).toEqual(["Dice lo mismo con otras palabras", "Respuesta"])
  })

  it("offers a relation the catalog adds later, by its id until the vocabulary names it", () => {
    const choices = relationChoices(
      [...RELATIONS, { id: "annex-of", sameDocument: false, directed: true, measured: false, automatic: false }],
      words,
    )
    expect(choices.inferred.map((choice) => choice.name)).toContain("annex-of")
  })

  it("knows which relations leave one source for the wiki", () => {
    expect(makesOneDocument("version", RELATIONS)).toBe(true)
    expect(makesOneDocument("same-template", RELATIONS)).toBe(false)
    expect(makesOneDocument("unknown", RELATIONS)).toBe(false)
  })
})

describe("the wiki's source the app suggests", () => {
  it("takes the engine's suggestion", () => {
    expect(suggestedWikiSource(similarDocument())).toBe(2)
  })

  it("otherwise prefers a party's signature, then more words, then the older content", () => {
    const base = similarDocument({ suggestedSource: null })
    expect(suggestedWikiSource(base)).toBe(2)
    const unsigned = { ...base.evidence, marks: { a: [], b: [] } }
    expect(suggestedWikiSource({ ...base, evidence: unsigned })).toBe(2)
    expect(suggestedWikiSource({ ...base, evidence: { ...unsigned, words: { a: 500, b: 500 } } })).toBe(1)
  })
})

describe("a document joined with another as the same one", () => {
  const joined = (wikiSource: number, other = docRef(2, "Contrato de servicios firmado.pdf")) =>
    similarDocument({ state: "joined", chosenRelation: "signed-copy", wikiSource, other })

  it("says it no longer feeds the wiki when the other one is the chosen source", () => {
    expect(passedOverNotice([joined(2)])).toBe(
      "Este documento no alimenta el wiki: se unió con «Contrato de servicios firmado.pdf», que es la fuente elegida.",
    )
  })

  it("says nothing while it is the chosen source, or the pair is not joined", () => {
    expect(passedOverNotice([joined(1)])).toBeNull()
    expect(passedOverNotice([similarDocument()])).toBeNull()
    expect(passedOverNotice([similarDocument({ state: "joined", chosenRelation: "same-template" })])).toBeNull()
    expect(passedOverNotice([])).toBeNull()
  })

  it("names every chosen source when there are several", () => {
    expect(passedOverNotice([joined(2), joined(3, docRef(3, "Contrato definitivo.pdf"))])).toBe(
      "Este documento no alimenta el wiki: se unió con «Contrato de servicios firmado.pdf» y «Contrato definitivo.pdf», que son las fuentes elegidas.",
    )
  })
})
