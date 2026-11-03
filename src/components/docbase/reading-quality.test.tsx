import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it } from "vitest"
import type { QualityDocument, ReadingQuality } from "@/commands/docbase"
import { readingQualityNotice } from "@/components/attention/sources/docbase"
import { below, effectText, pagesText, percent, verdictText } from "./reading-quality"
import { ReadingQualityBody } from "./reading-quality-view"

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

// Every name and number below is made up.
const doc = (versionId: number, share: number, verdict: QualityDocument["verdict"], changes: Partial<QualityDocument> = {}): QualityDocument => ({
  versionId,
  locationId: versionId * 10,
  name: `escrito-${versionId}.pdf`,
  path: `Prueba/escrito-${versionId}.pdf`,
  firstPage: null,
  lastPage: null,
  pages: 2,
  firm: Math.round(share * 100),
  approximate: 100 - Math.round(share * 100) - 5,
  discarded: 5,
  share,
  verdict,
  ...changes,
})

const quality = (changes: Partial<ReadingQuality> = {}): ReadingQuality => ({
  quality: 0.5,
  lowConfidenceType: 0.3,
  documents: 6,
  measured: 5,
  belowOne: [
    doc(1, 0.1, "no-legible", { firstPage: 3, lastPage: 4, pages: 2 }),
    doc(2, 0.45, "no-legible", { pages: 1 }),
    doc(3, 0.8, null, { pages: 5 }),
  ],
  steps: [
    { value: 0.3, documents: 1, pages: 2 },
    { value: 0.5, documents: 2, pages: 3 },
    { value: 1, documents: 3, pages: 8 },
  ],
  unmeasured: [doc(4, 1, null)],
  withoutText: [doc(5, 1, null, { name: "foto-vacia.jpg" })],
  ...changes,
})

function shown(data: ReadingQuality, value = 0.5): string {
  return text(
    renderToStaticMarkup(
      <ReadingQualityBody data={data} value={value} onValue={() => {}} onOpen={() => {}} action={() => <span>[Leer con IA]</span>} />,
    ),
  )
}

describe("Calidad de la lectura", () => {
  it("lists the documents that cannot be read with their file, pages, share and a link, and «Leer con IA» on each", () => {
    const words = shown(quality())
    expect(words).toContain("No legibles (2)")
    expect(words).toContain("escrito-1.pdf Prueba/escrito-1.pdf · págs. 3 a 4 del archivo")
    expect(words).toContain("10 % de palabras seguras (10 seguras, 85 aproximadas y 5 descartadas) · No legible")
    expect(words).toContain("escrito-2.pdf Prueba/escrito-2.pdf · 1 página")
    expect(words).toContain("De tipo de confianza baja (0)")
    expect(words).toContain("Con alguna palabra dudosa, que se usan (1)")
    expect(words.match(/Abrir/g)?.length).toBe(4)
    expect(words.match(/\[Leer con IA\]/g)?.length).toBe(4)
    expect(words).toContain("Sin texto (1)")
    expect(words).toContain("1 documento todavía no tiene su proporción")
  })

  it("says how many documents and pages stay below each value, and below the one the rule is on", () => {
    const words = shown(quality(), 0.5)
    expect(words).toContain("Con 50 %, quedan debajo 2 documentos (3 páginas). Hoy, con 50 %, quedan 2.")
    expect(words).toContain("30 % confianza baja hoy 1 2")
    expect(words).toContain("50 % «no legible» hoy 2 3")
    expect(shown(quality(), 0.9)).toContain("Con 90 %, quedan debajo 3 documentos (8 páginas)")
    expect(shown(quality(), 0.05)).toContain("Con 5 %, quedan debajo 0 documentos (0 páginas)")
  })

  it("explains the share with an example before the list", () => {
    const words = shown(quality())
    expect(words).toContain("tiene 9 de 27: 33 %")
    expect(words).toContain("Debajo de «no legible» (50 %) o del tipo de confianza baja (30 %), un documento sirve solo para consultas")
  })

  it("counts below a value with the rule of the base: strictly below", () => {
    const list = quality().belowOne
    expect(below(list, 0.45)).toEqual({ documents: 1, pages: 2 })
    expect(below(list, 0.46)).toEqual({ documents: 2, pages: 3 })
    expect(percent(0.333)).toBe("33 %")
    expect(pagesText(doc(9, 0.2, null, { firstPage: 7, lastPage: 7 }))).toBe("pág. 7 del archivo")
    expect(pagesText(doc(9, 0.2, null, { pages: null }))).toBe("páginas sin contar")
    expect(verdictText("low-confidence")).toBe("Tipo de confianza baja")
  })

  it("says what moving the thresholds of the confidence would change, and whether it asks again", () => {
    expect(effectText({ toLow: 0, toLowPages: 0, fromLow: 0, fromLowPages: 0, pagesWrittenAgain: 0 })).toBe(
      "Con estos valores ningún documento cambia de lado: no se pregunta nada.",
    )
    const up = effectText({ toLow: 3, toLowPages: 7, fromLow: 0, fromLowPages: 0, pagesWrittenAgain: 4 })
    expect(up).toContain("3 documentos pasan a confianza baja (7 páginas): lo que dieron sale de la wiki en la próxima resolución, sin preguntar nada.")
    expect(up).toContain("a lo sumo 4 páginas, una consulta cada una a «Preguntar al modelo»")
    const down = effectText({ toLow: 0, toLowPages: 0, fromLow: 1, fromLowPages: 1, pagesWrittenAgain: 0 })
    expect(down).toContain("1 documento deja de ser de confianza baja (1 página): su lectura ya está guardada y no se vuelve a pagar")
    expect(down).toContain("Ninguna página de personas, organizaciones o casos cambia: no se pregunta nada.")
  })

  it("is raised in Atención as a low line that leads to the screen", () => {
    expect(readingQualityNotice(quality({ belowOne: [doc(3, 0.8, null)] }))).toBeNull()
    const notice = readingQualityNotice(quality())
    expect(notice?.priority).toBe("low")
    expect(notice?.title).toBe("2 documentos leídos con pocas palabras seguras")
    expect(notice?.actions[0]).toEqual({ kind: "go", label: "Ver la calidad de la lectura", target: { view: "reading-quality" } })
    expect(notice?.cases?.[0].detail).toBe("10 % de palabras seguras · no legible")
  })
})
