import { beforeEach, describe, expect, it } from "vitest"
import type { AttentionNotice } from "@/lib/attention/types"
import { useAttentionStore } from "./attention-store"

function notice(id: string, count = 1): AttentionNotice {
  return {
    id,
    source: "s",
    kind: "problem",
    topic: id,
    priority: "normal",
    count,
    title: `${count} ${id}`,
    explanation: "",
    todo: "",
    resolvedWhen: "",
    actions: [{ kind: "run", label: "Reintentar", run: () => {} }],
  }
}

describe("what the sources publish", () => {
  beforeEach(() => useAttentionStore.setState({ bySource: {} }))

  it("replaces a source's list: a notice no longer published is gone", () => {
    const { publish } = useAttentionStore.getState()
    publish("s", [notice("a"), notice("b")])
    publish("s", [notice("b")])
    expect(useAttentionStore.getState().bySource.s.map((n) => n.id)).toEqual(["b"])
    publish("s", [])
    expect(useAttentionStore.getState().bySource.s).toEqual([])
  })

  it("keeps the same list when nothing but the functions changed, so polling does not redraw", () => {
    const { publish } = useAttentionStore.getState()
    publish("s", [notice("a")])
    const before = useAttentionStore.getState().bySource
    publish("s", [notice("a")])
    expect(useAttentionStore.getState().bySource).toBe(before)
    publish("s", [notice("a", 2)])
    expect(useAttentionStore.getState().bySource).not.toBe(before)
  })

  it("forgets a source that stops", () => {
    useAttentionStore.getState().publish("s", [notice("a")])
    useAttentionStore.getState().clear("s")
    expect(useAttentionStore.getState().bySource).toEqual({})
  })
})
