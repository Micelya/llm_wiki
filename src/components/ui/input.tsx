import * as React from "react"
import { Input as InputPrimitive } from "@base-ui/react/input"

import { cn } from "@/lib/utils"

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <InputPrimitive
      type={type}
      data-slot="input"
      className={cn(
        // The field of the style guide: the panel's colour, a strong grey line,
        // and the link's colour with a soft ring when it has the focus.
        "h-8 w-full min-w-0 rounded-lg border border-border-strong bg-card px-2.5 py-1 text-base transition-colors outline-none file:inline-flex file:h-6 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:border-link focus-visible:ring-3 focus-visible:ring-ring/30 disabled:pointer-events-none disabled:cursor-not-allowed disabled:bg-muted disabled:opacity-50 aria-invalid:border-err aria-invalid:ring-3 aria-invalid:ring-err/20 md:text-sm",
        className
      )}
      {...props}
    />
  )
}

export { Input }
