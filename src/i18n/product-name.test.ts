import { describe, expect, it } from "vitest"
import i18n from "./index"
import { withProductName } from "./product-name"

describe("product name in translated texts", () => {
  it("replaces every mention of the original app", () => {
    expect(withProductName("Exit LLM Wiki when LLM Wiki closes")).toBe(
      "Exit Micelya Desktop when Micelya Desktop closes",
    )
  })

  it("leaves other texts alone", () => {
    expect(withProductName("An LLM-driven wiki")).toBe("An LLM-driven wiki")
  })

  it("applies to texts from the language files", () => {
    expect(i18n.t("settings.sections.general.autostartHint")).toBe(
      "Starts Micelya Desktop automatically after you sign in to this computer.",
    )
  })

  it("applies to fallback texts written in components", () => {
    expect(i18n.t("no.such.key", { defaultValue: "Inside the current LLM Wiki project" })).toBe(
      "Inside the current Micelya Desktop project",
    )
  })
})
