/**
 * The connection with Micelya Core seen from the screens (Ajustes →
 * Conexiones → Micelya Core): what is missing in the data on screen, and
 * what «Probar la conexión» found, in plain words. Everything here is pure.
 */
import type { CoreConnectionFields, CoreConnectionTest } from "@/commands/micelya-core"

export const EMPTY_CORE_FIELDS: CoreConnectionFields = { api: "", issuer: "", tenant: "", clientId: "" }

/** The host of an address, or the address as it is. */
function host(address: string): string {
  try {
    return new URL(address.trim()).host || address.trim()
  } catch {
    return address.trim()
  }
}

/** An address of Core: https, or http only on this computer. */
function addressProblem(value: string, what: string): string | null {
  const text = value.trim()
  if (!text) return `Falta ${what}.`
  let url: URL
  try {
    url = new URL(text)
  } catch {
    return `${what[0].toUpperCase()}${what.slice(1)} no es una dirección web.`
  }
  const local = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)
  if (url.protocol === "https:" || (url.protocol === "http:" && local)) return null
  return `${what[0].toUpperCase()}${what.slice(1)} tiene que empezar con https://.`
}

/** What is missing or wrong in the data on screen, in words; empty when they can be saved or tested. */
export function coreConnectionProblems(fields: CoreConnectionFields, secret: string, hasSecret: boolean): string[] {
  const problems = [
    addressProblem(fields.api, "la dirección de la API"),
    addressProblem(fields.issuer, "el emisor"),
  ].filter((problem): problem is string => !!problem)
  for (const [value, what] of [
    [fields.tenant, "la organización"],
    [fields.clientId, "el identificador del cliente"],
  ] as const) {
    if (!value.trim()) problems.push(`Falta ${what}.`)
    else if (/\s/.test(value.trim())) problems.push(`${what[0].toUpperCase()}${what.slice(1)} no puede tener espacios.`)
  }
  if (!hasSecret && !secret.trim()) problems.push("Falta el secreto del cliente.")
  return problems
}

/** Whether the data on screen differ from the ones kept. */
export function coreFieldsChanged(fields: CoreConnectionFields, kept: CoreConnectionFields | null, secret: string): boolean {
  if (secret.trim()) return true
  const base = kept ?? EMPTY_CORE_FIELDS
  return (Object.keys(EMPTY_CORE_FIELDS) as (keyof CoreConnectionFields)[]).some((key) => fields[key].trim() !== base[key].trim())
}

/** What «Probar la conexión» found, in plain words. */
export function coreTestText(test: CoreConnectionTest, fields: CoreConnectionFields): string {
  if (test.ok) {
    const kept =
      test.readings === null
        ? ""
        : test.readings === 0
          ? " Core no guarda ninguna lectura de esta credencial."
          : ` Core guarda ${test.moreReadings ? "más de " : ""}${test.readings} ${test.readings === 1 ? "lectura" : "lecturas"} de esta credencial.`
    return `Anduvo: Micelya Core dio el permiso de acceso y la lista de lecturas.${kept}`
  }
  const issuer = host(fields.issuer)
  const api = host(fields.api)
  if (test.step === "fields") return "Faltan datos o alguno no tiene la forma esperada: revisalos."
  if (test.step === "permit") {
    switch (test.problem) {
      case "unreachable":
        return `No se pudo llegar al emisor (${issuer}): revisá su dirección y la conexión a internet.`
      case "timeout":
        return `El emisor (${issuer}) no respondió a tiempo: probá de nuevo en un rato.`
      case "rejected":
        return "El emisor rechazó el identificador del cliente o el secreto: revisalos."
      case "organization":
        return "El emisor no reconoce la organización para este cliente: revisá la organización."
      case "not-found":
        return "En la dirección del emisor no hay un emisor de permisos: revisala (suele terminar en /realms/ y un nombre)."
      case "unavailable":
        return `El emisor (${issuer}) no está disponible ahora: probá de nuevo en un rato.`
    }
  } else {
    switch (test.problem) {
      case "unreachable":
        return `No se pudo llegar a la API de Core (${api}): revisá su dirección y la conexión a internet.`
      case "timeout":
        return `La API de Core (${api}) no respondió a tiempo: probá de nuevo en un rato.`
      case "rejected":
        return "La API de Core no aceptó el permiso de acceso: revisá que el emisor sea el del mismo ambiente que la API."
      case "forbidden":
        return "La credencial no tiene permiso para leer documentos en esta organización: revisá la organización o pedí el permiso a Core."
      case "organization":
        return "La organización todavía no está lista en Core: pedile a Core que la habilite."
      case "not-found":
        return "En esa dirección no está la API de Core: revisala (suele terminar en /v1)."
      case "unavailable":
        return "La API de Core no está disponible ahora: probá de nuevo en un rato."
    }
  }
  const where = test.step === "permit" ? "El emisor" : "La API de Core"
  return `${where} respondió algo inesperado${test.status ? ` (${test.status})` : ""}.`
}
