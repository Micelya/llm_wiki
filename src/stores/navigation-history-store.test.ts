import { afterEach, beforeEach, describe, expect, it } from "vitest"
import { useDocbaseStore } from "./docbase-store"
import { useKnowledgeStore } from "./knowledge-store"
import {
  HISTORY_LIMIT,
  notePersonAction,
  restoredTabFor,
  setPlaceResolver,
  startNavigationHistory,
  useNavigationHistoryStore,
} from "./navigation-history-store"
import { useWikiStore } from "./wiki-store"

/** Lets the steps of one click be recorded, as they are after it. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))

const history = () => useNavigationHistoryStore.getState()

function where() {
  return {
    view: useWikiStore.getState().activeView,
    entity: useKnowledgeStore.getState().entityId,
    matter: useKnowledgeStore.getState().matterId,
    document: useDocbaseStore.getState().openLocationId,
  }
}

async function openEntity(id: string) {
  useKnowledgeStore.getState().openEntity(id)
  useWikiStore.getState().setActiveView("entities")
  await settle()
}

async function openMatter(id: string) {
  useKnowledgeStore.getState().openMatter(id)
  useWikiStore.getState().setActiveView("matters")
  await settle()
}

async function openDocument(locationId: number) {
  useDocbaseStore.getState().openDocument(locationId)
  useWikiStore.getState().setActiveView("documents")
  await settle()
}

let stop: () => void = () => {}

beforeEach(() => {
  useWikiStore.getState().setActiveView("home")
  useKnowledgeStore.getState().openEntity(null)
  useKnowledgeStore.getState().openMatter(null)
  useDocbaseStore.getState().closeDocument()
  setPlaceResolver(null)
  history().reset()
  stop = startNavigationHistory()
})

afterEach(() => {
  stop()
  setPlaceResolver(null)
})

describe("navigation history", () => {
  it("goes back and forward on the way the person went, one step per click", async () => {
    await openEntity("person-1")
    await openMatter("matter-1")
    await openDocument(7)
    // Home, the person, the matter, the document: the two changes of each click are one step.
    expect(history().entries).toHaveLength(4)
    expect(history().index).toBe(3)

    await history().back()
    expect(where()).toMatchObject({ view: "matters", matter: "matter-1" })
    await history().back()
    expect(where()).toMatchObject({ view: "entities", entity: "person-1" })
    await history().back()
    expect(where().view).toBe("home")
    expect(history().index).toBe(0)

    // Nowhere further back: nothing changes.
    await history().back()
    expect(where().view).toBe("home")
    expect(history().index).toBe(0)

    await history().forward()
    await history().forward()
    expect(where()).toMatchObject({ view: "matters", matter: "matter-1" })
    await history().forward()
    expect(where()).toMatchObject({ view: "documents", document: 7 })
    expect(history().index).toBe(3)

    // Going back and forward records no new steps.
    await settle()
    expect(history().entries).toHaveLength(4)
  })

  it("going back to a list opens the list, not the last thing open in it", async () => {
    useWikiStore.getState().setActiveView("entities")
    await settle()
    await openEntity("person-1")
    await history().back()
    expect(where()).toMatchObject({ view: "entities", entity: null })
  })

  it("a new step after going back forgets the steps ahead", async () => {
    await openEntity("person-1")
    await openMatter("matter-1")
    await openDocument(7)
    await history().back()
    await history().back()
    expect(history().index < history().entries.length - 1).toBe(true)

    await openDocument(9)
    expect(history().entries.map((entry) => entry.subject)).toEqual([null, "person-1", 9])
    expect(history().index).toBe(2)
    await history().forward()
    expect(where()).toMatchObject({ view: "documents", document: 9 })
  })

  it("skips what no longer exists and leads to where a joined entity remained", async () => {
    await openEntity("person-joined")
    await openEntity("person-gone")
    await openDocument(7)
    setPlaceResolver((place) => {
      if (place.subject === "person-gone") return null
      if (place.subject === "person-joined") return { ...place, subject: "person-kept" }
      return place
    })

    await history().back()
    expect(where()).toMatchObject({ view: "entities", entity: "person-kept" })
    // The step that is gone is forgotten; the joined one now says where it remained.
    expect(history().entries.map((entry) => entry.subject)).toEqual([null, "person-kept", 7])

    await history().forward()
    expect(where()).toMatchObject({ view: "documents", document: 7 })
  })

  it("forgets a step that became the one the person is on", async () => {
    await openEntity("person-kept")
    await openEntity("person-joined")
    await openEntity("person-kept")
    setPlaceResolver((place) => (place.subject === "person-joined" ? { ...place, subject: "person-kept" } : place))

    await history().back()
    expect(where().view).toBe("home")
    expect(history().entries.map((entry) => entry.subject)).toEqual([null, "person-kept"])
  })

  it("keeps a limited number of steps, the oldest going first", async () => {
    for (let id = 1; id <= HISTORY_LIMIT + 20; id++) await openDocument(id)
    expect(history().entries).toHaveLength(HISTORY_LIMIT)
    expect(history().index).toBe(HISTORY_LIMIT - 1)
    expect(history().entries[0].subject).toBe(21)
    expect(history().entries[HISTORY_LIMIT - 1].subject).toBe(HISTORY_LIMIT + 20)
  })

  it("remembers the tab chosen and opens the screen on it when going back", async () => {
    await openEntity("person-1")
    history().noteTab({ view: "entities", subject: "person-1" }, "data")
    // A tab of another screen does not touch this step.
    history().noteTab({ view: "matters", subject: "matter-1" }, "article")
    await openDocument(7)
    expect(restoredTabFor({ view: "entities", subject: "person-1" })).toBeNull()

    await history().back()
    expect(restoredTabFor({ view: "entities", subject: "person-1" })).toBe("data")
    expect(restoredTabFor({ view: "documents", subject: 7 })).toBeNull()

    // Going somewhere new forgets it: the next visit opens as always.
    await openMatter("matter-1")
    expect(restoredTabFor({ view: "entities", subject: "person-1" })).toBeNull()
  })
})

/**
 * A click on the menu, and then the screen it showed changing what is
 * open by itself, after the click (an effect, an answer that arrives
 * later): two changes outside one microtask (P-332).
 */
