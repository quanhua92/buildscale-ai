/**
 * Converts raw file content into a displayable string.
 * Handles both string content and content wrapped in a standard { text: string } object.
 */
export function getContentAsString(content: any, fileType?: string): string {
  if (!content) return ''
  
  // If it's a document, we primarily want the text field
  if (fileType === 'document') {
    if (content.text !== undefined) {
      return String(content.text)
    }
    // Fallback if document structure is corrupted but we want to show something
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2)
  }
  
  // For specialized types (canvas, chat, etc.), we show the raw JSON structure
  try {
    return JSON.stringify(content, null, 2)
  } catch (e) {
    return String(content)
  }
}
