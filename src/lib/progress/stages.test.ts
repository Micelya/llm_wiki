import { describe, expect, it } from "vitest"
import type { ModelError, StageState } from "@/commands/docbase"
import { vocabularyFor } from "@/vocabulary"
import {
  activityLine,
  modelErrorText,
  ongoingCount,
  pageOf,
  stageDetails,
  stageFigures,
  stageShare,
  stageText,
  stateLabel,
  waitsFirstLap,
} from "./stages"
import { IDS, progress, stage } from "./progress.fixtures"

// Every figure below is made up.
const words = vocabularyFor("legal", "es")

const limit: ModelError = {
  stage: "reading",
  kind: "limit",
  message: "You've hit your usage limit",
  at: new Date(2026, 9, 21, 14, 5).getTime(),
  retryAt: new Date(2026, 9, 21, 14, 15).getTime(),
}

describe("the stages of «En curso» in words", () => {
  it("names every stage, with the matters as the vertical calls them", () => {
    for (const id of IDS) expect(stageText(id, words).title.length).toBeGreaterThan(0)
    expect(stageText("matters", words).title).toBe("Casos")
    expect(stageText("reading", words).explanation).toContain("personas y organizaciones")
    expect(stageText("recognition", words).explanation).toContain("cada página es una pregunta al modelo")
    expect(stageText("templates", words).title).toBe("Plantillas: nombres de los campos")
    expect(stageText("templates", words).explanation).toContain("Una pregunta por plantilla")
  })

  it("says how each stage stands, the scans like the others", () => {
    const labels = (["done", "running", "waiting", "paused", "error"] as StageState[]).map((state) => stateLabel({ id: "types", state }))
    expect(labels).toEqual(["Hecha", "En curso", "Esperando", "En pausa", "Con errores"])
    expect(stateLabel({ id: "recognition", state: "paused" })).toBe("En pausa")
  })

  it("gives the figures as done of total, in the unit of the stage", () => {
    expect(stageFigures(stage("reading", { done: 12, total: 68, pending: 56 }))).toBe("12 de 68 documentos")
    expect(stageFigures(stage("text", { done: 1, total: 1 }))).toBe("1 de 1 archivo")
    expect(stageFigures(stage("pages", { done: 0, total: 0 }))).toBe("Ninguna página todavía")
    expect(stageFigures(stage("types", { done: 0, total: 0 }))).toBe("Ningún documento todavía")
    // The templates, with the profession's word.
    expect(stageFigures(stage("templates", { done: 3, total: 12 }), words)).toBe("3 de 12 plantillas")
    expect(stageFigures(stage("templates", { done: 0, total: 0 }), words)).toBe("Ninguna plantilla todavía")
    expect(stageShare(stage("reading", { done: 17, total: 68 }))).toBe(0.25)
    expect(stageShare(stage("reading", { done: 0, total: 0 }))).toBeNull()
  })

  it("says that the matters and the pages wait for the first pass of the engine, not that they are done (D-452)", () => {
    const matters = stage("matters", { state: "paused", done: 12, total: 12, extra: { grouped: 4, loose: 8, firstLap: 1 } })
    expect(waitsFirstLap(matters)).toBe(true)
    expect(stateLabel(matters)).toBe("En pausa")
    expect(stageDetails(matters, words)[0]).toBe("esperando la primera vuelta del motor para saber si queda algo por hacer")
    expect(stageDetails(stage("pages", { state: "waiting", otherWork: true, extra: { firstLap: 1 } }), words)).toEqual([
      "esperando la primera vuelta del motor para saber si queda algo por hacer",
    ])
    // Its pass made, nothing of that is said.
    expect(waitsFirstLap(stage("matters"))).toBe(false)
    expect(stageDetails(stage("matters", { extra: { grouped: 4, loose: 8 } }), words)).toEqual(["4 en un caso", "8 sueltos"])
    // The reading still says what was read with another version or model (D-465).
    expect(stageDetails(stage("reading", { extra: { outdated: 134 } }), words)).toEqual([
      "134 leídos con otra versión de la pregunta o con otro modelo",
    ])
  })

  it("tells what is left, what failed and what is out", () => {
    expect(stageDetails(stage("text", { pending: 56, failed: 2, out: 4, state: "running" }), words)).toEqual([
      "faltan 56",
      "2 fallaron",
      "4 quedan afuera",
    ])
    expect(stageDetails(stage("recognition", { pending: 3, state: "paused" }), words)).toEqual(["3 esperan"])
    expect(
      stageDetails(stage("recognition", { pending: 12, failed: 1, state: "running", extra: { pages: 87, uncounted: 2 } }), words),
    ).toEqual(["12 esperan", "1 falló", "87 páginas por reconocer", "contando las páginas de 2 documentos"])
    expect(stageDetails(stage("matters", { extra: { grouped: 40, loose: 1 } }), words)).toEqual(["40 en un caso", "1 suelto"])
    expect(stageDetails(stage("resolve", { state: "running", extra: { questions: 3 } }), words)).toEqual([
      "3 preguntas pendientes al modelo",
    ])
    expect(stageDetails(stage("pages", { state: "running", otherWork: true }), words)).toEqual(["rehaciendo con lo nuevo"])
  })

  it("says the last error of the model in words, with its time and when it tries again", () => {
    const { what, when } = modelErrorText(limit)
    expect(what).toBe("El proveedor del modelo llegó a su límite de uso.")
    expect(when).toBe("Pasó a las 14:05. Vuelve a intentar a las 14:15.")
    expect(modelErrorText({ ...limit, kind: "key" }).what).toContain("rechazó la clave")
    expect(modelErrorText({ ...limit, kind: "connection" }).what).toContain("No hay conexión")
    expect(modelErrorText({ ...limit, kind: "other", retryAt: null }).when).toBe("Pasó a las 14:05.")
  })

  it("counts in the tab the stages that are not done", () => {
    expect(ongoingCount(null)).toBe(0)
    expect(ongoingCount(progress())).toBe(0)
    expect(ongoingCount(progress({ reading: { state: "running" }, matters: { state: "waiting" }, recognition: { state: "waiting" } }))).toBe(3)
  })
})

