/**
 * How the floating previews of an article behave (P-246, D-454): the card
 * of a numbered note (its quote and its document) and the card of a link
 * to a person, organization, document or matter (its short summary and
 * its type).
 *
 * - The mouse resting on the mark or the link opens the card after a
 *   moment; leaving it closes the card after another, so the mouse can
 *   reach the card on the way (base-ui's preview card keeps that safe
 *   zone).
 * - Focus from the keyboard opens it the same way; Escape closes it.
 * - A press is what touch has. On a note it shows the card and keeps it
 *   (pinned) until a second press, Escape or a press elsewhere: what the
 *   note did before the previews. On a link, the first touch shows the
 *   card and the second follows the link; the mouse and the keyboard
 *   follow it at once.
 * - Turned off in Settings, nothing opens by itself: a note still shows
 *   its card when pressed, and a link is a plain link.
 *
 * Kept apart from the components so it can be tested without a screen.
 */

/** The moment the mouse or the focus rests before a card opens. */
export const OPEN_DELAY_MS = 500
/** The moment a card waits, once the mouse leaves, before it closes. */
export const CLOSE_DELAY_MS = 300

export interface PreviewState {
  open: boolean
  /** Opened by a press: it stays when the mouse or the focus moves away. */
  pinned: boolean
}

export const CLOSED: PreviewState = { open: false, pinned: false }

/** What pressed: a pointer, as the browser names it, or the keyboard. */
export type PressPointer = "mouse" | "touch" | "pen" | "keyboard"

/** The changes that come from passing over or through, not from a decision. */
const PASSING = new Set(["trigger-hover", "trigger-focus", "focus-out"])

/**
 * A change base-ui's preview card asks for, by its reason (`trigger-hover`,
 * `trigger-focus`, `escape-key`, `outside-press`…). A pinned card ignores
 * the mouse or the focus leaving; with the previews off, the mouse and
 * the focus open nothing.
 */
export function previewChange(state: PreviewState, open: boolean, reason: string, enabled: boolean): PreviewState {
  if (open) {
    if (!enabled && PASSING.has(reason)) return state
    return state.open ? state : { open: true, pinned: state.pinned }
  }
  if (state.pinned && PASSING.has(reason)) return state
  return state.open ? CLOSED : state
}

export interface PressOutcome {
  state: PreviewState
  /** Follow the link (open what it names). */
  follow: boolean
  /** Move the focus into the card, so the keyboard reaches its button. */
  focusCard: boolean
}

/** A press on the number of a note: shows its card and keeps it; again, hides it. */
export function notePress(state: PreviewState, pointer: PressPointer): PressOutcome {
  if (state.open && state.pinned) return { state: CLOSED, follow: false, focusCard: false }
  return { state: { open: true, pinned: true }, follow: false, focusCard: pointer === "keyboard" }
}

/**
 * A press on a link: the first touch shows its card, the second follows
 * it; the mouse and the keyboard follow it at once.
 */
export function linkPress(state: PreviewState, pointer: PressPointer, enabled: boolean): PressOutcome {
  const touch = pointer === "touch" || pointer === "pen"
  if (enabled && touch && !state.open) return { state: { open: true, pinned: true }, follow: false, focusCard: false }
  return { state: CLOSED, follow: true, focusCard: false }
}

/** The pointer of a click: the keyboard when no pointer made it (`detail` 0). */
export function pressPointer(detail: number, lastPointer: string | null): PressPointer {
  if (detail === 0) return "keyboard"
  return lastPointer === "touch" || lastPointer === "pen" ? lastPointer : "mouse"
}
