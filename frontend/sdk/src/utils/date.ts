/**
 * Date formatting utilities
 */

// Format options to ensure consistency across the application
const DATE_OPTIONS: Intl.DateTimeFormatOptions = {
  dateStyle: 'medium',
}

const DATE_TIME_OPTIONS: Intl.DateTimeFormatOptions = {
  dateStyle: 'medium',
  timeStyle: 'short',
}

const TIME_OPTIONS: Intl.DateTimeFormatOptions = {
  timeStyle: 'short',
}

/**
 * Format a date string, number, or Date object to a localized date string
 * Example: "Jan 1, 2026"
 */
export function formatDate(date: string | number | Date | null | undefined): string {
  if (!date) return ''
  const d = new Date(date)
  if (isNaN(d.getTime())) return ''
  return d.toLocaleDateString(undefined, DATE_OPTIONS)
}

/**
 * Format a date string, number, or Date object to a localized date-time string
 * Example: "Jan 1, 2026, 12:00 PM"
 */
export function formatDateTime(date: string | number | Date | null | undefined): string {
  if (!date) return ''
  const d = new Date(date)
  if (isNaN(d.getTime())) return ''
  return d.toLocaleString(undefined, DATE_TIME_OPTIONS)
}

/**
 * Format a date string, number, or Date object to a localized time string
 * Example: "12:00 PM"
 */
export function formatTime(date: string | number | Date | null | undefined): string {
  if (!date) return ''
  const d = new Date(date)
  if (isNaN(d.getTime())) return ''
  return d.toLocaleTimeString(undefined, TIME_OPTIONS)
}
