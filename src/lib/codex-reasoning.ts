/**
 * How much the Codex CLI model reasons before answering, for the questions
 * about documents (Rust, `source_volume/asking.rs`) and for the wiki's own
 * calls (`codex-cli-transport.ts`).
 *
 * Only `low`, `medium` and `high` are offered (D-274): higher levels exist
 * in some models, but the process is improved by adjusting the questions,
 * not by spending more on each answer. The level is part of the engine's
 * name in the document base, so answers given with another level are never
 * mixed with these.
 */
export const CODEX_REASONING_LEVELS = ["low", "medium", "high"] as const

export type CodexReasoningEffort = (typeof CODEX_REASONING_LEVELS)[number]

/** The level used when none was chosen: the variant that won the comparison (D-279). */
export const DEFAULT_CODEX_REASONING: CodexReasoningEffort = "medium"

/** A saved level, or the default when it is missing or not offered. */
export function codexReasoningEffort(value: unknown): CodexReasoningEffort {
  return CODEX_REASONING_LEVELS.includes(value as CodexReasoningEffort)
    ? (value as CodexReasoningEffort)
    : DEFAULT_CODEX_REASONING
}
