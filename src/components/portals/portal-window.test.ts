import { describe, expect, it } from "vitest"
import { PortalWindow, type PortalRect, type PortalWindowCommands } from "./portal-window"

const area: PortalRect = { x: 400, y: 120, width: 800, height: 600 }
const moved: PortalRect = { x: 460, y: 120, width: 740, height: 600 }

/**
 * The app's side, as the commands see it: the first show creates the
 * window, every show places and shows it, a hide hides it. A command can
 * be held in flight until the test lets it go.
 */
function fakeApp() {
  const app = {
    created: 0,
    visible: false,
    rect: null as PortalRect | null,
    log: [] as string[],
    hold: false,
    failNext: null as string | null,
    held: [] as Array<() => void>,
  }
  const run = (name: string, apply: () => void) =>
    new Promise<void>((resolve, reject) => {
      app.log.push(name)
      const finish = () => {
        if (app.failNext) {
          const problem = app.failNext
          app.failNext = null
          reject(problem)
          return
        }
        apply()
        resolve()
      }
      if (app.hold) app.held.push(finish)
      else finish()
    })
  const commands: PortalWindowCommands = {
    show: (_portal, rect) =>
      run(`show ${rect.x},${rect.y},${rect.width},${rect.height}`, () => {
        if (app.created === 0) app.created = 1
        app.visible = true
        app.rect = rect
      }),
    hide: () => run("hide", () => (app.visible = false)),
  }
  /** Lets every held command go, and the ones they lead to. */
  const release = async () => {
    app.hold = false
    while (app.held.length) {
      app.held.shift()!()
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
  }
  return { app, commands, release }
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0))

describe("the portal's window as the page wants it", () => {
  it("is created once and stays shown when the page mounts twice at once (React in development)", async () => {
    const { app, commands, release } = fakeApp()
    const portal = new PortalWindow("pjn", commands)
    app.hold = true
    const first = portal.show(area) // first mount
    await tick()
    const cleanup = portal.hide() // React unmounts it…
    const second = portal.show(area) // …and mounts it again
    await release()
    await Promise.all([first, cleanup, second])
    expect(app.created).toBe(1)
    expect(app.visible).toBe(true)
    expect(app.log).toEqual(["show 400,120,800,600"])
  })

  it("is hidden when the page goes, so it does not cover other pages, and shown again when it comes back", async () => {
    const { app, commands } = fakeApp()
    const portal = new PortalWindow("pjn", commands)
    await portal.show(area)
    expect(app.visible).toBe(true)
    await portal.hide()
    expect(app.visible).toBe(false)
    await portal.show(area)
    expect(app.visible).toBe(true)
    expect(app.created).toBe(1)
    expect(app.log).toEqual(["show 400,120,800,600", "hide", "show 400,120,800,600"])
  })

  it("ends hidden when the page goes while the window is still opening", async () => {
    const { app, commands, release } = fakeApp()
    const portal = new PortalWindow("pjn", commands)
    app.hold = true
    const shown = portal.show(area)
    await tick()
    const hidden = portal.hide()
    await release()
    await Promise.all([shown, hidden])
    expect(app.visible).toBe(false)
    expect(app.log).toEqual(["show 400,120,800,600", "hide"])
  })

  it("follows the area when it moves, and sends nothing while it stays", async () => {
    const { app, commands } = fakeApp()
    const portal = new PortalWindow("pjn", commands)
    await portal.show(area)
    await portal.show(area)
    await portal.show(moved)
    expect(app.rect).toEqual(moved)
    expect(app.log).toEqual(["show 400,120,800,600", "show 460,120,740,600"])
  })

  it("says why it could not show the window, and tries again on the next wish", async () => {
    const { app, commands } = fakeApp()
    const portal = new PortalWindow("pjn", commands)
    app.failNext = "No se pudo abrir el portal"
    await expect(portal.show(area)).rejects.toBe("No se pudo abrir el portal")
    expect(app.visible).toBe(false)
    await portal.show(area)
    expect(app.visible).toBe(true)
    expect(app.log).toEqual(["show 400,120,800,600", "show 400,120,800,600"])
  })

  it("does not report a failure that a newer wish already left behind", async () => {
    const { app, commands, release } = fakeApp()
    const portal = new PortalWindow("pjn", commands)
    app.hold = true
    app.failNext = "No se pudo abrir el portal"
    const first = portal.show(area)
    await tick()
    const second = portal.show(moved)
    await release()
    await expect(first).resolves.toBeUndefined()
    await expect(second).resolves.toBeUndefined()
    expect(app.rect).toEqual(moved)
    expect(app.visible).toBe(true)
  })
})
