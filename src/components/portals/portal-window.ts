import { hidePortal, showPortal } from "@/commands/portals"

/** An area of the app's window, in CSS pixels. */
export interface PortalRect {
  x: number
  y: number
  width: number
  height: number
}

/** What moves the portal's window: the app's commands, or a fake in the tests. */
export interface PortalWindowCommands {
  show: (portal: string, rect: PortalRect) => Promise<void>
  hide: (portal: string) => Promise<void>
}

type Wish = { shown: true; rect: PortalRect } | { shown: false }

function keyOf(wish: Wish): string {
  return wish.shown ? `${wish.rect.x},${wish.rect.y},${wish.rect.width},${wish.rect.height}` : "hidden"
}

/**
 * The portal's window as the page wants it: shown over an area, or
 * hidden. The window outlives the page (it leaves, it reloads, and in
 * development React mounts it twice), so the page only says what it wants
 * and this applies it one command at a time, in order, always ending with
 * the last wish: a quick show-hide-show never leaves the window hidden nor
 * asks to create it twice, and a command that failed is tried again on
 * the next wish.
 */
export class PortalWindow {
  private wish: Wish = { shown: false }
  /**
   * What the window shows for sure; null when it is not known (the app
   * just opened, or a command failed), so the next wish is always sent.
   */
  private applied: string | null = null
  private queue: Promise<void> = Promise.resolve()

  constructor(
    private readonly portal: string,
    private readonly commands: PortalWindowCommands,
  ) {}

  /** Shows the window over this area, creating it only if there is none. */
  show(rect: PortalRect): Promise<void> {
    this.wish = { shown: true, rect: { ...rect } }
    return this.settle()
  }

  /** Hides the window, so it does not cover other pages. */
  hide(): Promise<void> {
    this.wish = { shown: false }
    return this.settle()
  }

  private settle(): Promise<void> {
    const run = this.queue.then(() => this.apply())
    this.queue = run.catch(() => {})
    return run
  }

  private async apply(): Promise<void> {
    let failed = false
    let failure: unknown
    while (keyOf(this.wish) !== this.applied) {
      const wish = this.wish
      const key = keyOf(wish)
      try {
        if (wish.shown) await this.commands.show(this.portal, wish.rect)
        else await this.commands.hide(this.portal)
        this.applied = key
        failed = false
      } catch (cause) {
        this.applied = null
        failed = true
        failure = cause
        // Only a newer wish is tried now; the same one waits for the next call.
        if (keyOf(this.wish) === key) break
      }
    }
    if (failed) throw failure
  }
}

const windows = new Map<string, PortalWindow>()

/** The one window of each portal, for the whole app. */
export function portalWindow(portal: string): PortalWindow {
  let found = windows.get(portal)
  if (!found) {
    found = new PortalWindow(portal, { show: showPortal, hide: hidePortal })
    windows.set(portal, found)
  }
  return found
}
