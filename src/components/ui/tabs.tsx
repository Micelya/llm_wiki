import type { ReactNode } from "react"
import { cn } from "@/lib/utils"
import { TONE_CLASSES, type Tone } from "./badge"

/**
 * A choice among a few, as a row of small chips («Todos», «Personas»,
 * «Organizaciones»): the chosen one in the brand's soft magenta.
 */
export function FilterChip({
  active,
  onClick,
  count,
  title,
  children,
}: {
  active: boolean
  onClick: () => void
  count?: number
  title?: string
  children: ReactNode
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      title={title}
      onClick={onClick}
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-lg border px-2.5 font-ui text-[0.8125rem] font-semibold whitespace-nowrap transition-colors",
        active ? "border-transparent bg-brand-soft font-bold text-brand-ink" : "border-border-strong text-muted-foreground hover:bg-hover hover:text-strong",
      )}
    >
      {children}
      {count !== undefined && <span className="tabular-nums opacity-80">{count}</span>}
    </button>
  )
}

/** A row of tabs, over a line; its tabs are `Tab`. */
export function TabList({ label, className, children }: { label?: string; className?: string; children: ReactNode }) {
  return (
    <div role="tablist" aria-label={label} className={cn("flex gap-1 border-b", className)}>
      {children}
    </div>
  )
}

/**
 * A tab of the style guide: Mulish, the chosen one in bold with the
 * brand's gradient under it, and how many things it has in a small round
 * count (amber when they wait for a person, `countTone="warn"`).
 */
export function Tab({
  active,
  count,
  countTone = "neutral",
  onClick,
  className,
  children,
}: {
  active: boolean
  count?: number | string
  countTone?: Tone
  onClick: () => void
  className?: string
  children: ReactNode
}) {
  return (
    <button
      role="tab"
      aria-selected={active}
      onClick={onClick}
      className={cn(
        "relative -mb-px inline-flex h-10 items-center gap-2 rounded-t-lg px-3.5 font-ui text-[0.90625rem] whitespace-nowrap transition-colors",
        active ? "font-bold text-strong" : "font-semibold text-muted-foreground hover:bg-hover hover:text-strong",
        className,
      )}
    >
      {children}
      {count !== undefined && count !== 0 && count !== "" && (
        <span
          className={cn(
            "inline-flex h-5 min-w-5 items-center justify-center rounded-full border px-1.5 text-xs font-bold",
            TONE_CLASSES[countTone],
          )}
        >
          {count}
        </span>
      )}
      {active && <span aria-hidden className="absolute inset-x-2.5 -bottom-px h-[3px] rounded-t-[3px] bg-grad-vibrante" />}
    </button>
  )
}
