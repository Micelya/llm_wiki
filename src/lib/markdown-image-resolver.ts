/**
 * Resolve markdown image `src` attributes so they actually load in
 * the Tauri webview.
 *
 * In Tauri there is no URL context for arbitrary file paths, so a local
 * image has to go through `convertFileSrc`. Only images inside one
 * folder (`root`, the agent's folder where the chat's outputs are) are
 * served that way, so markdown can never turn into a read of any other
 * local file:
 *
 *   - Any src starting with `http://`, `https://`, `data:`, `blob:`,
 *     `file:`, `tauri://` is passed through unchanged.
 *   - An absolute src is converted only when it stays inside `root`.
 *   - A relative src is resolved against the rendering markdown file's
 *     own directory (`currentFileDir`) when it is known, like Obsidian
 *     and every other markdown tool, or against `root` otherwise; it is
 *     converted only when the result stays inside `root`.
 *
 * Anything else is returned verbatim.
 */
import { convertFileSrc } from "@tauri-apps/api/core"
import { normalizePath } from "@/lib/path-utils"

const PASSTHROUGH_RE = /^(https?:|data:|blob:|file:|tauri:)/i

function trimTrailingSlash(path: string): string {
  return path.replace(/\/+$/, "")
}

function comparePath(path: string): string {
  return /^[a-zA-Z]:/.test(path) ? path.toLowerCase() : path
}

function isInside(path: string, root: string): boolean {
  const base = comparePath(trimTrailingSlash(normalizePath(root)))
  const candidate = comparePath(trimTrailingSlash(normalizePath(path)))
  return candidate === base || candidate.startsWith(`${base}/`)
}

function decodePathSrc(src: string): string {
  try {
    return decodeURIComponent(src)
  } catch {
    return src
  }
}

function isAbsolutePath(path: string): boolean {
  return path.startsWith("/") || /^[a-zA-Z]:/.test(path) || path.startsWith("\\\\")
}

/**
 * Collapse `.` and `..` segments in a forward-slashed path without
 * touching the filesystem. A leading `..` that would escape the
 * root is dropped (clamped at root) rather than throwing — image
 * references should degrade gracefully, not crash the renderer.
 */
function collapsePath(p: string): string {
  const isAbsolute = p.startsWith("/")
  const out: string[] = []
  for (const seg of p.split("/")) {
    if (seg === "" || seg === ".") continue
    if (seg === "..") {
      if (out.length > 0 && out[out.length - 1] !== "..") out.pop()
      else if (!isAbsolute) out.push("..")
      // absolute path: `..` above root is simply ignored
    } else {
      out.push(seg)
    }
  }
  return (isAbsolute ? "/" : "") + out.join("/")
}

/**
 * `root` is the folder images may come from; with none, every src is
 * passed through unchanged. `currentFileDir` is the directory of the
 * markdown file being rendered (absolute, or relative to `root`).
 */
export function resolveMarkdownImageSrc(
  rawSrc: string,
  root: string | null,
  currentFileDir?: string | null,
): string {
  if (!rawSrc) return rawSrc
  if (PASSTHROUGH_RE.test(rawSrc)) return rawSrc
  if (!root) return rawSrc

  const base = normalizePath(root)
  if (isAbsolutePath(rawSrc)) {
    const absolute = collapsePath(normalizePath(decodePathSrc(rawSrc)))
    return isInside(absolute, base) ? convertFileSrc(absolute) : rawSrc
  }

  // ReactMarkdown / remark percent-encode non-ASCII characters in image
  // URLs; the path on disk is the literal UTF-8 one, so decode before
  // assembling it (convertFileSrc would encode the `%` again and 404).
  const cleaned = decodePathSrc(rawSrc.replace(/^\.\//, ""))
  let dir = base
  if (currentFileDir) {
    const fileDir = normalizePath(currentFileDir)
    dir = isAbsolutePath(fileDir) ? fileDir : `${base}/${fileDir}`
  }
  const absolute = collapsePath(`${trimTrailingSlash(dir)}/${cleaned}`)
  return isInside(absolute, base) ? convertFileSrc(absolute) : rawSrc
}
