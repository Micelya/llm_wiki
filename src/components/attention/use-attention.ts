import { useMemo } from "react"
import { useAttentionStore } from "@/stores/attention-store"
import { attentionCounts, orderNotices, type AttentionCounts } from "@/lib/attention/order"
import type { AttentionNotice } from "@/lib/attention/types"
import { ATTENTION_SOURCES } from "./sources"

/** Every notice raised now, in the order of the view, with the counters. */
export function useAttention(): { notices: AttentionNotice[]; counts: AttentionCounts } {
  const bySource = useAttentionStore((s) => s.bySource)
  return useMemo(() => {
    const known = ATTENTION_SOURCES.map((source) => source.id)
    const others = Object.keys(bySource).filter((id) => !known.includes(id))
    const notices = orderNotices([...known, ...others].map((id) => bySource[id] ?? []))
    return { notices, counts: attentionCounts(notices) }
  }, [bySource])
}
