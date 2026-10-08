import { beforeEach, describe, expect, it, vi } from "vitest"

const preprocessFile = vi.fn()
vi.mock("@/commands/fs", () => ({
  preprocessFile: (...args: unknown[]) => preprocessFile(...args),
}))

import { useWikiStore } from "@/stores/wiki-store"
import {
  ensureRecognizedText,
  getRecognitionConfig,
  isRecognizableSource,
} from "./text-recognition"

function configure(opts: { captioning: boolean; useMainLlm?: boolean; provider: string }) {
  const state = useWikiStore.getState()
  useWikiStore.setState({
    multimodalConfig: {
      ...state.multimodalConfig,
      enabled: opts.captioning,
      useMainLlm: opts.useMainLlm ?? true,
    },
    llmConfig: { ...state.llmConfig, provider: opts.provider, model: "gpt-test" } as typeof state.llmConfig,
  })
}

describe("text recognition", () => {
  beforeEach(() => {
    preprocessFile.mockReset()
    preprocessFile.mockResolvedValue("texto")
  })

  it("is off unless image captioning is on with the Codex CLI as main model", () => {
    configure({ captioning: false, provider: "codex-cli" })
    expect(getRecognitionConfig()).toBeNull()

    configure({ captioning: true, provider: "openai" })
    expect(getRecognitionConfig()).toBeNull()

    configure({ captioning: true, useMainLlm: false, provider: "codex-cli" })
    expect(getRecognitionConfig()).toBeNull()

    configure({ captioning: true, provider: "codex-cli" })
    expect(getRecognitionConfig()).toEqual({ engine: "codex-cli", model: "gpt-test" })
  })

  it("recognizes PDFs and PNG/JPEG images only", () => {
    expect(isRecognizableSource("/p/raw/sources/a/Contrato.PDF")).toBe(true)
    expect(isRecognizableSource("/p/raw/sources/foto.jpeg")).toBe(true)
    expect(isRecognizableSource("/p/raw/sources/captura.png")).toBe(true)
    expect(isRecognizableSource("/p/raw/sources/nota.docx")).toBe(false)
    expect(isRecognizableSource("/p/raw/sources/sin-extension")).toBe(false)
  })

  it("preprocesses a recognizable source with the engine config", async () => {
    configure({ captioning: true, provider: "codex-cli" })
    await ensureRecognizedText("/p/raw/sources/escaneo.pdf")
    expect(preprocessFile).toHaveBeenCalledWith("/p/raw/sources/escaneo.pdf", {
      engine: "codex-cli",
      model: "gpt-test",
    })
  })

  it("does nothing when recognition is off or the source is not recognizable", async () => {
    configure({ captioning: false, provider: "codex-cli" })
    await ensureRecognizedText("/p/raw/sources/escaneo.pdf")
    configure({ captioning: true, provider: "codex-cli" })
    await ensureRecognizedText("/p/raw/sources/nota.md")
    expect(preprocessFile).not.toHaveBeenCalled()
  })

  it("lets a recognition failure propagate", async () => {
    configure({ captioning: true, provider: "codex-cli" })
    preprocessFile.mockRejectedValue(new Error("Text recognition failed on page 2"))
    await expect(ensureRecognizedText("/p/raw/sources/escaneo.pdf")).rejects.toThrow("page 2")
  })

  it("reports the stage only when it is going to recognize", async () => {
    const onStage = vi.fn()
    configure({ captioning: true, provider: "codex-cli" })
    await ensureRecognizedText("/p/raw/sources/nota.docx", onStage)
    expect(onStage).not.toHaveBeenCalled()

    await ensureRecognizedText("/p/raw/sources/escaneo.pdf", onStage)
    expect(onStage).toHaveBeenCalledWith("Recognizing text...")
  })
})
