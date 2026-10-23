import type { ReactNode } from "react"
import type { LucideIcon } from "lucide-react"
import { Badge, type Tone as BrandTone } from "@/components/ui/badge"
import { Callout } from "@/components/ui/callout"

export { Panel } from "@/components/ui/panel"

/**
 * Frame shared by the document base screens: the name of the screen large
 * in Mulish, what it is in grey below, its actions at the right, and the
 * body that scrolls.
 */
export function DocbasePage({
  title,
  subtitle,
  actions,
  children,
}: {
  title: string
  subtitle?: ReactNode
  actions?: ReactNode
  children: ReactNode
}) {
  return (
    <div className="flex h-full flex-col">
      <header className="flex shrink-0 items-start justify-between gap-4 border-b bg-background px-10 pt-7 pb-5">
        <div className="min-w-0">
          <h1 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">{title}</h1>
          {subtitle && (
            <p className="mt-1 font-ui text-sm font-medium text-muted-foreground first-letter:uppercase">{subtitle}</p>
          )}
        </div>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-10 py-7">{children}</div>
    </div>
  )
}

export type Tone = "neutral" | "ok" | "busy" | "warning" | "error"

const BRAND_TONE: Record<Tone, BrandTone> = {
  neutral: "neutral",
  ok: "ok",
  busy: "info",
  warning: "warn",
  error: "err",
}

/**
 * A small label in a list. Its words say the state; an `icon` can go with
 * them (the style guide's badges always carry one where they tell a state).
 */
export function StatusBadge({
  tone,
  children,
  title,
  icon = null,
  spin = false,
}: {
  tone: Tone
  children: ReactNode
  title?: string
  icon?: LucideIcon | null
  spin?: boolean
}) {
  return (
    <Badge tone={BRAND_TONE[tone]} icon={icon} spin={spin} title={title} className="h-auto rounded px-1.5 py-0.5 font-semibold">
      {children}
    </Badge>
  )
}

export function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <Callout tone="err" title={null} className="px-3 py-2">
      {children}
    </Callout>
  )
}

export function EmptyNote({ children }: { children: ReactNode }) {
  return <p className="py-6 text-center text-sm text-muted-foreground">{children}</p>
}
