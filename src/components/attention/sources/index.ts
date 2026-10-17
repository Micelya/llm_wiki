import { vocabulary } from "@/vocabulary"
import type { AttentionNotice, AttentionSourceInfo } from "@/lib/attention/types"
import { DOCBASE_SOURCE, useDocbaseNotices } from "./docbase"
import { FILE_SYNC_SOURCE, useFileSyncNotices } from "./file-sync"
import { KNOWLEDGE_SOURCE, useKnowledgeNotices } from "./knowledge"
import { QUEUE_SOURCE, useQueueNotices } from "./wiki-queue"
import {
  ARCHIVE_WIKI_SOURCE,
  LINT_SOURCE,
  REVIEW_SOURCE,
  useArchiveWikiNotices,
  useLintNotices,
  useReviewNotices,
} from "./wiki"

/**
 * A part of the app that raises notices for the attention view.
 *
 * `useNotices` is a React hook: it reads whatever the part keeps (a
 * store, a query to the base, the wiki's queue) and returns its current
 * notices, or `null` while it does not know yet. It runs once, in its own
 * component (`AttentionSources`), however many screens show the result,
 * and whatever it returns replaces what it returned before: a notice
 * that is no longer returned is gone. A hook that throws shows as a
 * problem of that source, without taking the others down.
 */
export interface AttentionSource extends AttentionSourceInfo {
  useNotices: () => AttentionNotice[] | null
}

/**
 * Every source, in the order their lines go inside each class and
 * priority. A new part of the app adds its entry here.
 */
export const ATTENTION_SOURCES: AttentionSource[] = [
  { id: DOCBASE_SOURCE, label: "la base documental", useNotices: useDocbaseNotices },
  { id: KNOWLEDGE_SOURCE, label: `el registro de ${vocabulary().knowledge.entities.toLowerCase()}`, useNotices: useKnowledgeNotices },
  { id: QUEUE_SOURCE, label: "la cola del wiki", useNotices: useQueueNotices },
  { id: FILE_SYNC_SOURCE, label: "el registro de cambios de los archivos del wiki", useNotices: useFileSyncNotices },
  { id: REVIEW_SOURCE, label: "la revisión del wiki", useNotices: useReviewNotices },
  { id: LINT_SOURCE, label: "la revisión de enlaces del wiki", useNotices: useLintNotices },
  { id: ARCHIVE_WIKI_SOURCE, label: "el wiki del archivo", useNotices: useArchiveWikiNotices },
]
