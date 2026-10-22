import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { AiServices, EngineChoice, EnginesCatalog } from "@/commands/docbase"
import catalog from "../../../../src-tauri/resources/catalog/ai-engines.json"
import { NO_SERVICES } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { useCoreConnectionStore } from "@/stores/core-connection-store"
import { useWikiStore } from "@/stores/wiki-store"
import { AiServicesSection } from "./ai-services-section"

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

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

// Every model name below is made up.
const choice = (engine: string, connection: string, model = "", reasoning = ""): EngineChoice => ({ engine, connection, model, reasoning })
const services: AiServices = {
  ask: { engine: choice("codex-cli", "codex-cli", "modelo-a", "high"), alternative: null },
  recognize: { engine: choice("codex-cli", "codex-cli", "modelo-a"), alternative: null },
  research: { engine: choice("tavily", "tavily"), alternative: null },
}
const coreFields = { api: "https://api.ejemplo.invalid/v1", issuer: "https://emisor.ejemplo.invalid/realms/prueba", tenant: "t", clientId: "c" }

function shown(): { html: string; words: string } {
  const html = renderToStaticMarkup(<AiServicesSection onOpen={() => {}} />)
  return { html, words: text(html) }
}

beforeEach(() => {
  useAiServicesStore.setState({ services, catalog: catalog as EnginesCatalog, loaded: true })
  useWikiStore.setState({ taskModelRouting: { chatPresetId: null, ingestPresetId: null }, customLlmPresets: [], providerConfigs: {} })
  useCoreConnectionStore.setState({ connection: null, loaded: true, problem: null })
})

describe("Ajustes → Servicios de IA", () => {
  it("shows each service with its engine, model and reasoning", () => {
    const { html, words } = shown()
    for (const service of ["Preguntar al modelo", "Reconocer escaneos", "Investigar en internet"]) expect(words).toContain(service)
    expect(html).toMatch(/<option value="codex-cli" selected="">Codex CLI \(local\)<\/option>/)
    expect(html).toContain('value="modelo-a"')
    expect(html).toMatch(/<option value="high" selected="">alto<\/option>/)
    expect(html).toMatch(/<option value="tavily" selected="">Tavily \(faltan sus datos\)<\/option>/)
    expect(words).toContain("Motor alternativo (opcional): Ninguno")
  })

  it("says what each engine can read, where it processes and what its provider keeps", () => {
    const { words } = shown()
    expect(words).toContain("Sabe leer Texto impreso: sí · Tablas: sí · Casillas marcadas: a veces · Manuscrito: a veces")
    expect(words).toContain("Dónde se procesa")
    expect(words).toContain("Qué guarda el proveedor")
    expect(words).toContain("Se mandan a Tavily las palabras de la búsqueda")
    expect(words).toContain("La síntesis de lo encontrado la escribe el motor de «Preguntar al modelo».")
  })

  it("lists the connections in use with the services that use each one", () => {
    const { words } = shown()
    expect(words).toContain("Codex CLI (local) — Preguntar al modelo, Reconocer escaneos")
    expect(words).toContain("Tavily — Investigar en internet")
  })

  it("warns when the base cannot run the engine, when there is none, and when the chat uses another", () => {
    useAiServicesStore.setState({
      services: { ...NO_SERVICES, ask: { engine: choice("openai", "openai", "modelo-b"), alternative: null } },
    })
    useWikiStore.setState({ taskModelRouting: { chatPresetId: "claude-code-cli", ingestPresetId: null } })
    const { words } = shown()
    expect(words).toContain("La base todavía no lee con este motor")
    expect(words).toContain("Sin motor, los escaneos quedan sin texto.")
    expect(words).toContain("Sin buscador no se puede investigar en internet.")
    expect(words).toContain("El chat usa otro motor, elegido antes: Claude Code CLI")
  })

  it("offers Micelya Core to recognize scans only with its connection, and says where it processes", () => {
    useCoreConnectionStore.setState({ connection: null, loaded: true })
    let { html } = shown()
    expect(html).toMatch(/<option value="core" disabled="">Micelya Core \(Textract\) \(falta la conexión\)<\/option>/)

    useAiServicesStore.setState({
      services: { ...services, recognize: { engine: choice("core", "micelya-core", "textract-1.0"), alternative: null } },
    })
    let words = shown().words
    expect(words).toContain("Falta la conexión con Micelya Core: sin ella no se lee nada con este motor. Ir a Conexiones")

    useCoreConnectionStore.setState({ connection: { ...coreFields, environment: "", hasSecret: true } })
    ;({ html, words } = shown())
    expect(html).toMatch(/<option value="core" selected="">Micelya Core \(Textract\)<\/option>/)
    expect(words).not.toContain("Falta la conexión con Micelya Core")
    expect(words).toContain("Modelo textract-1.0 (el que usa el proveedor")
    expect(words).toContain("Procesa en Oregón (Estados Unidos).")
    expect(words).toContain("Textract guarda el resultado 7 días. Desktop borra cada lectura en Core apenas la guarda.")
    expect(words).toContain("Texto impreso: sí · Tablas: no · Casillas marcadas: no · Manuscrito: a veces")
    expect(words).not.toContain("La base todavía no reconoce escaneos con este motor.")
    expect(words).toContain("Micelya Core — Reconocer escaneos")
  })

  it("says so when the base cannot be read", () => {
    useAiServicesStore.setState({ loaded: false, catalog: null })
    expect(shown().words).toContain("la base documental no está disponible")
  })
})
