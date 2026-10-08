/**
 * Shows this product's name wherever a translated text names the
 * original app.
 *
 * The language files come from upstream and are the files it changes
 * most, so they are left untouched: every text that goes through `t()`
 * — including fallback texts written in components — is passed through
 * this post-processor instead. Texts added upstream later are covered
 * without any further change here.
 */
import type { PostProcessorModule } from "i18next"
import { PRODUCT_NAME, UPSTREAM_PRODUCT_NAME } from "@/lib/app-identity"

export function withProductName(text: string): string {
  return text.split(UPSTREAM_PRODUCT_NAME).join(PRODUCT_NAME)
}

export const productNamePostProcessor: PostProcessorModule = {
  type: "postProcessor",
  name: "productName",
  process: (value) => (typeof value === "string" ? withProductName(value) : value),
}
