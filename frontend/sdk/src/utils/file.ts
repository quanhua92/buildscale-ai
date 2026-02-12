/**
 * File utilities for sorting and comparing file entries
 */

interface FileLike {
  file_type: string
  name: string
}

/**
 * Sort comparator for file entries: folders first, then alphabetically by name.
 *
 * @example
 * ```typescript
 * entries.sort(sortFileEntries)
 * ```
 */
export function sortFileEntries<T extends FileLike>(a: T, b: T): number {
  if (a.file_type === 'folder' && b.file_type !== 'folder') return -1
  if (a.file_type !== 'folder' && b.file_type === 'folder') return 1
  return a.name.localeCompare(b.name)
}
