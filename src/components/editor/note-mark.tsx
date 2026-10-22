import { useMemo, useState } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { Popover } from "@base-ui/react/popover"
import { hideHtmlComments, transformWikilinks } from "@/lib/wikilink-transform"
import { openWikiPath } from "@/lib/open-wiki-path"

/**
 * The number of a note of an article (D-441), like the notes of an
 * encyclopedia: touched, it shows the note beside it (the quote and the
 * document it is from), and the document opens from its name. The element
 * carries its note (`data-note`), for a floating preview later (P-246).
 */
export function NoteMark({ label, note }: { label: string; note: string }) {
  const [open, setOpen] = useState(false)
  const shown = useMemo(() => transformWikilinks(hideHtmlComments(note)), [note])

  function follow(event: React.MouseEvent<HTMLAnchorElement>, href: string) {
    if (!href.startsWith("#")) return
    event.preventDefault()
    setOpen(false)
    let slug = href.slice(1)
    try {
      slug = decodeURIComponent(slug)
    } catch {
      // As written.
    }
    void openWikiPath(slug)
  }

  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger
        data-note={label}
        aria-label={`Nota ${label}: ver la cita`}
        title="Ver la cita"
        className="mx-px cursor-pointer align-super text-[0.7em] font-medium leading-none text-primary hover:underline focus-visible:outline-2 focus-visible:outline-primary"
      >
        [{label}]
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner side="top" sideOffset={6} className="z-50">
          <Popover.Popup className="max-h-72 w-[min(28rem,90vw)] overflow-y-auto rounded-md border bg-popover p-3 text-sm leading-relaxed text-popover-foreground shadow-md">
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
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}
