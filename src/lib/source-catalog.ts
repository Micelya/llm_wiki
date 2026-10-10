/**
 * Ingest-side use of the source catalog (Rust: `source_volume/catalog.rs`).
 *
 * The catalog tells one document from the files that hold it. When a
 * source turns out to hold a document that was already ingested from
 * another path or in another format, it is not sent to the model again:
 * the source is added to the `sources` of the pages that document
 * produced, so those pages now answer for both files and survive the
 * deletion of either.
 *
 * Only an exact match counts. Similar documents are different
 * documents here.
 *
 * There are two checks, the cheap one first:
 *
 * - Same bytes (`leaveOutIdenticalFiles`), before a source is queued.
 *   It needs no text, so an identical scan or photo is recognized
 *   without the recognition engine reading it.
 * - Same normalized text (`adoptCataloguedContent`), inside the ingest
 *   task, right after the text is read and before any model call. This
 *   catches the same document in another format, or scanned twice.
 *
 * The catalog is an optimisation: any failure in it is logged and the
 * source is ingested the ordinary way.
 */
import { invoke } from "@tauri-apps/api/core"
import { fileExists, readFile, writeFile } from "@/commands/fs"
import { copyIngestCacheEntry, saveIngestCache } from "@/lib/ingest-cache"
import { withProjectLock } from "@/lib/project-mutex"
import { sourceIdentityForPath } from "@/lib/source-identity"
import { useActivityStore } from "@/stores/activity-store"
import { isAbsolutePath, normalizePath } from "@/lib/path-utils"
import { parseSources, writeSources } from "@/lib/sources-merge"

interface CatalogContent {
  id: string
  recognized: boolean
  wordCount: number
  locations: string[]
  ingested?: { identity: string; files: string[] }
}

export interface AdoptedContent {
  /** Wiki pages the source now shares, relative to the project. */
  files: string[]
  /** Source the document was ingested from. */
  original: string
  /** The match involved machine-read text, so it is less certain. */
  recognized: boolean
}

export interface AdoptionRequest {
  projectPath: string
  sourcePath: string
  identity: string
  text: string
  /** Runs page writes under the project's commit lock. */
  commit: <T>(operation: () => Promise<T>) => Promise<T>
}

interface Claim {
  holder: string
  done: Promise<void>
  release: () => void
}

/** Contents being ingested right now, so a second copy waits instead of racing. */
const claims = new Map<string, Claim>()

function claimKey(projectPath: string, contentId: string): string {
  return `${projectPath}\0${contentId}`
}

/** Recorded pages are project-relative; older records may be absolute. */
function pagePath(projectPath: string, file: string): string {
  return isAbsolutePath(file) ? normalizePath(file) : `${projectPath}/${file}`
}

function sameIdentity(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase()
}

async function register(request: AdoptionRequest): Promise<CatalogContent | null> {
  const content = await invoke<CatalogContent | null>("catalog_register_source", {
    projectPath: request.projectPath,
    identity: request.identity,
    text: request.text,
  })
  return content ?? null
}

async function allExist(projectPath: string, files: readonly string[]): Promise<boolean> {
  if (files.length === 0) return false
  for (const file of files) {
    if (!(await fileExists(pagePath(projectPath, file)))) return false
  }
  return true
}

async function addSourceToPages(
  projectPath: string,
  files: readonly string[],
  original: string,
  added: string,
): Promise<void> {
  for (const file of files) {
    if (!file.toLowerCase().endsWith(".md")) continue
    const path = pagePath(projectPath, file)
    const page = await readFile(path)
    const sources = parseSources(page)
    const listsOriginal = sources.some((source) => sameIdentity(source, original))
    const listsAdded = sources.some((source) => sameIdentity(source, added))
    if (!listsOriginal || listsAdded) continue
    await writeFile(path, writeSources(page, [...sources, added]))
  }
}

/**
 * If `request.identity` holds a document that was already ingested from
 * another source, attach it to that document's pages and return them.
 * Otherwise return null, and the caller ingests the source; it must call
 * `releaseCataloguedContent` when that ingest ends, however it ends.
 */
