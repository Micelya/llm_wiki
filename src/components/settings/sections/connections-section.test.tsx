import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { CoreConnectionFields, CoreConnectionTest, CoreConnectionView } from "@/commands/micelya-core"
import { coreConnectionProblems, coreFieldsChanged, coreTestText } from "@/lib/micelya-core"
import { useCoreConnectionStore } from "@/stores/core-connection-store"
import { ConnectionsSection } from "./connections-section"

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

// Every value below is made up.
const fields: CoreConnectionFields = {
  api: "https://api.ejemplo.invalid/v1",
  issuer: "https://emisor.ejemplo.invalid/realms/prueba",
  tenant: "organizacion-de-prueba",
  clientId: "cliente-de-prueba",
}
const kept: CoreConnectionView = { ...fields, environment: "ambiente-de-prueba", hasSecret: true }
const passed = (change: Partial<CoreConnectionTest>): CoreConnectionTest => ({
  ok: false,
  step: null,
  problem: null,
  status: null,
  detail: null,
  readings: null,
  moreReadings: false,
  ...change,
})

function shown(): { html: string; words: string } {
  const html = renderToStaticMarkup(<ConnectionsSection />)
  return { html, words: text(html) }
}

beforeEach(() => {
  useCoreConnectionStore.setState({ connection: null, loaded: true, problem: null })
})

describe("Ajustes → Conexiones → Micelya Core", () => {
  it("asks for every datum when there is no connection", () => {
    const { html, words } = shown()
    expect(words).toContain("Conexiones")
    expect(words).toContain("Micelya Core Sin cargar")
    for (const label of ["Dirección de la API", "Emisor", "Organización", "Identificador del cliente", "Secreto del cliente"]) {
      expect(words).toContain(label)
    }
    expect(html).toContain('type="password"')
    expect(words).toContain("Probar la conexión")
    expect(words).not.toContain("Quitar la conexión")
  })

  it("shows what is kept, never the secret, which is changed apart", () => {
    useCoreConnectionStore.setState({ connection: kept })
    const { html, words } = shown()
    expect(words).toContain("Micelya Core Cargada")
    expect(html).toContain('value="https://api.ejemplo.invalid/v1"')
    expect(html).toContain('value="organizacion-de-prueba"')
    expect(words).toContain("Ambiente: ambiente-de-prueba")
    expect(words).toContain("Guardado en el llavero del sistema. Cambiar")
    expect(html).not.toContain('type="password"')
    expect(words).toContain("Quitar la conexión")
  })

  it("says what is missing or wrong in the data on screen", () => {
    expect(coreConnectionProblems(fields, "", true)).toEqual([])
    expect(coreConnectionProblems(fields, "", false)).toEqual(["Falta el secreto del cliente."])
    expect(coreConnectionProblems({ ...fields, api: "http://api.ejemplo.invalid/v1", issuer: "emisor" }, "s", false)).toEqual([
      "La dirección de la API tiene que empezar con https://.",
      "El emisor no es una dirección web.",
    ])
    expect(coreConnectionProblems({ ...fields, tenant: " ", clientId: "dos palabras" }, "s", false)).toEqual([
      "Falta la organización.",
      "El identificador del cliente no puede tener espacios.",
    ])
    expect(coreConnectionProblems({ ...fields, api: "http://127.0.0.1:8080/v1" }, "", true)).toEqual([])
    expect(coreFieldsChanged(fields, fields, "")).toBe(false)
    expect(coreFieldsChanged({ ...fields, tenant: "otra" }, fields, "")).toBe(true)
    expect(coreFieldsChanged(fields, fields, "secreto-nuevo")).toBe(true)
  })

  it("tells in plain words what the test found", () => {
    expect(coreTestText(passed({ ok: true, readings: 0 }), fields)).toBe(
      "Anduvo: Micelya Core dio el permiso de acceso y la lista de lecturas. Core no guarda ninguna lectura de esta credencial.",
    )
    expect(coreTestText(passed({ ok: true, readings: 100, moreReadings: true }), fields)).toContain("guarda más de 100 lecturas")
    expect(coreTestText(passed({ step: "permit", problem: "unreachable" }), fields)).toBe(
      "No se pudo llegar al emisor (emisor.ejemplo.invalid): revisá su dirección y la conexión a internet.",
    )
    expect(coreTestText(passed({ step: "permit", problem: "rejected", status: 401 }), fields)).toBe(
      "El emisor rechazó el identificador del cliente o el secreto: revisalos.",
    )
    expect(coreTestText(passed({ step: "permit", problem: "organization" }), fields)).toContain("no reconoce la organización")
    expect(coreTestText(passed({ step: "readings", problem: "forbidden", status: 403 }), fields)).toContain(
      "no tiene permiso para leer documentos",
    )
    expect(coreTestText(passed({ step: "readings", problem: "not-found", status: 404 }), fields)).toContain("no está la API de Core")
    expect(coreTestText(passed({ step: "readings", problem: "unavailable", status: 503 }), fields)).toContain("no está disponible ahora")
    expect(coreTestText(passed({ step: "readings", problem: "other", status: 418 }), fields)).toBe(
      "La API de Core respondió algo inesperado (418).",
    )
  })
})
