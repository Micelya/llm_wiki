import { useEffect, useMemo, useRef, useState } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { PreviewCard, PreviewCardContent, PreviewCardTrigger } from "@/components/ui/preview-card"
import { firstWikilink } from "@/lib/article-preview"
import { CLOSED, notePress, pressPointer, previewChange, type PreviewState } from "@/lib/preview-card-behavior"
import { hideHtmlComments, transformWikilinks } from "@/lib/wikilink-transform"
import { openWikiPath } from "@/lib/open-wiki-path"
import { usePreviewCardsStore } from "@/stores/preview-cards-store"

const FOCUSABLE = "a[href], button:not([disabled])"

/**
 * The number of a note of an article (D-441), like the notes of an
 * encyclopedia. Its card shows the note (the quote and the document it is
 * from) and opens that document. The card opens when the mouse rests on
 * the number or the keyboard reaches it, and closes when they leave
 * (P-246, D-454); a press (a touch, a click, Enter) shows it and keeps it
 * until a second press, Escape or a press elsewhere. Enter also takes the
 * keyboard into the card, to its button; leaving the card with Tab goes
 * back to the number. The element carries its note's label (`data-note`).
 */
export function NoteMark({ label, note }: { label: string; note: string }) {
  const enabled = usePreviewCardsStore((s) => s.enabled)
  const [state, setState] = useState<PreviewState>(CLOSED)
  const shown = useMemo(() => transformWikilinks(hideHtmlComments(note)), [note])
  const source = useMemo(() => firstWikilink(note), [note])
  const lastPointer = useRef<string | null>(null)
  const trigger = useRef<HTMLElement | null>(null)
  const card = useRef<HTMLDivElement | null>(null)
  const focusCard = useRef(false)

  // Enter took the keyboard to the card: to its button once it is shown.
  useEffect(() => {
    if (!state.open || !focusCard.current) return
    let frame = 0
    let tries = 0
    const reach = () => {
      const target = card.current?.querySelector<HTMLElement>("[data-card-open]") ?? card.current
      if (target) {
        focusCard.current = false
        target.focus()
      } else if (tries++ < 10) {
        frame = requestAnimationFrame(reach)
      }
    }
    frame = requestAnimationFrame(reach)
    return () => cancelAnimationFrame(frame)
  }, [state])

  function close(returnFocus: boolean) {
    setState(CLOSED)
    if (returnFocus && card.current?.contains(document.activeElement)) trigger.current?.focus()
  }

  function open(target: string) {
    close(false)
    let slug = target
    try {
      slug = decodeURIComponent(slug)
    } catch {
      // As written.
    }
    void openWikiPath(slug)
  }

  function follow(event: React.MouseEvent<HTMLAnchorElement>, href: string) {
    if (!href.startsWith("#")) return
    event.preventDefault()
    open(href.slice(1))
  }

  function leaveWithTab(event: React.KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Tab" || !card.current) return
    const inside = Array.from(card.current.querySelectorAll<HTMLElement>(FOCUSABLE))
    const at = inside.indexOf(document.activeElement as HTMLElement)
    const out = event.shiftKey ? at <= 0 : at === inside.length - 1
    // Back on the number, the Tab itself moves on from there.
    if (out) close(true)
  }

  return (
    <PreviewCard
      open={state.open}
      onOpenChange={(next, details) => {
        if (!next && details.reason === "escape-key") {
          close(true)
          return
        }
        setState((current) => previewChange(current, next, details.reason, enabled))
      }}
    >
      <PreviewCardTrigger
        ref={(element: HTMLElement | null) => {
          trigger.current = element
        }}
        render={<button type="button" />}
        data-note={label}
        aria-label={`Nota ${label}: ver la cita`}
        aria-expanded={state.open}
        onPointerDown={(event) => {
          lastPointer.current = event.pointerType
        }}
        onClick={(event) => {
          event.preventDefault()
          const outcome = notePress(state, pressPointer(event.detail, lastPointer.current))
          focusCard.current = outcome.focusCard
          setState(outcome.state)
        }}
        className="mx-px cursor-pointer align-super text-[0.7em] font-medium leading-none text-primary hover:underline focus-visible:outline-2 focus-visible:outline-primary"
      >
        [{label}]
      </PreviewCardTrigger>
      <PreviewCardContent className="w-[min(28rem,90vw)] p-0">
        <div
          ref={card}
          tabIndex={-1}
          role="group"
          aria-label={`Nota ${label}`}
          className="outline-none"
          onKeyDown={leaveWithTab}
        >
          <div className="max-h-60 overflow-y-auto px-3 pt-3 pb-2">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                p: ({ children }) => <p className="my-0">{children}</p>,
                a: ({ href, children }) => {
                  const target = typeof href === "string" ? href : ""
                  return (
                    <a
                      href={target || undefined}
                      onClick={(event) => follow(event, target)}
                      className="cursor-pointer text-primary underline decoration-primary/40 underline-offset-2 hover:decoration-primary"
                    >
                      {children}
                    </a>
                  )
                },
              }}
            >
              {shown}
            </ReactMarkdown>
          </div>
          {source && (
            <div className="border-t px-3 py-2">
              <button
                type="button"
                data-card-open=""
                onClick={() => open(source.target)}
                className="max-w-full truncate rounded-sm text-start text-xs font-medium text-primary hover:underline focus-visible:outline-2 focus-visible:outline-primary"
              >
                Abrir el documento: {source.name}
              </button>
            </div>
          )}
        </div>
      </PreviewCardContent>
    </PreviewCard>
  )
}
