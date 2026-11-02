import { describe, expect, it } from "vitest"
import type { ProcessView, StepState } from "@/commands/docbase"
import { backLine, statusLine, stepGroups, stepLine } from "./process"

// Every figure below is made up.
const NAMES: [StepState["about"]["key"], string, "file" | "document"][] = [
  ["inventory", "Inventario de las carpetas", "file"],
  ["file-hash", "Huella del archivo", "file"],
  ["text", "Texto", "file"],
  ["scans", "Lectura de escaneos", "file"],
  ["shapes", "Ficha de las páginas", "file"],
  ["zones", "Lectura por zonas", "file"],
  ["cut-rules", "Corte con reglas", "file"],
  ["cut-question", "Pregunta del corte", "file"],
  ["parts", "Partes", "file"],
  ["signature", "Firma", "document"],
  ["vectors", "Vectores", "document"],
  ["similar", "Parecidos y familias", "document"],
  ["type", "Tipo", "document"],
  ["type-data", "Datos del tipo", "document"],
  ["template-fields", "Campos de las plantillas", "document"],
  ["reading", "Lectura de entidades", "document"],
  ["identity", "Identidad", "document"],
  ["matters", "Asuntos", "document"],
  ["pages", "Páginas", "document"],
]

function step(number: number, count: Partial<StepState["count"]> = {}, held: string | null = null): StepState {
  const [key, name, level] = NAMES[number - 1]
  return {
    about: { number, key, name, level, class: "local" },
    class: "local",
    count: { pending: 0, setAside: 0, person: 0, low: 0, withoutAsking: 0, known: true, ...count },
    held,
  }
}

function view(over: Partial<ProcessView> = {}, steps: StepState[] = NAMES.map((_, i) => step(i + 1))): ProcessView {
  return {
    status: { state: "stopped", step: null, operation: null, scope: null, back: null, waitingFor: null, turns: 3 },
    state: { steps, firstIncomplete: null, unknown: [] },
    totals: { places: 3, files: 2, documents: 4, typed: 4 },
    aside: {},
    ...over,
  }
}

describe("«En curso» with the 19 steps", () => {
  it("says how the loop stands in the line above", () => {
    expect(statusLine(view())).toEqual({
      state: "Detenido",
      detail: "todo completo · sigue solo cuando aparezca un archivo nuevo o si pedís volver a procesar",
    })
    const working = view({
      status: { state: "working", step: 8, operation: "paso 8 de 19, pregunta del corte", scope: "todo", back: null, waitingFor: null, turns: 9 },
    })
    expect(statusLine(working)).toEqual({ state: "Procesando", detail: "paso 8 de 19, pregunta del corte" })
    const person = view({
      status: { state: "waiting-person", step: 8, operation: null, scope: null, back: null, waitingFor: "esperando a una persona: la ronda de preguntas llegó a su tope (¿seguir?)", turns: 9 },
    })
    expect(statusLine(person).state).toBe("Esperando a una persona")
    expect(statusLine(view({ status: { ...view().status, state: "waiting", waitingFor: null } }), ).state).toBe("Esperando")
    // Stopped with questions held by a setting: not «todo completo».
    const steps = NAMES.map((_, i) => step(i + 1, i === 11 ? { setAside: 2 } : {}, i === 11 ? "la lectura automática está apagada" : null))
    expect(statusLine(view({}, steps))).toEqual({
      state: "Detenido",
      detail: "lo que se hace sin IA está completo · lo demás espera: la lectura automática está apagada",
    })
  })

  it("says why it went back to the first step the last time", () => {
    expect(backLine(view())).toBeNull()
    const back = view({ status: { ...view().status, back: "volvió al paso 1 (inventario de las carpetas) después del paso 12 (tipo): un archivo nuevo o cambiado en «Clientes»" } })
    expect(backLine(back)).toBe(
      "La última vez volvió al paso 1 (inventario de las carpetas) después del paso 12 (tipo): un archivo nuevo o cambiado en «Clientes».",
    )
  })

  it("puts the 19 steps in two groups, with the same totals as Inicio, and no line of the barrier", () => {
    const groups = stepGroups(view())
    expect(groups.map((g) => [g.title, g.steps.length])).toEqual([
      ["Cada archivo", 9],
      ["Cada documento", 10],
    ])
    expect(groups[0].counts).toBe("2 archivos distintos en 3 lugares")
    // A file of 3 parts and a file alone: 4 documents, not 5.
    expect(groups[1].counts).toBe("4 documentos (cada parte de un archivo compuesto cuenta como uno)")
    expect(JSON.stringify(groups)).not.toMatch(/barrera/i)
  })

  it("tells each step: complete, working, pending, set aside by failure, by a person or by low confidence", () => {
    const base = view()
    expect(stepLine(step(1), base)).toEqual({ mark: "complete", text: "completo", attention: false })
    const working = view({ status: { ...base.status, state: "working", step: 3 } })
    expect(stepLine(step(3, { pending: 2 }), working).text).toBe("trabajando · 2 pendientes")
    expect(stepLine(step(3, { pending: 2 }), base)).toEqual({ mark: "pending", text: "2 pendientes", attention: false })
    expect(stepLine(step(12, { setAside: 1 }), base)).toEqual({ mark: "aside", text: "1 apartado por falla", attention: true })
    expect(stepLine(step(4, { setAside: 2, person: 2 }), base)).toEqual({
      mark: "person",
      text: "2 esperando a una persona",
      attention: true,
    })
    expect(stepLine(step(14, { setAside: 3, low: 3 }), base)).toEqual({
      mark: "aside",
      text: "3 apartados por confianza baja",
      attention: true,
    })
    // Questions held by a setting wait, and are not a problem of Atención.
    expect(stepLine(step(12, { setAside: 5 }, "la lectura automática está apagada"), base)).toEqual({
      mark: "aside",
      text: "5 esperando: la lectura automática está apagada",
      attention: false,
    })
    // The round past its cap: the step waits for a person.
    const round = view({ aside: { "8": "esperando a una persona: la ronda de preguntas llegó a su tope (¿seguir?)" } })
    expect(stepLine(step(8, { setAside: 4 }, "apartado por ahora"), round)).toEqual({
      mark: "person",
      text: "4 esperando a una persona",
      attention: true,
    })
    // The steps that know what is left only by going over them.
    expect(stepLine(step(17, { known: false }), base).mark).toBe("unknown")
  })
})
