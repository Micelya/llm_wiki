import { useEffect, useRef, useState, type ReactNode } from "react"
import { PreviewCard, PreviewCardContent, PreviewCardTrigger } from "@/components/ui/preview-card"
import { knownLinkPreview, loadLinkPreview, type LinkPreview } from "@/lib/link-preview"
import { CLOSED, linkPress, pressPointer, previewChange, type PreviewState } from "@/lib/preview-card-behavior"
import { openPageObject, openWikiPath } from "@/lib/open-wiki-path"
import { usePreviewCardsStore } from "@/stores/preview-cards-store"

const LINK_CLASS = "cursor-pointer text-primary underline decoration-primary/40 underline-offset-2 hover:decoration-primary"

/** The name a link (`#name`) gives what it opens. */
function referenceOf(href: string): string {
  try {
    return decodeURIComponent(href.slice(1))
  } catch {
    return href.slice(1)
  }
}

/**
 * A link of an article to a person, organization, document or matter
 * (`[[name|text]]`): it opens the screen of what it names (D-407). Its
 * card (P-246, D-454) shows the type and the short summary of what it
 * names, read from its article in the base, when the mouse rests on the
 * link or the keyboard reaches it; on a touch screen the first touch
 * shows the card and the second opens the link. With the previews off in
 * Settings, it is a plain link.
 */
export function PageLink({ href, children }: { href: string; children: ReactNode }) {
  const enabled = usePreviewCardsStore((s) => s.enabled)
  const [state, setState] = useState<PreviewState>(CLOSED)
  const lastPointer = useRef<string | null>(null)
  const reference = referenceOf(href)

  function follow(event: React.MouseEvent<HTMLAnchorElement>) {
    event.preventDefault()
    setState(CLOSED)
    void openWikiPath(reference)
  }

  if (!enabled) {
    return (
      <a href={href} onClick={follow} className={LINK_CLASS}>
        {children}
      </a>
    )
  }

  return (
    <PreviewCard
      open={state.open}
      onOpenChange={(next, details) => setState((current) => previewChange(current, next, details.reason, enabled))}
    >
      <PreviewCardTrigger
        href={href}
        onPointerDown={(event) => {
          lastPointer.current = event.pointerType
        }}
        onClick={(event) => {
          const outcome = linkPress(state, pressPointer(event.detail, lastPointer.current), enabled)
          if (outcome.follow) {
            follow(event)
            return
          }
          event.preventDefault()
          setState(outcome.state)
        }}
        className={LINK_CLASS}
      >
        {children}
      </PreviewCardTrigger>
      <PreviewCardContent className="w-80 p-0">
        <LinkCard
          reference={reference}
          onOpen={(preview) => {
            setState(CLOSED)
            if (!openPageObject(preview.object)) void openWikiPath(reference)
          }}
        />
      </PreviewCardContent>
    </PreviewCard>
  )
}

/**
 * What the card of a link shows, at a fixed size: its type, its name, its
 * short summary and a button that opens it; three gray lines while the
 * base answers.
 */
export function LinkCard({ reference, onOpen }: { reference: string; onOpen: (preview: LinkPreview) => void }) {
  const [preview, setPreview] = useState<LinkPreview | null | undefined>(() => knownLinkPreview(reference))
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    if (knownLinkPreview(reference) !== undefined) return
    let current = true
    loadLinkPreview(reference).then(
      (found) => current && setPreview(found),
      () => current && setFailed(true),
    )
    // The card closed before the base answered: what it answers waits for the next time.
    return () => {
      current = false
    }
  }, [reference])

  return (
    <div className="flex h-36 flex-col gap-1.5 p-3" aria-live="polite" aria-busy={preview === undefined && !failed}>
      {preview === undefined && !failed ? (
        <div className="flex flex-1 flex-col gap-2" aria-label="Cargando la ficha">
          <div className="h-3 w-1/4 rounded bg-muted" />
          <div className="h-4 w-2/3 rounded bg-muted" />
          <div className="h-3 w-full rounded bg-muted" />
          <div className="h-3 w-5/6 rounded bg-muted" />
        </div>
      ) : preview ? (
        <>
          <p className="text-[0.7rem] font-medium tracking-wide text-muted-foreground uppercase">{preview.typeLabel}</p>
          <p className="truncate font-semibold text-foreground">{preview.title}</p>
          <p className="line-clamp-3 flex-1 text-xs leading-relaxed text-muted-foreground">
            {preview.summary ?? "Sin resumen todavía: lo que dicen los documentos está en su página."}
          </p>
          <button
            type="button"
            onClick={() => onOpen(preview)}
            className="self-start rounded-sm text-xs font-medium text-primary hover:underline focus-visible:outline-2 focus-visible:outline-primary"
          >
            Abrir
          </button>
        </>
      ) : (
        <p className="text-xs text-muted-foreground">
          {failed ? "No se pudo leer la ficha en la base." : "La base no tiene una ficha con este nombre."}
        </p>
      )}
    </div>
  )
}
