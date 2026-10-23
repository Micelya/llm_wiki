import { describe, expect, it } from "vitest"
import { parseOverview } from "./overview"

// Every name and figure below is made up.
const written = `---
type: overview
title: "Resumen del archivo"
generated_by: micelya
updated: 2026-10-23
---

# Resumen del archivo

Lo que el registro sabe del archivo, armado por la app desde los documentos.

- Documentos leídos: 1284
- Documentos cuya lectura falló: 3
- Con página: Persona: 212 · Organización: 64 · Asunto: 2

## Los más nombrados

- [[persona-uno-ab12|Persona Uno]] · Persona — 96 documentos
- [[empresa-de-prueba-cd34|Empresa de Prueba S.A.]] · Organización — 1 documento

## Casos

- [[caso-de-prueba-ef56|Prueba c/ Ejemplo s/ cobro]] — 14 documentos
- [[otro-caso-gh78|Otro caso inventado]] — 9 documentos
`

describe("the overview of the archive in Inicio", () => {
  it("reads its figures, who has a page by kind, the most named and the matters", () => {
    const overview = parseOverview(written)
    expect(overview?.figures).toEqual([
      { label: "Documentos leídos", value: 1284 },
      { label: "Documentos cuya lectura falló", value: 3 },
    ])
    expect(overview?.withPage).toEqual([
      { kind: "Persona", count: 212 },
      { kind: "Organización", count: 64 },
      { kind: "Asunto", count: 2 },
    ])
    expect(overview?.mostNamed).toEqual([
      { page: "persona-uno-ab12", name: "Persona Uno", kind: "Persona", documents: 96 },
      { page: "empresa-de-prueba-cd34", name: "Empresa de Prueba S.A.", kind: "Organización", documents: 1 },
    ])
    expect(overview?.matters.map((item) => [item.name, item.documents, item.kind])).toEqual([
      ["Prueba c/ Ejemplo s/ cobro", 14, null],
      ["Otro caso inventado", 9, null],
    ])
  })

  it("reads an archive without names yet", () => {
    const overview = parseOverview("# Resumen del archivo\n\nAlgo.\n\n- Documentos leídos: 0\n- Con página: \n")
    expect(overview).toEqual({ figures: [{ label: "Documentos leídos", value: 0 }], withPage: [], mostNamed: [], matters: [] })
  })

  it("gives up on a text of another shape, so that it is shown as an article", () => {
    expect(parseOverview("# Resumen\n\nUn archivo de prueba.")).toBeNull()
    expect(parseOverview(written.replace("— 14 documentos", "(catorce)"))).toBeNull()
  })
})
