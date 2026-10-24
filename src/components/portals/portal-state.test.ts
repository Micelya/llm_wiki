import { describe, expect, it } from "vitest"
import { downloadingAlone, progressFigures, showsPortal } from "./portal-state"
import { statusOf } from "./portal.fixtures"

describe("the portal's window while the app downloads", () => {
  it("is hidden while the app works alone, and shows when the person asks", () => {
    const status = statusOf()
    expect(downloadingAlone(status)).toBe(true)
    expect(showsPortal(status, false)).toBe(false)
    expect(showsPortal(status, true)).toBe(true)
  })

  it("shows while the app fills the portal's form, before it reaches the record", () => {
    expect(downloadingAlone(statusOf({}, { page: null, message: "Completando el formulario del portal…" }))).toBe(false)
    expect(showsPortal(statusOf({}, { page: null }), false)).toBe(true)
  })

  it("shows again by itself when the portal needs the person or processes a query", () => {
    expect(showsPortal(statusOf({}, { state: "needs-person", person: "challenge" }), false)).toBe(true)
    expect(showsPortal(statusOf({}, { state: "needs-person", person: "login" }), false)).toBe(true)
    // Its own «Cancelar» must be at hand.
    expect(showsPortal(statusOf({}, { busy: true }), false)).toBe(true)
  })

  it("shows when nothing runs, when the person took it, and before anything is known", () => {
    expect(showsPortal(statusOf({ running: null }, { state: "done" }), false)).toBe(true)
    expect(showsPortal(statusOf({ running: null, manual: true }, { state: "" }), false)).toBe(true)
    expect(showsPortal(null, false)).toBe(true)
  })
})

describe("the figures of the walk in course", () => {
  it("say the page, the entries, the documents and what is left of the page", () => {
    expect(progressFigures(statusOf())).toEqual([
      { label: "Página", value: 2 },
      { label: "Actuaciones vistas", value: 20 },
      { label: "Con documento", value: 12 },
      { label: "Bajados ahora", value: 9 },
      { label: "Ya estaban", value: 1 },
      { label: "Faltan de esta página", value: 2 },
    ])
  })

  it("add what failed only when something did, and no page before the record shows", () => {
    const figures = progressFigures(statusOf({}, { page: null, failed: 3 }))
    expect(figures[0].label).toBe("Actuaciones vistas")
    expect(figures[figures.length - 1]).toEqual({ label: "No se pudieron bajar", value: 3 })
  })
})