async function clickMenuThenScreenChangesItself() {
  notePersonAction()
  useWikiStore.getState().setActiveView("entities")
  await settle()
  useKnowledgeStore.getState().openEntity("person-open-by-itself")
  await settle()
  useKnowledgeStore.getState().openEntity(null)
  await settle()
}

describe("navigation history, steps by what the person does", () => {
  beforeEach(() => {
    stop()
    history().reset()
    stop = startNavigationHistory({ stepsByAction: true })
  })

  async function clickMatter(id: string) {
    notePersonAction()
    await openMatter(id)
  }

  it("records every change as a step without it: the two «Atrás» of P-332", async () => {
    stop()
    history().reset()
    stop = startNavigationHistory()
    await openMatter("matter-1")
    await clickMenuThenScreenChangesItself()
    await history().back()
    expect(where().view).toBe("entities")
  })

  it("one «Atrás» undoes one click, whatever the screen changes after it", async () => {
    await clickMatter("matter-1")
    await clickMenuThenScreenChangesItself()
    expect(history().entries.map((entry) => [entry.view, entry.subject])).toEqual([
      ["home", null],
      ["matters", "matter-1"],
      ["entities", null],
    ])

    await history().back()
    expect(where()).toMatchObject({ view: "matters", matter: "matter-1" })
    await history().forward()
    expect(where()).toMatchObject({ view: "entities", entity: null })
  })

  it("after going back, what the screen changes by itself keeps the steps ahead", async () => {
    await clickMatter("matter-1")
    notePersonAction()
    await openDocument(7)
    notePersonAction()
    await history().back()
    // The matter's screen opens something by itself once it is back.
    useKnowledgeStore.getState().openMatter("matter-2")
    await settle()
    expect(history().entries.map((entry) => entry.subject)).toEqual([null, "matter-2", 7])
    await history().forward()
    expect(where()).toMatchObject({ view: "documents", document: 7 })
  })

  it("a change that comes back to the step before merges into it", async () => {
    await clickMatter("matter-1")
    notePersonAction()
    useWikiStore.getState().setActiveView("entities")
    await settle()
    useWikiStore.getState().setActiveView("matters")
    await settle()
    expect(history().entries.map((entry) => entry.view)).toEqual(["home", "matters"])
  })

  it("a change the person causes later, after an answer arrives, is still a step", async () => {
    notePersonAction()
    // The click changes nothing yet; the place changes when the answer comes.
    await settle()
    await openMatter("matter-new")
    expect(history().entries.map((entry) => entry.subject)).toEqual([null, "matter-new"])
  })
})
