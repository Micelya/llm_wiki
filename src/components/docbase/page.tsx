import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

/** Frame shared by the document base screens: title, actions, scrolling body. */
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
      <header className="flex shrink-0 items-start justify-between gap-4 border-b px-8 py-5">
        <div className="min-w-0">
          <h1 className="text-xl font-semibold">{title}</h1>
          {subtitle && <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p>}
        </div>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-6">{children}</div>
    </div>
  )
}

export type Tone = "neutral" | "ok" | "busy" | "warning" | "error"

const TONES: Record<Tone, string> = {
  neutral: "bg-muted text-muted-foreground",
  ok: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  busy: "bg-blue-500/10 text-blue-700 dark:text-blue-400",
  warning: "bg-amber-500/15 text-amber-700 dark:text-amber-400",
  error: "bg-destructive/10 text-destructive",
}

export function StatusBadge({ tone, children, title }: { tone: Tone; children: ReactNode; title?: string }) {
  return (
    <span
      title={title}
      className={cn(
        "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-medium whitespace-nowrap",
        TONES[tone],
      )}
    >
      {children}
    </span>
  )
}

export function Panel({
  title,
  count,
  action,
  children,
  className,
}: {
  title?: ReactNode
  count?: ReactNode
  action?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <section className={cn("rounded-lg border bg-card", className)}>
      {title && (
        <div className="flex items-center justify-between gap-3 border-b px-4 py-2.5">
          <h2 className="flex items-center gap-2 text-sm font-semibold">
            {title}
            {count !== undefined && (
              <span className="rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {count}
              </span>
            )}
          </h2>
          {action}
        </div>
      )}
      <div className="px-4 py-3">{children}</div>
    </section>
  )
}

export function ErrorNote({ children }: { children: ReactNode }) {
  return (
    <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {children}
    </div>
  )
}

export function EmptyNote({ children }: { children: ReactNode }) {
  return <p className="py-6 text-center text-sm text-muted-foreground">{children}</p>
}
