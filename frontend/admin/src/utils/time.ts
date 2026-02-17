/**
 * Format a timestamp as a relative time string (e.g., "5m ago", "2h ago", "yesterday")
 *
 * @param timestamp - ISO string timestamp or Date object
 * @returns Formatted relative time string
 */
export function formatTimeAgo(timestamp: string | Date): string {
  const now = new Date()
  const time = typeof timestamp === 'string' ? new Date(timestamp) : timestamp
  const diff = now.getTime() - time.getTime()

  const seconds = Math.floor(diff / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)

  if (seconds < 60) return 'just now'
  if (minutes < 60) return `${minutes}m ago`
  if (hours < 24) return `${hours}h ago`
  if (days === 1) return 'yesterday'
  if (days < 7) return `${days}d ago`
  return time.toLocaleDateString()
}
