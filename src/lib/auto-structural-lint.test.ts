import { beforeEach, describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))

import { useLintStore } from "@/stores/lint-store"
import { replaceStructuralItems } from "./auto-structural-lint"

describe("the structural link review run by itself", () => {
  beforeEach(() => useLintStore.getState().clearItems())

  it("replaces the previous structural results and keeps what the model found", () => {
    const store = useLintStore.getState()
    store.addItems([
      { type: "broken-link", severity: "warning", page: "a.md", detail: "", brokenTarget: "X" },
      { type: "semantic", severity: "info", page: "b.md", detail: "Contradicción" },
    ])
    replaceStructuralItems([{ type: "orphan", severity: "info", page: "c.md", detail: "" }])
    const items = useLintStore.getState().items
    expect(items.map((item) => [item.type, item.page])).toEqual([
      ["semantic", "b.md"],
      ["orphan", "c.md"],
    ])
  })
})
