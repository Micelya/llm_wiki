import { describe, expect, it } from "vitest"
import { optionLabel, resolvedLabel } from "./review-labels"

describe("the review of the wiki in Spanish", () => {
  it("shows the model's fixed buttons in Spanish and leaves any other label as it came", () => {
    expect(optionLabel("Create Page")).toBe("Crear página")
    expect(optionLabel("Skip")).toBe("Omitir")
    expect(optionLabel("Crear la página del contrato")).toBe("Crear la página del contrato")
  })

  it("says what was done with a proposal", () => {
    expect(resolvedLabel("Saved to Wiki")).toBe("Guardada en el wiki")
    expect(resolvedLabel("Bulk resolved")).toBe("Marcada como resuelta")
    expect(resolvedLabel("open:entities/x.md")).toBe("Abierta")
    expect(resolvedLabel("Skip")).toBe("Omitida")
    expect(resolvedLabel(undefined)).toBe("Resuelta")
  })
})
