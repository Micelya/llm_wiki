import type { ReactNode } from "react"
import { Lightbulb, type LucideIcon } from "lucide-react"
import { cn } from "@/lib/utils"
import { TONE_ICONS, TONE_WORDS } from "./badge"

/** `quiet` is a hint, dashed and without a state. */
export type CalloutTone = "ok" | "warn" | "err" | "info" | "quiet"

const BOX: Record<CalloutTone, string> = {
  ok: "border-ok-line bg-ok-bg",
  warn: "border-warn-line bg-warn-bg",
  err: "border-err-line bg-err-bg",
  info: "border-info-line bg-info-bg",
  quiet: "border-dashed border-border-strong bg-transparent text-muted-foreground",
}

const MARK: Record<CalloutTone, string> = {
  ok: "text-ok",
  warn: "text-warn",
  err: "text-err",
  info: "text-info",
  quiet: "text-muted-foreground",
}

/**
 * A notice of the style guide: «Bien», «Aviso», «Problema» or
 * «Información», each with its icon and its word, and what it says below.
 * The word is the state's own unless a `title` is given; `title={null}`
 * leaves it out only where the text already says the state. An action
 * (a button) can go at its right.
 */
export function Callout({
  tone = "info",
  title,
  icon,
  action,
  className,
  children,
}: {
  tone?: CalloutTone
  title?: ReactNode | null
  icon?: LucideIcon
  action?: ReactNode
  className?: string
  children?: ReactNode
}) {
  const Icon = icon ?? TONE_ICONS[tone as keyof typeof TONE_ICONS] ?? Lightbulb
  const heading = title === undefined ? TONE_WORDS[tone as keyof typeof TONE_WORDS] : title
  return (
    <div
      role={tone === "err" ? "alert" : undefined}
      data-tone={tone}
      className={cn("flex items-start gap-3 rounded-[10px] border px-3.5 py-3 text-sm", BOX[tone], className)}
    >
      <Icon className={cn("mt-0.5 size-[1.125rem] shrink-0", MARK[tone])} aria-hidden />
      <div className="min-w-0 flex-1">
        {heading && <p className="mb-1 font-ui text-sm font-bold text-strong">{heading}</p>}
        {children}
      </div>
      {action && <div className="flex shrink-0 items-center gap-2">{action}</div>}
    </div>
  )
}
