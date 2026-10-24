import { beforeEach, describe, expect, it } from "vitest"
import { statusOf } from "@/components/portals/portal.fixtures"
import { usePortalStore } from "./portal-store"

const running = statusOf()
const done = statusOf({ running: null }, { state: "done", message: "ZZZ 4321/2011: 3 documentos nuevos, 9 ya estaban." })

describe("how the end of a walk is told", () => {
  beforeEach(() => usePortalStore.setState({ status: null, endUnseen: false, open: false }))

  it("stays to be seen when it ends while «Portales» is not on screen, until she opens it", () => {
    const { publish, setOpen } = usePortalStore.getState()
    publish(running)
    expect(usePortalStore.getState().endUnseen).toBe(false)
    publish(done)
    expect(usePortalStore.getState().endUnseen).toBe(true)
    // The same status again does not tell it twice nor forget it.
    publish(done)
    expect(usePortalStore.getState().endUnseen).toBe(true)
    setOpen(true)
    expect(usePortalStore.getState().endUnseen).toBe(false)
    setOpen(false)
    expect(usePortalStore.getState().endUnseen).toBe(false)
  })

  it("is seen at once when it ends with «Portales» on screen", () => {
    const { publish, setOpen } = usePortalStore.getState()
    setOpen(true)
    publish(running)
    publish(done)
    expect(usePortalStore.getState().endUnseen).toBe(false)
  })

  it("is forgotten when another walk starts, and a stop or an error is told too", () => {
    const { publish } = usePortalStore.getState()
    publish(running)
    publish(statusOf({ running: null }, { state: "error", message: "El portal no responde" }))
    expect(usePortalStore.getState().endUnseen).toBe(true)
    publish(running)
    expect(usePortalStore.getState().endUnseen).toBe(false)
    publish(statusOf({ running: null }, { state: "stopped" }))
    expect(usePortalStore.getState().endUnseen).toBe(true)
  })

  it("does not tell an end it did not see happen (the app just opened)", () => {
    usePortalStore.getState().publish(done)
    expect(usePortalStore.getState().endUnseen).toBe(false)
  })
})
