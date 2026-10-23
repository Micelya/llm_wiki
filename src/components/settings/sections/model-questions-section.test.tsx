import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { EnginesCatalog, ModelQuestion } from "@/commands/docbase"
import catalog from "../../../../src-tauri/resources/catalog/ai-engines.json"
import type { DocbaseQuery } from "@/components/docbase/use-docbase-query"
import { NO_SERVICES } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { hooksOf } from "@/test-helpers/hook-order"
import { ModelQuestionsSection } from "./model-questions-section"

// Rendered once to text, a store shows the state its first render would
// read (`getInitialState`); here, the state each test sets.
vi.mock("zustand", async (importOriginal) => {
  const actual = await importOriginal<typeof import("zustand")>()
  const vanilla = await import("zustand/vanilla")
  const make = (createState: any) => {
    const api = vanilla.createStore(createState)
    api.getInitialState = api.getState
    const hook = (selector?: any) => actual.useStore(api, selector)
    return Object.assign(hook, api)
  }
  return { ...actual, create: (createState?: any) => (createState ? make(createState) : make) }
})

// What the base answers, as each test sets it: loading, failed or loaded.
const base = vi.hoisted(() => ({ query: null as unknown }))
vi.mock("@/components/docbase/use-docbase-query", () => ({ useDocbaseQuery: () => base.query }))

function answer(data: ModelQuestion[] | null, error: string | null = null): DocbaseQuery<ModelQuestion[]> {
  return { data, error, loading: !data && !error, reload: () => {} }
}

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

// Every name, number and model below is made up.
const codex = { engine: "codex-cli", connection: "codex-cli", model: "modelo-a", reasoning: "high" }

const reading: ModelQuestion = {
  id: "knowledge",
  name: "Leer un documento",
  purpose: "Registra lo que nombra el documento.",
  when: "Una vez por documento.",
  note: null,
  version: "conocimiento-7",
  catalogs: [
    { file: "knowledge.json", name: "Vocabulario", fromFolder: false, folderPath: "C:/datos/docbase/knowledge.json", unusedFile: true },
    { file: "document-types.json", name: "Tipos de documento", fromFolder: true, folderPath: "C:/datos/docbase/document-types.json", unusedFile: false },
  ],
  service: "ask",
  engine: codex,
  engineNote: null,
  example: [
    {
      title: "Pregunta",
      segments: [
        { text: "Leé el documento de abajo.\nArchivo: «", part: null },
        { text: "Ejemplo - carta.pdf", part: "el nombre del archivo" },
        { text: "»\n", part: null },
        { text: "Texto inventado de Ejemplo Uno S.A.", part: "el texto del documento" },
      ],
    },
  ],
  noExample: null,
  answerShape: '{\n  "type": "object"\n}',
  kept: true,
  keptByVersion: [
    { version: "conocimiento-7", count: 3 },
    { version: "conocimiento-6", count: 2 },
  ],
}

const recognition: ModelQuestion = {
  ...reading,
  id: "recognition",
  name: "Reconocer el texto de un escaneo",
  version: null,
  catalogs: [],
  service: "recognize",
  engine: null,
  engineNote: "Sin motor en «Reconocer escaneos», esta pregunta no se hace.",
  example: [{ title: "Pregunta", segments: [{ text: "Transcribí todo el texto visible.", part: null }] }],
  answerShape: null,
  kept: false,
  keptByVersion: [],
}

const chat: ModelQuestion = {
  ...recognition,
  id: "chat",
  name: "Contestar en el chat",
  service: "ask",
  engine: codex,
  engineNote: null,
  example: [
    { title: "Texto de sistema", segments: [{ text: "You are the backend Agent.", part: null }] },
    { title: "Mensaje", segments: [{ text: "¿Qué reclama Ejemplo Uno S.A.?", part: "lo que escribió la persona" }] },
  ],
}

const synthesis: ModelQuestion = { ...reading, id: "synthesis", name: "Lo que dicen los documentos", version: "sintesis-3", keptByVersion: [{ version: "sintesis-2", count: 4 }] }
const data: ModelQuestion = { ...reading, id: "data", name: "Los datos", version: "datos-1", keptByVersion: [] }

