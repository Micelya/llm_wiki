import { afterEach, describe, expect, it } from "vitest"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { NO_SERVICES } from "@/lib/ai-services"
import { scansWaiting } from "@/components/docbase/background-reading-panel"
import { progress } from "@/lib/progress/progress.fixtures"
import {
  engineName,
  getRecognitionConfig,
  illegibleParts,
  isRecognitionEngine,
  pageList,
  recognitionEngineId,
  recognitionFailureText,
  withoutRecognitionMarkers,
} from "./text-recognition"

// Every text below is made up.
afterEach(() => useAiServicesStore.setState({ services: NO_SERVICES }))

function recognizeWith(engine: string, model: string) {
  useAiServicesStore.setState({
    services: { ...NO_SERVICES, recognize: { engine: { engine, connection: engine, model, reasoning: "" }, alternative: null } },
  })
}

describe("the engine that recognizes scans", () => {
  it("is the one of «Reconocer escaneos» when the base can run it, and none otherwise", () => {
    recognizeWith("codex-cli", "modelo-de-prueba")
    expect(getRecognitionConfig()).toEqual({ engine: "codex-cli", model: "modelo-de-prueba" })
    expect(recognitionEngineId(getRecognitionConfig())).toBe("codex-cli:modelo-de-prueba")
    recognizeWith("otro-motor", "otro")
    expect(getRecognitionConfig()).toBeNull()
    useAiServicesStore.setState({ services: NO_SERVICES })
    expect(getRecognitionConfig()).toBeNull()
    expect(recognitionEngineId(null)).toBeNull()
    expect(recognitionEngineId({ engine: "codex-cli", model: " " })).toBe("codex-cli")
  })

  it("is named in words, and told apart from plain extraction", () => {
    expect(engineName("codex-cli:modelo-de-prueba")).toBe("Codex (modelo-de-prueba)")
    expect(engineName("codex-cli")).toBe("Codex")
    expect(isRecognitionEngine("codex-cli:m")).toBe(true)
    expect(isRecognitionEngine("extraction")).toBe(false)
    expect(isRecognitionEngine(null)).toBe(false)
  })
})

describe("what a recognition says", () => {
  it("tells why it failed in words", () => {
    expect(recognitionFailureText("Text recognition failed on page 2: You've hit your usage limit")).toBe(
      "El proveedor del modelo llegó a su límite de uso (página 2).",
    )
    expect(recognitionFailureText("Codex did not answer within 300 seconds")).toBe("El motor no respondió a tiempo, o no hay conexión.")
    expect(recognitionFailureText("`codex` not found on PATH")).toContain("No se encontró el programa del motor")
    expect(recognitionFailureText("The file cannot be reached: its folder is not connected")).toContain("No se encuentra el archivo")
    expect(recognitionFailureText("Text recognition failed on page 4: Page 4 could not be rendered: x")).toBe(
      "No se pudo convertir la página en imagen: el PDF puede estar dañado o protegido (página 4).",
    )
    expect(recognitionFailureText("Codex returned an empty answer")).toBe("El motor no devolvió texto.")
    expect(recognitionFailureText("algo raro")).toBe("El motor que reconoce el texto falló.")
  })

  it("finds the pages the engine could not read", () => {
    const text = "## Page 1\n\nTodo claro.\n\n## Page 2\n\nUn monto [ilegible].\n\n## Page 3\n\nOtro [ilegible] más.\n"
    expect(illegibleParts(text)).toEqual({ pages: [2, 3] })
    expect(illegibleParts("Una foto con algo [ilegible].")).toEqual({ pages: [] })
    expect(illegibleParts("Todo claro.")).toBeNull()
    expect(illegibleParts(null)).toBeNull()
    expect(pageList([2, 3, 7])).toBe("páginas 2, 3 y 7")
    expect(pageList([4])).toBe("página 4")
  })

  it("says, before the automatic reading is turned on, how many scans and pages wait", () => {
    expect(scansWaiting(progress({ recognition: { pending: 12, failed: 1, extra: { pages: 87 } } }))).toBe(
      "Hay 13 escaneos por reconocer (87 páginas, una pregunta al modelo por página). ",
    )
    expect(scansWaiting(progress({ recognition: { pending: 1 } }))).toBe("Hay 1 escaneo por reconocer. ")
    expect(scansWaiting(progress())).toBe("")
    expect(scansWaiting(null)).toBe("")
  })

  it("leaves out the marks of the engine on each page", () => {
    const text = '## Page 1\n\n<!-- texto-reconocido motor="codex-cli:m" -->\nPrimera hoja.\n'
    expect(withoutRecognitionMarkers(text)).toBe("## Page 1\n\nPrimera hoja.\n")
  })
})
