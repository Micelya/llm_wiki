import remarkMath from "remark-math"

/**
 * How every reader of the app turns `$` into a formula.
 *
 * A legal archive is full of amounts ("$ 1.500.000", "USD $ 300"). With
 * remark-math's default, whatever sits between two single `$` on a line is
 * drawn as a formula: the words lose their spaces and look glued together.
 * Only `$$…$$` is math here; a single `$` is always the currency sign.
 * Disabling it in the readers (rather than escaping when pages are written)
 * covers every page at once: the ones the app projects, the ones the wiki
 * model writes and the notes a person types.
 */
export const REMARK_MATH_OPTIONS = { singleDollarTextMath: false } as const

export const remarkMathPlugin: [typeof remarkMath, typeof REMARK_MATH_OPTIONS] = [
  remarkMath,
  REMARK_MATH_OPTIONS,
]
