import { Button as ButtonPrimitive } from "@base-ui/react/button"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

/**
 * The buttons of the brand's style guide: radius of 8 px and Mulish 600, as
 * the manual says; three sizes (28, 36 and 44 px; `xs` stays for dense
 * lists). The main one is the magenta gradient in light mode and the pink
 * in dark mode; `secondary` is outlined in magenta; `outline` is the quiet
 * one; `link` reads as a link. Only the large main button grows under the
 * mouse; the others get a shadow or a background.
 */
const buttonVariants = cva(
  "group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding font-ui text-sm font-semibold whitespace-nowrap transition-[color,background-color,box-shadow,transform] duration-200 outline-none select-none focus-visible:ring-3 focus-visible:ring-ring active:not-aria-[haspopup]:translate-y-px disabled:pointer-events-none disabled:opacity-45 disabled:shadow-none aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default:
          "bg-primary [background-image:var(--btn-primary-image)] text-primary-foreground hover:shadow-primary aria-expanded:shadow-primary",
        outline:
          "border-border-strong bg-transparent text-foreground hover:bg-hover hover:text-strong aria-expanded:bg-hover",
        secondary:
          "bg-transparent text-[var(--btn-secondary-ink)] shadow-[inset_0_0_0_2px_var(--btn-secondary-line)] hover:bg-brand-soft aria-expanded:bg-brand-soft",
        ghost:
          "text-foreground hover:bg-hover hover:text-strong aria-expanded:bg-hover",
        destructive:
          "bg-err-bg text-err hover:border-err-line focus-visible:ring-err/30",
        link: "px-1.5 text-link underline-offset-4 hover:underline hover:decoration-[var(--link-line)]",
      },
      size: {
        default:
          "h-9 gap-1.5 px-4 has-data-[icon=inline-end]:pr-3 has-data-[icon=inline-start]:pl-3",
        xs: "h-6 gap-1 px-2 text-xs has-data-[icon=inline-end]:pr-1.5 has-data-[icon=inline-start]:pl-1.5 [&_svg:not([class*='size-'])]:size-3",
        sm: "h-7 gap-1.5 px-2.5 text-[0.8125rem] has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2 [&_svg:not([class*='size-'])]:size-3.5",
        lg: "h-11 gap-2 px-6 text-base has-data-[icon=inline-end]:pr-5 has-data-[icon=inline-start]:pl-5",
        icon: "size-9",
        "icon-xs": "size-6 [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-7",
        "icon-lg": "size-11",
      },
    },
    compoundVariants: [
      // Manual: only the large main button grows under the mouse.
      { variant: "default", size: "lg", className: "hover:scale-105" },
    ],
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function Button({
  className,
  variant = "default",
  size = "default",
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }
