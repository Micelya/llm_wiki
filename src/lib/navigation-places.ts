import { getDocument, getEntity, getMatter } from "@/commands/docbase"
import type { Place } from "@/stores/navigation-history-store"

/** How many joins in a row the way back follows before it stops looking. */
const MAX_JOINS = 5

/**
 * Where a step of the history leads now. A person or organization joined
 * with another leads to the one that remained; one no document names any
 * more, a matter that no longer exists and a document gone from the archive
 * are skipped. When the check itself fails the step stays as it was: its
 * screen says what happened.
 */
export async function resolvePlace(place: Place): Promise<Place | null> {
  if (place.subject === null) return place
  try {
    if (place.view === "entities") {
      let id = String(place.subject)
      for (let joins = 0; joins < MAX_JOINS; joins++) {
        const entity = await getEntity(id)
        if (!entity) return null
        if (entity.state === "merged" && entity.mergedInto) {
          id = entity.mergedInto
          continue
        }
        if (entity.state === "retired") return null
        break
      }
      return { ...place, subject: id }
    }
    if (place.view === "matters") {
      const matter = await getMatter(String(place.subject))
      return matter && matter.state !== "retired" ? place : null
    }
    if (place.view === "documents") {
      return (await getDocument(Number(place.subject))) ? place : null
    }
    return place
  } catch (err) {
    console.warn("[navigation] could not check a step of the way back:", err)
    return place
  }
}
