"use client"

import { PreviewCard as PreviewCardPrimitive } from "@base-ui/react/preview-card"

import { cn } from "@/lib/utils"
import { CLOSE_DELAY_MS, OPEN_DELAY_MS } from "@/lib/preview-card-behavior"

/**
 * The floating card of a note or a link in an article (P-246), wrapped
 * like `tooltip`: base-ui opens it when the mouse rests on its trigger or
 * the keyboard focuses it, keeps a safe zone between the two, and closes
 * it with Escape. Its size is fixed by who uses it, so that loading does
 * not make it jump; it only fades in when the system allows motion.
 */
function PreviewCard<Payload>({ ...props }: PreviewCardPrimitive.Root.Props<Payload>) {
  return <PreviewCardPrimitive.Root data-slot="preview-card" {...props} />
}

function PreviewCardTrigger({ delay = OPEN_DELAY_MS, closeDelay = CLOSE_DELAY_MS, ...props }: PreviewCardPrimitive.Trigger.Props) {
  return <PreviewCardPrimitive.Trigger data-slot="preview-card-trigger" delay={delay} closeDelay={closeDelay} {...props} />
}

function PreviewCardContent({
  className,
  side = "top",
  sideOffset = 6,
  align = "center",
  children,
  ...props
}: PreviewCardPrimitive.Popup.Props &
  Pick<PreviewCardPrimitive.Positioner.Props, "align" | "side" | "sideOffset">) {
  return (
    <PreviewCardPrimitive.Portal>
      <PreviewCardPrimitive.Positioner
        side={side}
        sideOffset={sideOffset}
        align={align}
        collisionPadding={8}
        className="isolate z-50"
      >
        <PreviewCardPrimitive.Popup
          data-slot="preview-card-content"
          className={cn(
            "z-50 origin-(--transform-origin) rounded-md border bg-popover p-3 text-sm leading-relaxed text-popover-foreground shadow-md outline-none",
            "motion-safe:data-open:animate-in motion-safe:data-open:fade-in-0 motion-safe:data-open:zoom-in-95 motion-safe:data-closed:animate-out motion-safe:data-closed:fade-out-0 motion-safe:data-closed:zoom-out-95",
            className,
          )}
          {...props}
        >
          {children}
        </PreviewCardPrimitive.Popup>
      </PreviewCardPrimitive.Positioner>
    </PreviewCardPrimitive.Portal>
  )
}

export { PreviewCard, PreviewCardTrigger, PreviewCardContent }