describe("the line at the foot of the window", () => {
  it("says what is being processed and how far it went", () => {
    const line = activityLine(progress({ reading: { state: "running", done: 12, total: 68, pending: 56 }, resolve: { state: "waiting" } }), words)
    expect(line).toEqual({ tone: "busy", text: "Procesando: Lectura con el modelo: a quiénes nombran, 12 de 68" })
  })

  it("says the error of the model before anything else", () => {
    const line = activityLine(progress({ reading: { state: "error", error: limit }, pages: { state: "waiting" } }), words)
    expect(line?.tone).toBe("error")
    expect(line?.text).toContain("límite de uso")
    expect(line?.text).toContain("Vuelve a intentar a las 14:15")
  })

  it("says when the automatic reading is paused, or a model is missing", () => {
    const paused = progress({ types: { state: "paused", pending: 68 } }, { paused: 68, backgroundReading: false })
    expect(activityLine(paused, words)).toEqual({ tone: "paused", text: "Lectura automática en pausa: 68 documentos esperan" })
    expect(activityLine({ ...paused, engineChosen: false }, words)?.text).toBe("Falta elegir un modelo: 68 documentos esperan")
  })

  it("says the scans being recognized, with the page", () => {
    const current = { status: "pending" as const, locationId: 1, contentId: 1, originId: "o", pagePath: null, name: "Escaneo.pdf", place: "O", detail: null, page: 2, pages: 8 }
    const line = activityLine(progress({ recognition: { state: "running", done: 3, total: 13, pending: 10, current } }), words)
    expect(line).toEqual({ tone: "busy", text: "Procesando: Texto de los escaneos, 3 de 13 (página 2 de 8)" })
    expect(pageOf(current)).toBe("página 2 de 8")
    expect(pageOf({ ...current, page: null })).toBeNull()
  })

  it("says when the scans wait for an engine that reads them", () => {
    const waiting = progress({ recognition: { state: "paused", pending: 3 } }, { paused: 3, recognitionEngine: null })
    expect(activityLine(waiting, words)).toEqual({ tone: "paused", text: "Falta elegir el motor que reconoce escaneos: 3 documentos esperan" })
    const off = progress({ recognition: { state: "paused", pending: 3 } }, { paused: 3, backgroundReading: false, recognitionEngine: "codex-cli:m" })
    expect(activityLine(off, words)?.text).toBe("Lectura automática en pausa: 3 documentos esperan")
  })

  it("says everything is up to date when nothing waits", () => {
    expect(activityLine(progress(), words)).toEqual({ tone: "ok", text: "Todo al día" })
  })

  it("says nothing before the first folder is incorporated", () => {
    expect(activityLine(progress({ origins: { total: 0, done: 0 } }), words)).toBeNull()
    expect(activityLine(null, words)).toBeNull()
  })
})
