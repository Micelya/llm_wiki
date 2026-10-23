import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

/**
 * A card of the style guide: white (or the panel colour in dark mode),
 * corners of 12 px, a soft shadow, and a head with its title in Mulish,
 * what it is about in grey, and its actions at the right. `flush` leaves
 * the body without padding, for lists whose rows run edge to edge.
 */
export function Panel({
  title,
  count,
  meta,
  action,
  flush = false,
  className,
  bodyClassName,
  children,
}: {
  title?: ReactNode
  /** How many things it has, in a small round count after the title. */
  count?: ReactNode
  /** A word in grey after the title: «en las carpetas incorporadas». */
  meta?: ReactNode
  action?: ReactNode
  flush?: boolean
  className?: string
  bodyClassName?: string
  children: ReactNode
}) {
  return (
    <section className={cn("rounded-xl border bg-card text-card-foreground shadow-card", className)}>
      {title && (
        <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1 border-b px-[18px] py-3">
          <h2 className="flex items-center gap-2 font-ui text-base font-bold text-strong">
            {title}
            {count !== undefined && <CountPill>{count}</CountPill>}
          </h2>
          {meta && <span className="text-[0.8125rem] text-muted-foreground">{meta}</span>}
          {action && <div className="ml-auto flex items-center gap-2">{action}</div>}
        </div>
      )}
      <div className={cn(!flush && "px-[18px] py-4", bodyClassName)}>{children}</div>
    </section>
  )
}

/** A small round count, grey: how many things a title or a tab has. */
export function CountPill({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex h-[22px] min-w-[22px] items-center justify-center rounded-full bg-muted px-[7px] font-ui text-xs font-bold text-muted-foreground",
        className,
      )}
    >
      {children}
    </span>
  )
}

/** A small heading in capitals, grey: «Los más nombrados». */
export function MiniHeading({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <h3 className={cn("mb-2 font-ui text-xs font-bold tracking-[0.08em] text-muted-foreground uppercase", className)}>
      {children}
    </h3>
  )
}

/** A large figure with what it counts below: «1.284 documentos leídos». */
export function StatTile({
  value,
  label,
  onClick,
  title,
}: {
  value: ReactNode
  label: ReactNode
  onClick?: () => void
  title?: string
}) {
  const body = (
    <>
      <span className="block font-ui text-[1.625rem] leading-[1.1] font-bold text-strong tabular-nums">{value}</span>
      <span className="mt-0.5 block text-[0.8125rem] text-muted-foreground">{label}</span>
    </>
  )
  const box = "flex flex-col items-start justify-start rounded-[10px] border bg-panel-sub px-3.5 py-3 text-left"
  return onClick ? (
    <button className={cn(box, "transition-colors hover:border-border-strong hover:bg-hover")} onClick={onClick} title={title}>
      {body}
    </button>
  ) : (
    <div className={box} title={title}>
      {body}
    </div>
  )
}

/** A list of figures, one per row, the name at the left and the value at the right. */
export function MetricList({ children, className }: { children: ReactNode; className?: string }) {
  return <dl className={cn("flex flex-col divide-y", className)}>{children}</dl>
}

export function MetricRow({ label, children, below }: { label: ReactNode; children: ReactNode; below?: ReactNode }) {
  return (
    <div className="py-2.5 first:pt-0 last:pb-0">
      <div className="flex items-center justify-between gap-3">
        <dt className="text-[0.84375rem] text-muted-foreground">{label}</dt>
        <dd className="flex items-center gap-1.5 font-ui font-bold text-strong tabular-nums">{children}</dd>
      </div>
      {below}
    </div>
  )
}

/** How far something went, as a thin bar with the brand's gradient. */
export function Meter({ share, label, className }: { share: number; label?: string; className?: string }) {
  const percent = Math.round(Math.max(0, Math.min(1, share)) * 100)
  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      aria-label={label}
      className={cn("h-1.5 overflow-hidden rounded-full bg-border", className)}
    >
      <span className="block h-full rounded-full bg-grad-principal" style={{ width: `${percent}%` }} />
    </div>
  )
}
