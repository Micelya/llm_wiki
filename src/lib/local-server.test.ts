import { describe, expect, it } from "vitest"
import { apiExample, claudeCodeCommand, codexConfig, lastUseWords, permissionWords, SERVER_STATE_WORDS } from "./local-server"

const MCP = "http://127.0.0.1:19827/mcp"

describe("what Ajustes → Programas conectados gives to copy", () => {
  it("the command for Claude Code, with the token or the place where it goes", () => {
    expect(claudeCodeCommand(MCP)).toBe(`claude mcp add --transport http micelya ${MCP} --header "Authorization: Bearer EL-TOKEN"`)
    expect(claudeCodeCommand(MCP, "mcy_inventado")).toContain('--header "Authorization: Bearer mcy_inventado"')
  })

  it("the configuration of Codex keeps the token in an environment variable, never in the file", () => {
    const config = codexConfig(MCP)
    expect(config).toContain("[mcp_servers.micelya]")
    expect(config).toContain(`url = "${MCP}"`)
    expect(config).toContain('bearer_token_env_var = "MICELYA_TOKEN"')
    expect(config).not.toContain("mcy_")
  })

  it("a first call to the local API from Windows", () => {
    expect(apiExample("http://127.0.0.1:19827", "mcy_inventado")).toBe(
      'curl.exe -H "Authorization: Bearer mcy_inventado" http://127.0.0.1:19827/api/v1/operaciones',
    )
  })

  it("each permission and the state of the server in words, with the vertical's word", () => {
    const words = permissionWords("caso")
    expect(words.read.label).toBe("Solo leer")
    expect(words.change.label).toBe("Leer y pedir cambios")
    expect(words.change.hint).toContain("sumar documentos a un caso")
    expect(words.change.hint).toContain("espera en Atención")
    expect(permissionWords("asunto").change.hint).toContain("a un asunto")
    expect(SERVER_STATE_WORDS["port-in-use"]).toContain("puerto 19827")
  })

  it("when a program was last used", () => {
    expect(lastUseWords(null, 0, () => "")).toBe("Todavía no se usó")
    expect(lastUseWords(1, 1, () => "ayer")).toBe("Último uso: ayer · 1 operación en los últimos 30 días")
    expect(lastUseWords(1, 12, () => "hace 4 min")).toBe("Último uso: hace 4 min · 12 operaciones en los últimos 30 días")
  })
})
