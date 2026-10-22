import { describe, expect, it, vi } from "vitest"
import { stopListening } from "./use-docbase-query"

vi.mock("@/commands/docbase", () => ({}))

const moment = (ms = 20) => new Promise((resolve) => setTimeout(resolve, ms))

/** A subscription that is in place only when the test says so. */
function pending<T>(): { promise: Promise<T>; resolve: (value: T) => void; reject: (cause: unknown) => void } {
  let resolve!: (value: T) => void
  let reject!: (cause: unknown) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

describe("stopping to listen to the base", () => {
  it("waits for the subscription before stopping it", async () => {
    const stop = vi.fn(async () => {})
    const subscription = pending<() => Promise<void>>()
    stopListening(subscription.promise, 5, 1)
    await moment()
    expect(stop).not.toHaveBeenCalled()
    subscription.resolve(stop)
    await moment()
    expect(stop).toHaveBeenCalledTimes(1)
  })

  it("tries again when the page has not registered the listener yet", async () => {
    let registered = false
    const stop = vi.fn(async () => {
      if (!registered) throw new TypeError("Cannot read properties of undefined (reading 'handlerId')")
    })
    stopListening(Promise.resolve(stop), 5, 5)
    await moment(2)
    registered = true
    await moment(40)
    expect(stop.mock.calls.length).toBeGreaterThanOrEqual(2)
    const results = stop.mock.results
    await expect(results[results.length - 1].value).resolves.toBeUndefined()
  })

  it("gives up after a few tries without leaving an error behind", async () => {
    const stop = vi.fn(() => {
      throw new Error("no se puede")
    })
    stopListening(Promise.resolve(stop), 3, 1)
    await moment(40)
    expect(stop).toHaveBeenCalledTimes(3)
  })

  it("does nothing when it never got to listen", async () => {
    const subscription = pending<() => void>()
    stopListening(subscription.promise, 5, 1)
    subscription.reject(new Error("sin app"))
    await moment()
  })
})
