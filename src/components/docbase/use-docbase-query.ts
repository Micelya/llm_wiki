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
    let disposed = false
    let unlisten: (() => void) | null = null
    onDocbaseChanged(reload)
      .then((stop) => {
        if (disposed) stop()
        else unlisten = stop
      })
      .catch(() => {})
    return () => {
      disposed = true
      latest.current++
      unlisten?.()
    }
  }, [key, reload])

  return { data, error, loading, reload }
}
