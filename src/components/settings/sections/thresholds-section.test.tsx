import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it } from "vitest"
import type { ConfidenceEffect, Thresholds } from "@/commands/docbase"
import { BY_DEFAULT, changed, invalid, parsed, shown, THRESHOLDS, ThresholdsView } from "./thresholds-section"

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

function view(saved: Thresholds, draft: Thresholds, effect: ConfidenceEffect | null = null): string {
  return text(
    renderToStaticMarkup(
      <ThresholdsView
        saved={saved}
        draft={draft}
        effect={effect}
        problem={null}
        saving={false}
        onChange={() => {}}
        onSave={() => {}}
        onDiscard={() => {}}
        onQuality={() => {}}
      />,
    ),
  )
}

const info = (key: keyof Thresholds) => THRESHOLDS.find((t) => t.key === key)!

describe("Ajustes → Umbrales", () => {
  it("shows every threshold of the base, with what it decides, its value, whether it is provisional and its value by default", () => {
    const words = view(BY_DEFAULT, BY_DEFAULT)
    expect(THRESHOLDS.map((t) => t.key).sort()).toEqual(Object.keys(BY_DEFAULT).sort())
    for (const group of ["La lectura", "La calidad de la lectura", "La identidad", "El tipo por parecido", "Las fallas", "Las preguntas solas"]) {
      expect(words).toContain(group)
    }
    expect(words).toContain("No legible Provisorio Debajo de esta proporción de palabras seguras un documento no se puede leer")
    expect(words).toContain("Tipo de confianza baja Provisorio")
    expect(words).toContain("Letras de diferencia en el nombre Provisorio")
    expect(words).toContain("Por omisión: 50 %")
    expect(words).toContain("Por omisión: 10, 60, 360, 1440 minutos")
    expect(words).not.toContain("Si lo cambiás")
    expect(words).not.toContain("Volver al valor por omisión")
    expect(words).toContain("Grupos por parecido Provisorio")
    expect(words).toContain("Vecinos muy cercanos Provisorio")
    expect(words).toContain("Por omisión: 85 %")
    expect(invalid({ ...BY_DEFAULT, groupSimilarity: 1.2 })).toBe("El parecido va de 0 a 1.")
  })

  it("says before saving what changing «no legible» does: which documents change side and how many questions", () => {
    const draft = { ...BY_DEFAULT, quality: 0.4 }
    const words = view(BY_DEFAULT, draft, { toLow: 0, toLowPages: 0, fromLow: 2, fromLowPages: 5, pagesWrittenAgain: 3 })
    expect(words).toContain("Si lo cambiás de 50 a 40 %:")
    expect(words).toContain("2 documentos dejan de ser de confianza baja (5 páginas)")
    expect(words).toContain("a lo sumo 3 páginas, una consulta cada una")
    expect(words).toContain("Volver al valor por omisión")
    expect(words).toContain("1 umbral cambiado, sin guardar.")
  })

  it("says that the bands apply from now on and ask nothing, and that the failures and the round add no question", () => {
    const words = view(BY_DEFAULT, { ...BY_DEFAULT, low: 0.85, roundCap: 100, retryMinutes: [5, 30] })
    expect(words).toContain("Si lo cambiás de 90 a 85 %: Se aplica a lo que se lea desde ahora. Lo ya leído no se vuelve a leer ni se recalcula (P-551): 0 consultas.")
    expect(words).toContain("Si lo cambiás de 50 a 100 : No suma consultas")
    expect(words).toContain("Si lo cambiás de 10, 60, 360, 1440 a 5, 30 minutos: Las consultas son las mismas, más pronto o más tarde: 0 consultas nuevas.")
    expect(words).toContain("3 umbrales cambiados, sin guardar.")
  })

  it("validates as the base does", () => {
    expect(invalid(BY_DEFAULT)).toBeNull()
    expect(invalid({ ...BY_DEFAULT, low: 0.98 })).toContain("El umbral bajo tiene que ser mayor que 0 y menor que el alto")
    expect(invalid({ ...BY_DEFAULT, quality: 1.2 })).toContain("entre 0 y 1")
    expect(invalid({ ...BY_DEFAULT, roundCap: 0 })).toContain("1 o más")
    expect(view(BY_DEFAULT, { ...BY_DEFAULT, low: 0.98 })).toContain("El umbral bajo tiene que ser mayor que 0")
  })

  it("reads what a person types: a share as a percentage, whole numbers, minutes separated by commas", () => {
    expect(parsed(info("quality"), "37,5")).toBe(0.375)
    expect(parsed(info("quality"), "")).toBeNull()
    expect(parsed(info("roundCap"), "2.5")).toBeNull()
    expect(parsed(info("retryMinutes"), "10, 60")).toEqual([10, 60])
    expect(parsed(info("retryMinutes"), "10, ")).toBeNull()
    expect(shown(info("nameShare"), 0.15)).toBe("15")
    expect(changed(BY_DEFAULT, { ...BY_DEFAULT, nameLetters: 3 })).toEqual(["nameLetters"])
  })
})
