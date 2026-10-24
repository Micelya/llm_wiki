import { describe, expect, it } from "vitest"
import { statusOf } from "@/components/portals/portal.fixtures"
import { portalLine } from "@/components/portals/portal-line"
import { portalNotices, walkFigures, PORTALS_SOURCE } from "./portals"

describe("what the portals raise in Atención", () => {
  it("nothing before anything is known, nor when the last end was seen", () => {
    expect(portalNotices(null, false)).toEqual([])
    expect(portalNotices(statusOf({ running: null }, { state: "done" }), false)).toEqual([])
  })

  it("a walk in course, with its figures, a way to stop it and to go to «Portales»", () => {
    const [notice] = portalNotices(statusOf(), false)
    expect(notice.id).toBe(`${PORTALS_SOURCE}:walk`)
    expect(notice.kind).toBe("ongoing")
    expect(notice.title).toBe("Trayendo el expediente ZZZ 4321/2011 del portal")
    expect(notice.explanation).toContain("página 2 · 20 actuaciones vistas · 9 documentos bajados · 1 ya estaban")
    expect(notice.progress).toEqual({ done: 10, total: 12 })
    expect(notice.actions.map((a) => a.label)).toEqual(["Ir a Portales", "Detener"])
    expect(notice.actions[0]).toMatchObject({ kind: "go", target: { view: "portals" } })
  })

  it("a problem that comes first when the portal waits for the person", () => {
    const [challenge] = portalNotices(statusOf({}, { state: "needs-person", person: "challenge" }), false)
    expect([challenge.kind, challenge.priority]).toEqual(["problem", "high"])
    expect(challenge.todo).toContain("resolvé el desafío")
    const [login] = portalNotices(statusOf({}, { state: "needs-person", person: "login" }), false)
    expect(login.todo).toContain("iniciá sesión")
  })

  it("how the last walk ended, while it was not seen", () => {
    const done = portalNotices(statusOf({ running: null }, { state: "done", message: "ZZZ 4321/2011: 3 documentos nuevos.", warnings: ["La página 4 mostró 9 actuaciones y no 15 como las demás"] }), true)
    expect(done).toHaveLength(1)
    expect([done[0].kind, done[0].priority, done[0].title]).toEqual(["ongoing", "low", "Terminó de traer el expediente ZZZ 4321/2011"])
    expect(done[0].explanation).toContain("La página 4 mostró 9 actuaciones")
    const [failed] = portalNotices(statusOf({ running: null }, { state: "error", message: "El portal no responde" }), true)
    expect([failed.kind, failed.title]).toEqual(["problem", "No se pudo terminar de traer el expediente ZZZ 4321/2011"])
    const [stopped] = portalNotices(statusOf({ running: null }, { state: "stopped" }), true)
    expect(stopped.title).toBe("Se detuvo el recorrido del expediente ZZZ 4321/2011")
  })

  it("the figures in words leave out what is zero", () => {
    expect(walkFigures(statusOf({}, { page: null, already: 0, downloaded: 1, seen: 1 }))).toBe("1 actuación vista · 1 documento bajado")
  })
})

describe("the line of the portals at the foot of the window", () => {
  it("says the walk in course and how far it went", () => {
    expect(portalLine(statusOf(), false)).toEqual({
      tone: "busy",
      text: "Portales: trayendo ZZZ 4321/2011 · página 2 · 20 actuaciones vistas · 9 documentos bajados · 1 ya estaban",
    })
  })

  it("says that the portal waits for the person", () => {
    expect(portalLine(statusOf({}, { state: "needs-person", person: "challenge" }), false)).toEqual({
      tone: "person",
      text: "Portales: el portal te necesita para resolver el desafío y seguir con ZZZ 4321/2011",
    })
  })

  it("says how the last walk ended until it is seen, and nothing after", () => {
    const done = statusOf({ running: null }, { state: "done", message: "ZZZ 4321/2011: 3 documentos nuevos, 9 ya estaban." })
    expect(portalLine(done, true)).toEqual({ tone: "ok", text: "Portales: terminó. ZZZ 4321/2011: 3 documentos nuevos, 9 ya estaban." })
    expect(portalLine(done, false)).toBeNull()
    expect(portalLine(statusOf({ running: null }, { state: "error", message: "El portal no responde" }), true)?.tone).toBe("error")
    expect(portalLine(statusOf({ running: null }, { state: "stopped" }), true)?.tone).toBe("stopped")
    expect(portalLine(null, true)).toBeNull()
  })
})
