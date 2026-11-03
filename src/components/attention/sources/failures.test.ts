import { describe, expect, it } from "vitest"
import type { FailureView } from "@/commands/docbase"
import { FAILURES_SOURCE, SUPPORT, failureNotices, whatTheAppDid } from "./failures"

// Every name is made up (R-24).
function failure(id: number, patch: Partial<FailureView>): FailureView {
  return {
    id,
    stage: "shapes",
    itemKind: "file",
    item: `h${id}`,
    cause: "file-locked",
    message: "Failed to open PDF (os error 32)",
    class: "transient",
    scope: "item",
    engine: null,
    attempt: 1,
    firstAt: 1_000,
    lastAt: 1_000,
    nextAt: 1_000 + 600_000,
    state: "open",
    indicator: false,
    toDiagnose: false,
    path: `carpeta/inventado-${id}.pdf`,
    locationId: id,
    ...patch,
  }
}

describe("failure notices", () => {
  it("say nothing without failures", () => {
    expect(failureNotices(null)).toEqual([])
    expect(failureNotices([])).toEqual([])
  })

  it("group by cause and say the cause, what the app did, what to do and whom to turn to", () => {
    const notices = failureNotices([
      failure(1, {}),
      failure(2, { attempt: 4, indicator: true }),
      failure(3, { cause: "pdf-encrypted", class: "permanent", nextAt: null, state: "person", indicator: true, toDiagnose: true }),
    ])
    expect(notices.map((n) => [n.id, n.count, n.priority])).toEqual([
      [`${FAILURES_SOURCE}:file-locked`, 2, "normal"],
      [`${FAILURES_SOURCE}:pdf-encrypted`, 1, "high"],
    ])
    const [locked, encrypted] = notices
    expect(locked.title).toContain("abiertos en otro programa")
    expect(locked.explanation).toContain("vuelve a intentar sola")
    expect(locked.todo).toContain("Cerralo en ese programa")
    expect(locked.todo).toContain(SUPPORT)
    expect(locked.cases?.[1].detail).toContain("Van 4 intentos")
    expect(encrypted.explanation).toContain("no lo vuelve a intentar")
    expect(encrypted.cases?.[0].detail).toContain("No se reintenta")
    // Sending to the support comes later: the button is there, disabled.
    const send = encrypted.actions[0]
    expect(send.kind === "run" && send.disabled).toBeTruthy()
  })

  it("tell when a transient one is tried again, and that the others are not", () => {
    expect(whatTheAppDid(failure(1, {}))).toMatch(/^Se va a reintentar el /)
    expect(whatTheAppDid(failure(1, { class: "unknown", nextAt: null }))).toBe("No se reintenta.")
  })
})
