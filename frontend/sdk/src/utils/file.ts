/**
 * File utilities for sorting and comparing file entries
 */

interface FileLike {
  file_type: string
  name: string
  display_name?: string
}

/**
 * Sort comparator for file entries: folders first, then alphabetically by display name.
 * Falls back to name (slug) if display_name is not available.
 *
 * @example
 * ```typescript
 * entries.sort(sortFileEntries)
 * ```
 */
export function sortFileEntries<T extends FileLike>(a: T, b: T): number {
  if (a.file_type === 'folder' && b.file_type !== 'folder') return -1
  if (a.file_type !== 'folder' && b.file_type === 'folder') return 1
  const nameA = a.display_name || a.name
  const nameB = b.display_name || b.name
  return nameA.localeCompare(nameB)
}
