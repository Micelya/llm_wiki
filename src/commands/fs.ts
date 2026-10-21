import { invoke } from "@tauri-apps/api/core"
import type { FileNode } from "@/types/wiki"
import { isAbsolutePath } from "@/lib/path-utils"

export async function readFile(
  path: string,
  options?: { extractImages?: boolean },
): Promise<string> {
  return invoke<string>("read_file", {
    path,
    extractImages: options?.extractImages,
  })
}

export async function writeFile(path: string, contents: string): Promise<void> {
  assertAbsoluteFsPath("writeFile", path)
  return invoke<void>("write_file", { path, contents })
}

/**
 * List a directory tree. Dot-prefixed entries (`.claude`, `.env`, …) are
 * hidden by default; pass `includeHidden: true` to list them too. See
 * `entry_is_visible` in fs.rs.
 */
export interface ListDirectoryOptions {
  includeHidden?: boolean
  maxDepth?: number
}

// In-flight dedupe only: entries are removed when the request settles. Each
// caller receives its own tree copy when a request is actually shared, so
// accidental in-place mutations do not leak across concurrent waiters.
interface PendingListDirectory {
  request: Promise<FileNode[]>
  shared: boolean
}

const pendingListDirectory = new Map<string, PendingListDirectory>()

function cloneFileNodes(nodes: FileNode[]): FileNode[] {
  return nodes.map((node) => ({
    ...node,
    children: node.children ? cloneFileNodes(node.children) : node.children,
  }))
}

export async function listDirectory(
  path: string,
  includeHiddenOrOptions: boolean | ListDirectoryOptions = false,
): Promise<FileNode[]> {
  const options =
    typeof includeHiddenOrOptions === "boolean"
      ? { includeHidden: includeHiddenOrOptions }
      : includeHiddenOrOptions
  const includeHidden = options.includeHidden ?? false
  const maxDepth = options.maxDepth
  const requestKey = JSON.stringify([path, includeHidden, maxDepth ?? null])
  const pending = pendingListDirectory.get(requestKey)
  if (pending) {
    pending.shared = true
    return pending.request.then(cloneFileNodes)
  }

  const request = invoke<FileNode[]>("list_directory", {
    path,
    includeHidden,
    maxDepth,
  }).finally(() => {
    pendingListDirectory.delete(requestKey)
  })
  const entry: PendingListDirectory = { request, shared: false }
  pendingListDirectory.set(requestKey, entry)
  return request.then((nodes) => (entry.shared ? cloneFileNodes(nodes) : nodes))
}

export async function deleteFile(path: string): Promise<void> {
  return invoke("delete_file", { path })
}

export async function createDirectory(path: string): Promise<void> {
  assertAbsoluteFsPath("createDirectory", path)
  return invoke<void>("create_directory", { path })
}

export async function getFileSize(path: string): Promise<number> {
  return invoke<number>("get_file_size", { path })
}

function assertAbsoluteFsPath(operation: string, path: string): void {
  if (!isAbsolutePath(path)) {
    throw new Error(`${operation} requires an absolute path: ${path}`)
  }
}

/** Mirror of `commands::fs::FileBase64` (Rust side). */
export interface FileBase64 {
  base64: string
  mimeType: string
}

/**
 * Read any file off disk as base64 + a guessed mime type. The
 * vision-caption pipeline uses this to pick up extracted images
 * without having to read them as UTF-8 strings (PNG bytes aren't
 * valid UTF-8 — `readFile` would corrupt them).
 */
export async function readFileAsBase64(path: string): Promise<FileBase64> {
  return invoke<FileBase64>("read_file_as_base64", { path })
}
