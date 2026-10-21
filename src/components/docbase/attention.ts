import type { DocumentState, Health } from "@/commands/docbase"

/**
 * Figures about how the archive is doing, for Inicio. What needs a
 * person is no longer listed here: it is raised by the sources of the
 * attention view (`src/components/attention/sources/`); what is still
 * being processed is counted by the stages of «En curso» (D-424).
 */

/** Share of the processable files whose text is already in the base, 0 to 100. */
export function processedPercent(health: Health): number | null {
  const count = (state: DocumentState) => health.byState[state] ?? 0
  const processable = health.locations - count("link") - count("unsupported") - count("excluded")
  if (processable <= 0) return null
  return Math.round((count("processed") / processable) * 1000) / 10
}
