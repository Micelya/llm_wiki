import type { ReactNode } from "react"
import { CircleAlert, CircleCheck, Info, TriangleAlert, type LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"

/**
 * The tones of the states of the brand's style guide (propuesta): good in
 * green, warning in amber, error in a red that leans to orange (far from
 * the pink of the brand) and information in a muted blue. `neutral` and
 * `brand` are not states. A state is never told only by its colour: it
 * goes with an icon and a word.
 */
export type Tone = "ok" | "warn" | "err" | "info" | "neutral" | "brand"

export const TONE_CLASSES: Record<Tone, string> = {
  ok: "border-ok-line bg-ok-bg text-ok",
  warn: "border-warn-line bg-warn-bg text-warn",
  err: "border-err-line bg-err-bg text-err",
  info: "border-info-line bg-info-bg text-info",
  neutral: "border-border bg-muted text-muted-foreground",
  brand: "border-transparent bg-brand-soft text-brand-ink",
}

/** The icon of each state, for when the place has no better one. */
export const TONE_ICONS: Record<Tone, LucideIcon | null> = {
  ok: CircleCheck,
  warn: TriangleAlert,
  err: CircleAlert,
  info: Info,
  neutral: null,
  brand: null,
}

/** The word of each state, for when the place has no better one. */
export const TONE_WORDS: Record<Tone, string | null> = {
  ok: "Bien",
  warn: "Aviso",
  err: "Problema",
  info: "Información",
  neutral: null,
  brand: null,
}

/**
 * A short label with a state («2 problemas», «Confirmado», «1 de 4 sin
 * conexión»): Mulish bold, 22 px high, with its icon before the words.
 * `icon` is the state's own icon unless one is given; `null` leaves it out
 * (only for `neutral` and `brand`, which are not states).
 */
export function Badge({
  tone = "neutral",
  icon,
  spin = false,
  title,
  className,
  children,
}: {
  tone?: Tone
  icon?: LucideIcon | null
  /** The icon turns: something is in progress. */
  spin?: boolean
  title?: string
  className?: string
  children: ReactNode
}) {
  const Icon = icon === undefined ? TONE_ICONS[tone] : icon
  return (
    <span
      title={title}
      data-tone={tone}
      className={cn(
        "inline-flex h-[22px] items-center gap-1.5 rounded-md border px-2 font-ui text-xs leading-none font-bold whitespace-nowrap",
        TONE_CLASSES[tone],
        className,
      )}
    >
      {Icon && <Icon className={cn("size-3.5 shrink-0", spin && "animate-spin")} strokeWidth={2.2} aria-hidden />}
      {children}
    </span>
  )
}

/** A key of the keyboard, as in «Ctrl K». */
export function Kbd({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <kbd
      className={cn(
        "inline-flex items-center rounded-[5px] border border-b-2 border-border-strong bg-card px-1.5 font-ui text-[11px] leading-[1.45] font-semibold text-muted-foreground",
        className,
      )}
    >
      {children}
    </kbd>
  )
}
