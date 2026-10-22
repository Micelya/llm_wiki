import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { AiServices, TypesPlan } from "@/commands/docbase"
import { NO_SERVICES } from "@/lib/ai-services"
import { MISSING_ASK_ENGINE } from "@/lib/document-asking"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { hooksOf } from "@/test-helpers/hook-order"
import { DocumentTypesPanel } from "./document-types-view"
import type { DocbaseQuery } from "./use-docbase-query"

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
vi.mock("./use-docbase-query", () => ({ useDocbaseQuery: () => base.query }))

function answer(data: TypesPlan | null, error: string | null = null): DocbaseQuery<TypesPlan> {
  return { data, error, loading: !data && !error, reload: () => {} }
}

// Every number and name below is made up.
const plan: TypesPlan = {
  documents: 12,
  typed: 4,
  typeQuestions: 3,
  dataQuestions: 2,
  proposedTypes: [{ id: "constancia-inventada", name: "Constancia inventada", description: null, documents: 2 }],
  engine: null,
}

const withEngine: AiServices = {
  ...NO_SERVICES,
  ask: { engine: { engine: "codex-cli", connection: "codex-cli", model: "modelo-a", reasoning: "" }, alternative: null },
}

const states: Array<[string, DocbaseQuery<TypesPlan>, AiServices]> = [
  ["loading", answer(null), NO_SERVICES],
  ["failed", answer(null, "La base no respondió"), NO_SERVICES],
  ["without documents", answer({ ...plan, documents: 0, typed: 0, typeQuestions: 0, dataQuestions: 0, proposedTypes: [] }), NO_SERVICES],
  ["loaded, without engine", answer(plan), NO_SERVICES],
  ["loaded, with engine", answer(plan), withEngine],
  ["up to date", answer({ ...plan, typed: 12, typeQuestions: 0, dataQuestions: 0, proposedTypes: [] }), withEngine],
]

function shown(query: DocbaseQuery<TypesPlan>, services: AiServices = NO_SERVICES): string {
  base.query = query
  useAiServicesStore.setState({ services })
  return renderToStaticMarkup(<DocumentTypesPanel />)
}

beforeEach(() => {
  base.query = answer(null)
  useAiServicesStore.setState({ services: NO_SERVICES })
})

describe("Orígenes → Tipos de documento", () => {
  it("calls the same hooks in every state, so going from loading to loaded does not break the page", () => {
    const traces = states.map(([name, query, services]) => {
      base.query = query
      useAiServicesStore.setState({ services })
      return [name, hooksOf(DocumentTypesPanel, {})] as const
    })
    const [, first] = traces[0]
    expect(first.length).toBeGreaterThan(0)
    for (const [name, trace] of traces) expect({ name, trace }).toEqual({ name, trace: first })
  })

  it("shows nothing while loading or without documents, and the problem when the base fails", () => {
    expect(shown(answer(null))).toBe("")
    expect(shown(states[2][1])).toBe("")
    expect(shown(answer(null, "La base no respondió"))).toContain("La base no respondió")
  })

  it("without an engine, says what is missing and does not let the run start", () => {
    const html = shown(answer(plan))
    expect(html).toContain("4 de 12 documentos tienen tipo y etapa")
    expect(html).toContain("3 preguntas sobre tipo y etapa y 2 preguntas sobre datos")
    expect(html).toContain("Reconocer tipos")
    expect(html).toContain('disabled=""')
    expect(html).toContain(MISSING_ASK_ENGINE)
    expect(html).toContain("Constancia inventada")
  })

  it("with an engine, lets the run start", () => {
    const html = shown(answer(plan), withEngine)
    expect(html).toContain("Reconocer tipos")
    expect(html).not.toContain('disabled=""')
    expect(html).not.toContain(MISSING_ASK_ENGINE)
  })

  it("says it is up to date when nothing is left to ask", () => {
    const html = shown(states[5][1], withEngine)
    expect(html).toContain("Al día")
    expect(html).not.toContain("Reconocer tipos")
  })
})
