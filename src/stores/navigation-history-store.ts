import { create } from "zustand"
import type { DetailTab } from "@/components/knowledge/detail-tabs"
import { useDocbaseStore } from "./docbase-store"
import { useKnowledgeStore } from "./knowledge-store"
import { useWikiStore } from "./wiki-store"

export type View = ReturnType<typeof useWikiStore.getState>["activeView"]

/**
 * One step of the way the person went: the screen, what was open in it
 * (a person or organization, a matter, a document; none on a list or on
 * any other screen) and the tab she chose there.
 */
export interface Place {
  view: View
  /** The entity id, the matter id or the document's location id; null on a list. */
  subject: string | number | null
  tab: DetailTab | null
}

/** What a step is, without its tab: what a detail screen asks for its tab. */
export type PlaceRef = Pick<Place, "view" | "subject">

/**
 * Where a step leads now: the same place, the place what was open there
 * became (an entity joined with another), or null when it is gone and the
 * history skips it.
 */
export type PlaceResolver = (place: Place) => Place | null | Promise<Place | null>

/** How many steps the history keeps; the oldest go first. */
export const HISTORY_LIMIT = 100

interface NavigationHistoryState {
  entries: Place[]
  /** The step the person is on; -1 before the first. */
  index: number
  /** The tab a detail screen opens on when the history goes back or forward to it. */
  restoredTab: { view: View; subject: string | number; tab: DetailTab } | null
  /** Records where the person is now as a new step, unless she is already there. */
  record: (place: PlaceRef) => void
  /** Remembers the tab chosen on the step the person is on. */
  noteTab: (place: PlaceRef, tab: DetailTab) => void
  back: () => Promise<void>
  forward: () => Promise<void>
  /** Forgets every step (a test, another archive). */
  reset: () => void
}

export function samePlace(a: PlaceRef | undefined, b: PlaceRef | undefined): boolean {
  return a !== undefined && b !== undefined && a.view === b.view && a.subject === b.subject
}

/** What is open now, read from the stores of the three screens that open something. */
export function currentPlace(): PlaceRef {
  const view = useWikiStore.getState().activeView
  let subject: string | number | null = null
  if (view === "entities") subject = useKnowledgeStore.getState().entityId
  else if (view === "matters") subject = useKnowledgeStore.getState().matterId
  else if (view === "documents") subject = useDocbaseStore.getState().openLocationId
  return { view, subject }
}

/** Opens a step: its screen and what was open in it (nothing: its list). */
function applyPlace(place: Place) {
  if (place.view === "entities") useKnowledgeStore.getState().openEntity(place.subject === null ? null : String(place.subject))
  else if (place.view === "matters") useKnowledgeStore.getState().openMatter(place.subject === null ? null : String(place.subject))
  else if (place.view === "documents") {
    if (place.subject === null) useDocbaseStore.getState().closeDocument()
    else useDocbaseStore.getState().openDocument(Number(place.subject))
  }
  useWikiStore.getState().setActiveView(place.view)
}

let resolver: PlaceResolver = (place) => place
/** Every step leads where it was until the app says how to check them. */
export function setPlaceResolver(next: PlaceResolver | null) {
  resolver = next ?? ((place) => place)
}

/** Counts the changes to the steps, so a slow check does not undo a newer step. */
let generation = 0
let moving = false

export const useNavigationHistoryStore = create<NavigationHistoryState>((set, get) => {
  async function go(direction: -1 | 1) {
    if (moving) return
    moving = true
    const started = generation
    try {
      let { entries, index } = get()
      let i = index + direction
      while (i >= 0 && i < entries.length) {
        const target = await resolver(entries[i])
        if (generation !== started) return
        if (target === null || samePlace(target, entries[index])) {
          // Gone, or it became the step she is on: skipped, and forgotten.
          entries = entries.filter((_, at) => at !== i)
          if (i < index) index--
          if (direction < 0) i--
          continue
        }
        const step: Place = { view: target.view, subject: target.subject, tab: entries[i].tab }
        entries = entries.map((entry, at) => (at === i ? step : entry))
        generation++
        set({
          entries,
          index: i,
          restoredTab: step.subject !== null && step.tab !== null ? { view: step.view, subject: step.subject, tab: step.tab } : null,
        })
        applyPlace(step)
        return
      }
      generation++
      set({ entries, index })
    } finally {
      moving = false
    }
  }

  return {
    entries: [],
    index: -1,
    restoredTab: null,
    record: (place) => {
      const { entries, index } = get()
      if (samePlace(place, entries[index])) return
      const kept = entries.slice(0, index + 1)
      kept.push({ view: place.view, subject: place.subject, tab: null })
      const trimmed = kept.length > HISTORY_LIMIT ? kept.slice(kept.length - HISTORY_LIMIT) : kept
      generation++
      set({ entries: trimmed, index: trimmed.length - 1, restoredTab: null })
    },
    noteTab: (place, tab) => {
      const { entries, index } = get()
      if (!samePlace(place, entries[index])) return
      // Chosen now, it is no longer the tab the history left for the screen.
      set({ entries: entries.map((entry, at) => (at === index ? { ...entry, tab } : entry)), restoredTab: null })
    },
    back: () => go(-1),
    forward: () => go(1),
    reset: () => {
      generation++
      set({ entries: [], index: -1, restoredTab: null })
    },
  }
})

/** The tab the history left for a detail screen that opens now, if it is the step it returned to. */
export function restoredTabFor(place: PlaceRef): DetailTab | null {
  const restored = useNavigationHistoryStore.getState().restoredTab
  return restored && samePlace(restored, place) ? restored.tab : null
}

/**
 * Starts recording the way: every change of screen or of what is open in
 * it becomes a step. The changes one click makes (open a person, then
 * show her screen) are one step: the place is read once they are all done.
 */
export function startNavigationHistory(): () => void {
  let pending = false
  const schedule = () => {
    if (pending) return
    pending = true
    queueMicrotask(() => {
      pending = false
      useNavigationHistoryStore.getState().record(currentPlace())
    })
  }
  const stops = [useWikiStore.subscribe(schedule), useKnowledgeStore.subscribe(schedule), useDocbaseStore.subscribe(schedule)]
  useNavigationHistoryStore.getState().record(currentPlace())
  return () => stops.forEach((stop) => stop())
}
