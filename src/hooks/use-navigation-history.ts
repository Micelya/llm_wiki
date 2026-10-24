import { useEffect } from "react"
import { resolvePlace } from "@/lib/navigation-places"
import { setPlaceResolver, startNavigationHistory, useNavigationHistoryStore } from "@/stores/navigation-history-store"

/** Whether the person is writing: there Alt+← and Alt+→ are left to the field. */
function isWriting(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false
  const field = target.closest("input, textarea, [contenteditable='true'], [role='textbox']")
  if (!field) return false
  if (field.tagName.toLowerCase() === "input") {
    return !["checkbox", "radio", "submit", "button", "reset"].includes((field as HTMLInputElement).type)
  }
  return true
}

/** The mouse's side buttons, as the browser numbers them. */
const MOUSE_BACK = 3
const MOUSE_FORWARD = 4

/**
 * Records the way the person goes through the app and lets her go back and
 * forward on it: Alt+← and Alt+→, and the side buttons of the mouse. Once,
 * in the bar at the top of every screen.
 */
export function useNavigationHistory() {
  useEffect(() => {
    setPlaceResolver(resolvePlace)
    const stop = startNavigationHistory()
    const history = () => useNavigationHistoryStore.getState()

    const onKey = (event: KeyboardEvent) => {
      if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return
      if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return
      if (isWriting(event.target)) return
      event.preventDefault()
      void (event.key === "ArrowLeft" ? history().back() : history().forward())
    }
    // The webview would take the side buttons as its own back and forward.
    const onMouseDown = (event: MouseEvent) => {
      if (event.button === MOUSE_BACK || event.button === MOUSE_FORWARD) event.preventDefault()
    }
    const onMouseUp = (event: MouseEvent) => {
      if (event.button !== MOUSE_BACK && event.button !== MOUSE_FORWARD) return
      event.preventDefault()
      void (event.button === MOUSE_BACK ? history().back() : history().forward())
    }

    window.addEventListener("keydown", onKey)
    window.addEventListener("mousedown", onMouseDown)
    window.addEventListener("mouseup", onMouseUp)
    return () => {
      window.removeEventListener("keydown", onKey)
      window.removeEventListener("mousedown", onMouseDown)
      window.removeEventListener("mouseup", onMouseUp)
      stop()
      setPlaceResolver(null)
    }
  }, [])
}