export async function adoptCataloguedContent(request: AdoptionRequest): Promise<AdoptedContent | null> {
  try {
    let content = await register(request)
    if (!content) return null
    const key = claimKey(request.projectPath, content.id)

    let running = claims.get(key)
    while (running && running.holder !== request.sourcePath) {
      await running.done
      // The other ingest may have produced the pages to share.
      content = await register(request)
      if (!content) return null
      running = claims.get(key)
    }

    const ingested = content.ingested
    if (
      ingested
      && !sameIdentity(ingested.identity, request.identity)
      && await allExist(request.projectPath, ingested.files)
    ) {
      await request.commit(async () => {
        await addSourceToPages(request.projectPath, ingested.files, ingested.identity, request.identity)
        await saveIngestCache(request.projectPath, request.identity, request.text, ingested.files)
      })
      return { files: ingested.files, original: ingested.identity, recognized: content.recognized }
    }

    let release = () => {}
    const done = new Promise<void>((resolve) => {
      release = resolve
    })
    claims.set(key, { holder: request.sourcePath, done, release })
    return null
  } catch (err) {
    console.warn(
      `[source-catalog] duplicate check failed for "${request.identity}"; ingesting normally:`,
      err instanceof Error ? err.message : err,
    )
    return null
  }
}

/**
 * The entries of `files` that still have to be ingested. A source whose
 * file is byte for byte the file of a document already ingested is
 * attached to that document's pages and left out; the activity panel
 * says so.
 */
export async function leaveOutIdenticalFiles<T extends { sourcePath: string }>(
  projectPath: string,
  files: readonly T[],
): Promise<T[]> {
  const pp = normalizePath(projectPath)
  const pending: T[] = []
  for (const file of files) {
    const identity = sourceIdentityForPath(pp, file.sourcePath)
    try {
      const content = await invoke<CatalogContent | null>("catalog_match_file", {
        projectPath: pp,
        identity,
      })
      const ingested = content?.ingested
      if (
        !content
        || !ingested
        || sameIdentity(ingested.identity, identity)
        || !(await allExist(pp, ingested.files))
      ) {
        pending.push(file)
        continue
      }
      await withProjectLock(pp, async () => {
        await addSourceToPages(pp, ingested.files, ingested.identity, identity)
        await copyIngestCacheEntry(pp, ingested.identity, identity)
      })
      useActivityStore.getState().addItem({
        type: "ingest",
        title: identity.split("/").pop() ?? identity,
        status: "done",
        detail: `Skipped (same file as ${ingested.identity}) — ${ingested.files.length} files shared`,
        filesWritten: ingested.files,
      })
    } catch (err) {
      console.warn(
        `[source-catalog] file check failed for "${identity}"; queueing it normally:`,
        err instanceof Error ? err.message : err,
      )
      pending.push(file)
    }
  }
  return pending
}

/** Let sources waiting on the ingest of `sourcePath` go on. Safe to call always. */
export function releaseCataloguedContent(projectPath: string, sourcePath: string): void {
  for (const [key, claim] of claims) {
    if (claim.holder === sourcePath && key.startsWith(`${projectPath}\0`)) {
      claims.delete(key)
      claim.release()
    }
  }
}

/** Remember the pages an ingest wrote, for later copies of the same document. */
export async function recordCataloguedIngest(
  projectPath: string,
  identity: string,
  files: readonly string[],
): Promise<void> {
  try {
    await invoke("catalog_record_ingest", { projectPath, identity, files: [...files] })
  } catch (err) {
    console.warn(
      `[source-catalog] could not record the ingest of "${identity}":`,
      err instanceof Error ? err.message : err,
    )
  }
}

export function describeAdoption(adopted: AdoptedContent): string {
  const basis = adopted.recognized ? " (matched on recognized text)" : ""
  return `Skipped (same document as ${adopted.original})${basis} — ${adopted.files.length} files shared`
}
