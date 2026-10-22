import { useCallback, useEffect, useRef, useState } from "react"
import { onDocbaseChanged } from "@/commands/docbase"

export interface DocbaseQuery<T> {
  data: T | null
  error: string | null
  loading: boolean
  reload: () => void
}

/**
 * Runs a document base query and runs it again whenever the base says
 * something changed. `key` identifies the query: a new key starts over.
 */
export function useDocbaseQuery<T>(key: string, run: () => Promise<T>): DocbaseQuery<T> {
  const [data, setData] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const runRef = useRef(run)
  useEffect(() => {
    runRef.current = run
  })
  // Only the answer to the latest request may land: a slow sweep-time
  // query must not overwrite a newer one.
  const latest = useRef(0)

  const reload = useCallback(() => {
    const request = ++latest.current
    runRef
      .current()
      .then((result) => {
        if (request !== latest.current) return
        setData(result)
        setError(null)
      })
      .catch((cause: unknown) => {
        if (request !== latest.current) return
        setError(typeof cause === "string" ? cause : String((cause as Error)?.message ?? cause))
      })
      .finally(() => {
        if (request === latest.current) setLoading(false)
      })
  }, [])

  useEffect(() => {
    setLoading(true)
    reload()
    const subscription = onDocbaseChanged(reload)
    return () => {
      latest.current++
      stopListening(subscription)
    }
  }, [key, reload])

  return { data, error, loading, reload }
}

/**
 * Stops listening to an event of the app once the subscription is in
 * place. Tauri answers `listen` before the page has registered the
 * listener, so a stop that comes right away (a screen that mounts and
 * unmounts at once, as React does twice in development) fails with
 * «Cannot read properties of undefined (reading 'handlerId')» and leaves
 * the listener behind: it is tried again a moment later, a few times.
 */
export function stopListening(subscription: Promise<() => unknown>, tries = 5, pause = 50): void {
  subscription.then(
    (stop) => {
      const attempt = (left: number) => {
        Promise.resolve()
          .then(stop)
          .catch(() => {
            if (left > 1) setTimeout(() => attempt(left - 1), pause)
          })
      }
      attempt(tries)
    },
    // It never listened: there is nothing to stop.
    () => {},
  )
}
