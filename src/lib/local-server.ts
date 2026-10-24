import type { LocalServerState, ProgramPermission } from "@/commands/local-server"

/**
 * What Ajustes → «Programas conectados» says and gives to copy (D-495, the
 * contract's design 6.6): how the server is, each permission in words, and
 * the configuration to connect an assistant by MCP or a script by the
 * local API. Without a token, `EL-TOKEN` stands in its place.
 */

export const TOKEN_PLACEHOLDER = "EL-TOKEN"
/** The environment variable the examples keep the token in. */
export const TOKEN_VARIABLE = "MICELYA_TOKEN"

export const SERVER_STATE_WORDS: Record<LocalServerState, string> = {
  starting: "Arrancando…",
  running: "Prendido, solo en esta computadora",
  "port-in-use": "No pudo arrancar: otro programa usa el puerto 19827. Cerralo y volvé a abrir Micelya.",
  stopped: "Se detuvo. Volvé a abrir Micelya.",
}

/** Each permission in words; `matter` is the vertical's word for a matter («caso»). */
export function permissionWords(matter: string): Record<ProgramPermission, { label: string; hint: string }> {
  return {
    read: { label: "Solo leer", hint: "Busca y lee en la base; no puede pedir cambios." },
    change: {
      label: "Leer y pedir cambios",
      hint: `Además puede pedir cambios (por ejemplo, sumar documentos a un ${matter}): cada pedido espera en Atención a que una persona lo confirme.`,
    },
  }
}

/** The command to connect Claude Code, for a terminal. */
export function claudeCodeCommand(mcpAddress: string, token?: string | null): string {
  return `claude mcp add --transport http micelya ${mcpAddress} --header "Authorization: Bearer ${token || TOKEN_PLACEHOLDER}"`
}

/** What goes in Codex's configuration file (`config.toml`), with the token in an environment variable. */
export function codexConfig(mcpAddress: string): string {
  return `[mcp_servers.micelya]\nurl = "${mcpAddress}"\nbearer_token_env_var = "${TOKEN_VARIABLE}"`
}

/** A first call to the local API, from PowerShell or a terminal of Windows. */
export function apiExample(address: string, token?: string | null): string {
  return `curl.exe -H "Authorization: Bearer ${token || TOKEN_PLACEHOLDER}" ${address}/api/v1/operaciones`
}

/** When a program was last used, in words. */
export function lastUseWords(lastUsedAt: number | null, uses: number, format: (ms: number) => string): string {
  if (lastUsedAt === null) return "Todavía no se usó"
  const month = uses === 1 ? "1 operación en los últimos 30 días" : `${uses} operaciones en los últimos 30 días`
  return `Último uso: ${format(lastUsedAt)} · ${month}`
}