const questions = [reading, data, synthesis, recognition, chat]

function shown(query: DocbaseQuery<ModelQuestion[]>): { html: string; words: string } {
  base.query = query
  const html = renderToStaticMarkup(<ModelQuestionsSection />)
  return { html, words: text(html) }
}

beforeEach(() => {
  base.query = answer(null)
  useAiServicesStore.setState({ services: NO_SERVICES, catalog: catalog as EnginesCatalog, loaded: true })
})

describe("Ajustes → Lo que se le pregunta al modelo", () => {
  it("calls the same hooks in every state, so going from loading to loaded does not break the page", () => {
    const states = [answer(null), answer(null, "La base no respondió"), answer([]), answer(questions)]
    const traces = states.map((query) => {
      base.query = query
      return hooksOf(ModelQuestionsSection, {})
    })
    for (const trace of traces) expect(trace).toEqual(traces[0])
  })

  it("says it is reading, and the problem when the base fails", () => {
    expect(shown(answer(null)).words).toContain("Leyendo las preguntas…")
    expect(shown(answer(null, "La base no respondió")).words).toContain("No se pudieron leer las preguntas: La base no respondió")
  })

  it("shows one row per question with its name, its version and its engine, and nothing to save", () => {
    const { html, words } = shown(answer(questions))
    expect(words).toContain("Lo que se le pregunta al modelo")
    expect(words).toContain("solo para mirar")
    expect(html.match(/<details class="group rounded-md border">/g)).toHaveLength(questions.length)
    expect(words).toContain("Leer un documento conocimiento-7 Codex (modelo-a, razonamiento alto)")
    expect(words).toContain("Reconocer el texto de un escaneo sin versión Ninguno")
    expect(words).toContain("Sin motor en «Reconocer escaneos», esta pregunta no se hace.")
    expect(html).not.toMatch(/<(input|select|textarea|button)\b/)
  })

  it("marks what changes in each case where the question puts it, and leaves the rest as it is", () => {
    const { html, words } = shown(answer([reading]))
    expect(html).toContain('<mark class="rounded bg-warn-bg px-0.5 text-inherit ring-1 ring-warn-line" title="Cambia en cada caso: el nombre del archivo">Ejemplo - carta.pdf</mark>')
    expect(html).toContain("<span>Leé el documento de abajo.\nArchivo: «</span>")
    expect(words).toContain("Cambia en cada caso: el nombre del archivo; el texto del documento.")
    expect(words).toContain("La forma de la respuesta que pide")
    // A question that never changes says so; the chat shows its two texts.
    const others = shown(answer([recognition, chat])).words
    expect(others).toContain("Sale igual en todos los casos.")
    expect(others).toContain("Texto de sistema You are the backend Agent.")
    expect(others).toContain("Mensaje ¿Qué reclama Ejemplo Uno S.A.?")
  })

  it("says which catalog is in use, the data folder's or the app's, and a file that is not in use", () => {
    const { words } = shown(answer([reading]))
    expect(words).toContain("Vocabulario ( knowledge.json ): el que viene con la app")
    expect(words).toContain("Hay un archivo en la carpeta de datos que no se está usando")
    expect(words).toContain("Tipos de documento ( document-types.json ): el de la carpeta de datos (C:/datos/docbase/document-types.json)")
    expect(shown(answer([recognition])).words).toContain("Catálogos ninguno")
  })

  it("counts what the base keeps of each version and warns that the last one carries the firm's documents", () => {
    const { words } = shown(answer(questions))
    expect(words).toContain("Guardadas: 3 de conocimiento-7 (la vigente) · 2 de conocimiento-6.")
    expect(words).toContain("Guardadas: 4 de sintesis-2. Ninguna de la versión vigente.")
    expect(words).toContain("Todavía no se guardó ninguna.")
    expect(words).toContain("Ver la última de la versión vigente (contiene texto de documentos del estudio)")
    expect(words).toContain("Esta pregunta no se guarda: no hay una última para mostrar.")
    // Its text is read only when a person opens it.
    expect(words).not.toContain("Salió el")
  })
})
