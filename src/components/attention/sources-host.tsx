import { Component, useEffect, type ReactNode } from "react"
import { useAttentionStore } from "@/stores/attention-store"
import type { AttentionNotice } from "@/lib/attention/types"
import { ATTENTION_SOURCES, type AttentionSource } from "./sources"

/**
 * Runs every source of the attention view once, for the whole app, and
 * keeps what each one raises in the attention store. Mounted with the
 * app's layout, so the counter and the view are right from any screen.
 */
export function AttentionSources() {
  return (
    <>
      {ATTENTION_SOURCES.map((source) => (
        <SourceBoundary key={source.id} source={source}>
          <SourceRunner source={source} />
        </SourceBoundary>
      ))}
    </>
  )
}

function SourceRunner({ source }: { source: AttentionSource }) {
  const notices = source.useNotices()
  const publish = useAttentionStore((s) => s.publish)
  const clear = useAttentionStore((s) => s.clear)
  useEffect(() => {
    if (notices) publish(source.id, notices)
  }, [notices, publish, source.id])
  useEffect(() => () => clear(source.id), [clear, source.id])
  return null
}

/** The line shown when a source fails: the others keep working. */
export function sourceFailure(source: AttentionSource, error: unknown): AttentionNotice {
  const message = error instanceof Error ? error.message : String(error)
  return {
    source: source.id,
    id: `${source.id}:failed`,
    kind: "problem",
    topic: "source-failed",
    priority: "normal",
    count: 1,
    title: `No se pudo saber qué avisa ${source.label}`,
    explanation: `Lo que ${source.label} tenga para avisar no se ve hasta que vuelva a responder. El error fue: ${message}`,
    todo: "Si sigue, reiniciá la app.",
    resolvedWhen: "Desaparece cuando vuelve a responder.",
    actions: [],
  }
}

class SourceBoundary extends Component<{ source: AttentionSource; children: ReactNode }, { failed: boolean }> {
  state = { failed: false }
  private retry: ReturnType<typeof setTimeout> | null = null

  static getDerivedStateFromError() {
    return { failed: true }
  }

  componentDidCatch(error: unknown) {
    console.error(`[attention] source ${this.props.source.id} failed:`, error)
    useAttentionStore.getState().publish(this.props.source.id, [sourceFailure(this.props.source, error)])
    // Try again after a while: most failures (the base still opening) pass.
    this.retry = setTimeout(() => this.setState({ failed: false }), 30_000)
  }

  componentWillUnmount() {
    if (this.retry) clearTimeout(this.retry)
  }

  render() {
    return this.state.failed ? null : this.props.children
  }
}
