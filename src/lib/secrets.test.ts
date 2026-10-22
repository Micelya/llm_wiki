import { beforeEach, describe, expect, it, vi } from "vitest"

// Every key below is made up. The keychain is a fake one in memory.
const mocks = vi.hoisted(() => {
  const file = new Map<string, unknown>()
  const keychain = new Map<string, string>()
  const state = { broken: false }
  return {
    file,
    keychain,
    state,
    invoke: vi.fn(async (command: string, args: Record<string, unknown>) => {
      if (state.broken) throw new Error("no keychain")
      if (command === "secrets_read") {
        const names = args.names as string[]
        return Object.fromEntries(names.filter((n) => keychain.has(n)).map((n) => [n, keychain.get(n)]))
      }
      if (command === "secrets_write") {
        const entries = args.entries as Record<string, string>
        for (const [name, value] of Object.entries(entries)) {
          if (value) keychain.set(name, value)
          else keychain.delete(name)
        }
        return Object.keys(entries)
      }
      throw new Error(`unexpected command ${command}`)
    }),
  }
})

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }))
vi.mock("@tauri-apps/plugin-store", () => ({
  load: async () => ({
    get: async (key: string) => structuredClone(mocks.file.get(key)),
    set: async (key: string, value: unknown) => void mocks.file.set(key, structuredClone(value)),
    save: async () => {},
  }),
}))

import { __secretsTest, keepKeys, keyNamesOf } from "./secrets"
import {
  loadLlmConfig,
  loadProviderConfigs,
  loadSearchApiConfig,
  loadSettingsKeys,
  saveLlmConfig,
  saveProviderConfigs,
  saveSearchApiConfig,
} from "./project-store"
import type { LlmConfig, SearchApiConfig } from "@/stores/wiki-store"

const llm: LlmConfig = {
  provider: "openai",
  apiKey: "clave-principal",
  model: "m",
  ollamaUrl: "http://localhost:11434",
  customEndpoint: "",
  maxContextSize: 8192,
}
const search: SearchApiConfig = {
  provider: "tavily",
  apiKey: "clave-busqueda",
  providerConfigs: { tavily: { apiKey: "clave-busqueda" }, searxng: { searXngUrl: "http://localhost:8080" } },
}

beforeEach(() => {
  mocks.file.clear()
  mocks.keychain.clear()
  mocks.state.broken = false
  mocks.invoke.mockClear()
  __secretsTest.reset(null)
})

function fileText(): string {
  return JSON.stringify(Object.fromEntries(mocks.file))
}

describe("the names of the keys", () => {
  it("are those of the places of the settings, as in Rust", () => {
    expect(
      keyNamesOf({
        llmConfig: llm,
        providerConfigs: { openai: { apiKey: "x" }, "custom-1": {} },
        searchApiConfig: search,
      }),
    ).toEqual(["llm", "provider:openai", "provider:custom-1", "search", "search:tavily", "search:searxng"])
    expect(keyNamesOf({})).toEqual([])
  })
})

describe("the keys of the settings", () => {
  it("go to the keychain and the file keeps the rest", async () => {
    expect(await loadSettingsKeys()).toBe(true)
    await saveLlmConfig(llm)
    await saveProviderConfigs({ openai: { apiKey: "clave-principal", model: "m" }, "codex-cli": { model: "m" } })
    await saveSearchApiConfig(search)
    expect(fileText()).not.toContain("clave-")
    expect(mocks.file.get("providerConfigs")).toEqual({ openai: { apiKey: "", model: "m" }, "codex-cli": { model: "m" } })
    expect((mocks.file.get("searchApiConfig") as SearchApiConfig).providerConfigs?.searxng).toEqual({ searXngUrl: "http://localhost:8080" })
    expect(Object.fromEntries(mocks.keychain)).toEqual({
      llm: "clave-principal",
      "provider:openai": "clave-principal",
      search: "clave-busqueda",
      "search:tavily": "clave-busqueda",
    })

    // The next time the app opens, they come back from the keychain.
    __secretsTest.reset(null)
    expect(await loadSettingsKeys()).toBe(true)
    expect(await loadLlmConfig()).toEqual(llm)
    expect((await loadProviderConfigs())?.openai?.apiKey).toBe("clave-principal")
    expect((await loadProviderConfigs())?.["codex-cli"]).toEqual({ model: "m" })
    expect(await loadSearchApiConfig()).toEqual(search)
  })

  it("stay in the file when the keychain cannot be used, as before", async () => {
    mocks.state.broken = true
    expect(await loadSettingsKeys()).toBe(false)
    await saveLlmConfig(llm)
    await saveSearchApiConfig(search)
    expect(mocks.file.get("llmConfig")).toEqual(llm)
    expect(mocks.file.get("searchApiConfig")).toEqual(search)
    expect(await loadLlmConfig()).toEqual(llm)
  })

  it("stay in the file when the keychain does not keep them", async () => {
    expect(await loadSettingsKeys()).toBe(true)
    mocks.state.broken = true
    await saveLlmConfig(llm)
    expect(mocks.file.get("llmConfig")).toEqual(llm)
  })

  it("are written only when they change, and a cleared one leaves the keychain", async () => {
    mocks.keychain.set("llm", "clave-principal")
    mocks.file.set("llmConfig", { ...llm, apiKey: "" })
    expect(await loadSettingsKeys()).toBe(true)
    mocks.invoke.mockClear()
    await saveLlmConfig(llm)
    expect(mocks.invoke).not.toHaveBeenCalled()
    expect((mocks.file.get("llmConfig") as LlmConfig).apiKey).toBe("")
    await saveLlmConfig({ ...llm, apiKey: "" })
    expect(mocks.keychain.has("llm")).toBe(false)
    const free = await keepKeys({ llm: "" })
    expect([...free]).toEqual(["llm"])
  })
})
