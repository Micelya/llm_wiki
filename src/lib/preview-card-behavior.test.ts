import { describe, expect, it } from "vitest"
import { CLOSED, linkPress, notePress, pressPointer, previewChange, type PreviewState } from "./preview-card-behavior"

const HOVERED: PreviewState = { open: true, pinned: false }
const PINNED: PreviewState = { open: true, pinned: true }

describe("the floating cards of an article (P-246)", () => {
  it("open when the mouse rests or the keyboard arrives, and close when they leave", () => {
    expect(previewChange(CLOSED, true, "trigger-hover", true)).toEqual(HOVERED)
    expect(previewChange(CLOSED, true, "trigger-focus", true)).toEqual(HOVERED)
    expect(previewChange(HOVERED, false, "trigger-hover", true)).toEqual(CLOSED)
    expect(previewChange(HOVERED, false, "trigger-focus", true)).toEqual(CLOSED)
  })

  it("a card shown by a press stays when the mouse or the focus leaves; Escape and a press elsewhere close it", () => {
    for (const passing of ["trigger-hover", "trigger-focus", "focus-out"]) {
      expect(previewChange(PINNED, false, passing, true)).toBe(PINNED)
    }
    expect(previewChange(PINNED, false, "escape-key", true)).toEqual(CLOSED)
    expect(previewChange(PINNED, false, "outside-press", true)).toEqual(CLOSED)
    expect(previewChange(HOVERED, false, "escape-key", true)).toEqual(CLOSED)
  })

  it("turned off in Settings, the mouse and the focus open nothing", () => {
    expect(previewChange(CLOSED, true, "trigger-hover", false)).toBe(CLOSED)
    expect(previewChange(CLOSED, true, "trigger-focus", false)).toBe(CLOSED)
    // What a press opened, Escape still closes.
    expect(previewChange(PINNED, false, "escape-key", false)).toEqual(CLOSED)
  })

  it("a note shows its quote on a touch and keeps it; a second touch hides it", () => {
    const first = notePress(CLOSED, "touch")
    expect(first).toEqual({ state: PINNED, follow: false, focusCard: false })
    expect(notePress(first.state, "touch")).toEqual({ state: CLOSED, follow: false, focusCard: false })
    // A click on a card the mouse opened keeps it; Enter takes the keyboard into it.
    expect(notePress(HOVERED, "mouse").state).toEqual(PINNED)
    expect(notePress(HOVERED, "keyboard")).toEqual({ state: PINNED, follow: false, focusCard: true })
    expect(notePress(CLOSED, "keyboard").focusCard).toBe(true)
  })

  it("a link: the first touch shows its card, the second opens it; the mouse and the keyboard open it at once", () => {
    const first = linkPress(CLOSED, "touch", true)
    expect(first).toEqual({ state: PINNED, follow: false, focusCard: false })
    expect(linkPress(first.state, "touch", true).follow).toBe(true)
    expect(linkPress(CLOSED, "pen", true).follow).toBe(false)
    expect(linkPress(CLOSED, "mouse", true)).toEqual({ state: CLOSED, follow: true, focusCard: false })
    expect(linkPress(HOVERED, "keyboard", true).follow).toBe(true)
    // Turned off, a touch opens the link as before.
    expect(linkPress(CLOSED, "touch", false).follow).toBe(true)
  })

  it("tells the keyboard from a pointer by the click", () => {
    expect(pressPointer(0, "mouse")).toBe("keyboard")
    expect(pressPointer(1, "touch")).toBe("touch")
    expect(pressPointer(1, "pen")).toBe("pen")
    expect(pressPointer(1, null)).toBe("mouse")
  })
})
