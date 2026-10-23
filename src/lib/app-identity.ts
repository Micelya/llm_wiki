/**
 * Where this build of the app is published. Kept in one place so the
 * update check never points users at another project's downloads.
 */
export const RELEASES_REPO = "Micelya/llm_wiki"

/** Name shown to users. The installer and window take theirs from tauri.conf.json. */
export const PRODUCT_NAME = "Micelya Desktop"

/** The site of the product, in Ajustes → Acerca de. */
export const SITE_URL = "https://micelya.com/"

/** Where the source code of this app is, in Ajustes → Acerca de and the legal notice. */
export const SOURCE_URL = `https://github.com/${RELEASES_REPO}`

/**
 * The legal notice the licence of the code this app derives from asks
 * for (D-485): its author's copyright, that this version was modified,
 * the licence, that there is no warranty and where the source is. It is
 * the only place that names the original project; the full text of the
 * licence is the repository's `LICENSE`, shipped with the app.
 */
export const LEGAL_NOTICE =
  "Micelya Desktop incluye código derivado de LLM Wiki (© 2024-2026 Yong Su), distribuido bajo la licencia GNU GPL versión 3. " +
  "Esta versión fue modificada por Micelya. Se distribuye sin ninguna garantía. " +
  "El código fuente está en github.com/Micelya/llm_wiki."
