/**
 * Converts raw file content into a displayable string.
 * Handles both string content and content wrapped in a standard { text: string } object.
 */
export function getContentAsString(content: any): string {
  if (!content) return ''
  
  if (typeof content === 'string') {
    return content
  }
  
  if (content.text !== undefined) {
    return String(content.text)
  }
  
  // For specialized types that are already objects, stringify them for the textarea
  try {
    return JSON.stringify(content, null, 2)
  } catch (e) {
    return String(content)
  }
}
