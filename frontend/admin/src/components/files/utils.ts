/**
 * Converts raw file content into a displayable string.
 * All file content is displayed as-is (strings, numbers, or JSON stringified).
 */
export function getContentAsString(content: any): string {
  if (!content) return ''

  // All content: return as string (no special handling)
  // AI is responsible for formatting content in their desired format
  if (typeof content === 'string') {
    return content
  }
  if (typeof content === 'number' || typeof content === 'boolean') {
    return String(content)
  }

  // For objects/arrays, stringify them
  try {
    return JSON.stringify(content, null, 2)
  } catch (e) {
    return String(content)
  }
}
