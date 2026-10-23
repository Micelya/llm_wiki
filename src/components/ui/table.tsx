import { useState, type CSSProperties, type ReactNode } from "react"
import { ChevronDown } from "lucide-react"
import { cn } from "@/lib/utils"

/**
 * The dense table of the style guide (D-493): rows of 36 to 40 px, small
 * headings in capitals, numbers aligned at the right, a soft background
 * under the row the mouse is on. `Table` is the frame; its rows are plain
 * `<tr>` with `Th` and `Td`.
 */
export function Table({ children, className, fixed = false }: { children: ReactNode; className?: string; fixed?: boolean }) {
  return (
    <div className={cn("overflow-x-auto rounded-[10px] border bg-card", className)}>
      <table
        className={cn(
          "w-full border-separate border-spacing-0 text-sm leading-[1.45] [&_tbody_tr:hover>td]:bg-hover [&_tr:last-child>td]:border-b-0",
          fixed && "table-fixed",
        )}
      >
        {children}
      </table>
    </div>
  )
}

export function Th({
  children,
  align = "left",
  width,
  className,
}: {
  children?: ReactNode
  align?: "left" | "right"
  width?: string
  className?: string
}) {
  return (
    <th
      style={width ? { width } : undefined}
      className={cn(
        "border-b bg-panel-sub px-3 py-2 font-ui text-xs font-bold tracking-[0.04em] whitespace-nowrap text-muted-foreground uppercase",
        align === "right" ? "text-right" : "text-left",
        className,
      )}
    >
      {children}
    </th>
  )
}

export function Td({
  children,
  align = "left",
  muted = false,
  nowrap = false,
  colSpan,
  width,
  className,
  style,
}: {
  children?: ReactNode
  align?: "left" | "right"
  /** Secondary: grey. */
  muted?: boolean
  nowrap?: boolean
  colSpan?: number
  width?: string
  className?: string
  style?: CSSProperties
}) {
  return (
    <td
      colSpan={colSpan}
      style={width ? { width, ...style } : style}
      className={cn(
        "border-b px-3 py-[9px] align-top",
        align === "right" && "text-right tabular-nums",
        muted && "text-muted-foreground",
        nowrap && "whitespace-nowrap",
        className,
      )}
    >
      {children}
    </td>
  )
}

/**
 * The first rows of a long list and a «Ver los otros…» under them: the
 * whole list when it is short. `render` gets the rows shown.
 */
export function useFirstRows<T>(rows: T[], limit: number, slack = 2): { shown: T[]; hidden: number; showAll: () => void } {
  const [all, setAll] = useState(false)
  const cut = !all && rows.length > limit + slack
  return { shown: cut ? rows.slice(0, limit) : rows, hidden: cut ? rows.length - limit : 0, showAll: () => setAll(true) }
}

/** The button that shows the rest of a long list: «Ver los otros 6 documentos». */
export function MoreButton({ children, onClick, className }: { children: ReactNode; onClick: () => void; className?: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn("inline-flex items-center gap-1 font-ui text-[0.8125rem] font-bold text-link hover:underline", className)}
    >
      <ChevronDown className="size-3.5" aria-hidden />
      {children}
    </button>
  )
}
