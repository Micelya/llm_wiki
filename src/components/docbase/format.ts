/** Small formatting helpers shared by the document base screens. */

export function formatCount(value: number): string {
  return value.toLocaleString("es-AR")
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ["KB", "MB", "GB", "TB"]
  let value = bytes / 1024
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit++
  }
  return `${value.toLocaleString("es-AR", { maximumFractionDigits: value < 10 ? 1 : 0 })} ${units[unit]}`
}

export function formatDate(ms: number): string {
  return new Date(ms).toLocaleDateString("es-AR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  })
}

/** "hace 4 min", "hace 2 h", "ayer", or the date for anything older. */
export function formatAgo(ms: number, now: number = Date.now()): string {
  const seconds = Math.max(0, Math.round((now - ms) / 1000))
  if (seconds < 60) return "hace un momento"
  const minutes = Math.round(seconds / 60)
  if (minutes < 60) return `hace ${minutes} min`
  const hours = Math.round(minutes / 60)
  if (hours < 24) return `hace ${hours} h`
  const days = Math.round(hours / 24)
  if (days === 1) return "ayer"
  if (days < 7) return `hace ${days} días`
  return formatDate(ms)
}

/** "1 archivo" / "3 archivos". */
export function plural(count: number, one: string, many: string): string {
  return `${formatCount(count)} ${count === 1 ? one : many}`
}

/** Turns what the user typed ("cliente, caso" or "cliente / caso") into levels. */
export function parseConvention(text: string): string[] {
  return text
    .split(/[,/>\n]/)
    .map((level) => level.trim())
    .filter((level) => level.length > 0)
}
