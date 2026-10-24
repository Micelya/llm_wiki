import { useEffect, useRef, useState, type ReactNode } from "react"
import { ChevronRight } from "lucide-react"
import { cn } from "@/lib/utils"

/**
 * The fact box of a page, as the mockup (D-493): a card with the brand's
 * line on top, what it is in small capitals and its name; below, rows of
 * a datum and its value, grouped under small headings.
 */
export function InfoBox({ kind, name, badges, children }: { kind: string; name: ReactNode; badges?: ReactNode; children?: ReactNode }) {
  return (
    <div data-infobox="" className="overflow-hidden rounded-xl border bg-card text-[0.84375rem] leading-[1.45] shadow-card">
      <div className="relative border-b bg-panel-sub px-4 pt-3.5 pb-3">
        <span aria-hidden className="absolute inset-x-0 top-0 h-[3px] bg-grad-principal" />
        <p className="font-ui text-[0.6875rem] font-bold tracking-[0.08em] text-muted-foreground uppercase">{kind}</p>
        <div className="mt-0.5 font-ui text-base leading-[1.3] font-bold break-words text-strong">{name}</div>
        {badges && <div className="mt-2 flex flex-wrap gap-1.5">{badges}</div>}
      </div>
      {children}
    </div>
  )
}

/** A small heading inside the fact box: «En el archivo», «Espera una decisión». */
export function InfoHeading({ children }: { children: ReactNode }) {
  return (
    <div className="border-y bg-panel-sub px-4 pt-1.5 pb-1 font-ui text-[0.6875rem] font-bold tracking-[0.08em] text-muted-foreground uppercase first:border-t-0">
      {children}
    </div>
  )
}

/** The rows of the fact box. */
export function InfoRows({ children }: { children: ReactNode }) {
  return <dl className="divide-y">{children}</dl>
}

/** A datum of the fact box: its name at the left, its value, and a small line under it. */
export function InfoRow({ label, children, below }: { label: ReactNode; children: ReactNode; below?: ReactNode }) {
  return (
    <div className="grid grid-cols-[40%_minmax(0,1fr)] gap-2.5 px-4 py-[7px]">
      <dt className="font-ui text-[0.78125rem] font-semibold text-muted-foreground">{label}</dt>
      <dd className="min-w-0 [overflow-wrap:anywhere] text-foreground">
        {children}
        {below && <span className="block text-xs text-muted-foreground">{below}</span>}
      </dd>
    </div>
  )
}

/**
 * What waits for a person on this page, each with its mark (a dot for a
 * problem of the data, a diamond for a decision) and the way to it.
 */
export function InfoPending({ items }: { items: { tone: "warn" | "decision"; text: ReactNode; action?: ReactNode }[] }) {
  if (items.length === 0) return null
  return (
    <div className="flex flex-col gap-2 px-4 pt-2.5 pb-3.5">
      {items.map((item, index) => (
        <div key={index} className="flex items-start gap-2">
          <span
            aria-hidden
            className={cn(
              "mt-[5px] size-2 shrink-0",
              item.tone === "warn"
                ? "rounded-full bg-warn shadow-[0_0_0_3px_var(--warn-bg)]"
                : "scale-90 rotate-45 rounded-[2px] bg-info shadow-[0_0_0_3px_var(--info-bg)]",
            )}
          />
          <span>
            {item.text} {item.action}
          </span>
        </div>
      ))}
    </div>
  )
}

/** A link inside a text that does something on the screen (a tab, a list). */
export function TextLink({ children, onClick, title }: { children: ReactNode; onClick: () => void; title?: string }) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className="text-link underline decoration-[var(--link-line)] underline-offset-[3px] hover:decoration-link"
    >
      {children}
    </button>
  )
}

export interface IndexEntry {
  id: string
  label: string
}

/**
 * «En esta página»: the sections of the page, the one being read marked,
 * that stays in view while the page goes down (D-493). A click takes the
 * page to the section. `jump` goes at its foot: the other tab.
 */
export function PageIndex({ entries, jump }: { entries: IndexEntry[]; jump?: { label: string; onClick: () => void } }) {
  const ref = useRef<HTMLElement | null>(null)
  const [active, setActive] = useState<string | null>(entries[0]?.id ?? null)
  const ids = entries.map((entry) => entry.id).join(" ")

  useEffect(() => {
    const scroller = ref.current?.closest<HTMLElement>("[data-detail-scroll]")
    if (!scroller) return
    let frame = 0
    const update = () => {
      frame = 0
      const top = scroller.getBoundingClientRect().top + 96
      let current: string | null = ids.split(" ")[0] || null
      for (const id of ids.split(" ")) {
        const element = document.getElementById(id)
        if (element && element.getBoundingClientRect().top <= top) current = id
      }
      // At the foot of the page, the last section is the one read.
      if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 4) current = ids.split(" ").slice(-1)[0] ?? current
      setActive(current)
    }
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }
    update()
    scroller.addEventListener("scroll", onScroll, { passive: true })
    return () => {
      scroller.removeEventListener("scroll", onScroll)
      if (frame) cancelAnimationFrame(frame)
    }
  }, [ids])

  if (entries.length === 0 && !jump) return null
  return (
    <nav ref={ref} aria-label="En esta página" className="sticky top-6 pt-1 pl-4 font-ui">
      <p className="mb-2.5 font-ui text-xs font-bold tracking-[0.08em] text-muted-foreground uppercase">En esta página</p>
      <ol className="border-l-2">
        {entries.map((entry) => (
          <li key={entry.id}>
            <a
              href={`#${entry.id}`}
              aria-current={active === entry.id ? "location" : undefined}
              onClick={(event) => {
                event.preventDefault()
                document.getElementById(entry.id)?.scrollIntoView({ behavior: "smooth", block: "start" })
                setActive(entry.id)
              }}
              className={cn(
                "-ml-0.5 block border-l-2 py-[5px] pl-3 text-[0.84375rem] leading-[1.35] font-semibold transition-colors",
                active === entry.id
                  ? "border-brand text-brand-ink"
                  : "border-transparent text-muted-foreground hover:border-border-strong hover:text-strong",
              )}
            >
              {entry.label}
            </a>
          </li>
        ))}
      </ol>
      {jump && (
        <button
          type="button"
          onClick={jump.onClick}
          className="mt-4 inline-flex items-center gap-1.5 font-ui text-[0.8125rem] font-semibold text-link hover:underline"
        >
          {jump.label} <ChevronRight className="size-3" aria-hidden />
        </button>
      )}
    </nav>
  )
}

/** The fact box, and under it the index of the page that stays in view. */
export function PageAside({ box, index }: { box?: ReactNode; index?: ReactNode }) {
  return (
    <div className="flex h-full flex-col gap-5">
      {box}
      {index && <div className="hidden flex-1 xl:block">{index}</div>}
    </div>
  )
}
