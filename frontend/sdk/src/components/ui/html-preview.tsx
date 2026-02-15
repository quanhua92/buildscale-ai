import * as React from "react"
import { cn } from "src/utils"

/**
 * Check if a file is an HTML file based on its extension
 */
export const isHtmlFile = (filename: string): boolean => {
  const lower = filename.toLowerCase()
  return lower.endsWith('.html') || lower.endsWith('.htm')
}

export interface HtmlPreviewProps extends React.IframeHTMLAttributes<HTMLIFrameElement> {
  /** The HTML content to render in the iframe */
  content: string
  /** Title for the iframe (for accessibility) */
  title?: string
  /** Sandbox attribute value. Defaults to "allow-scripts" for security */
  sandbox?: string
  /** Minimum height of the iframe */
  minHeight?: string | number
}

/**
 * A sandboxed iframe for safely previewing HTML content.
 *
 * Uses `srcDoc` to pass content directly and `sandbox="allow-scripts"` by default
 * to allow JavaScript execution (needed for CDNs like Tailwind) while preventing
 * access to the parent page's DOM, cookies, and storage.
 *
 * @example
 * ```tsx
 * <HtmlPreview content="<html><body>Hello World</body></html>" />
 * ```
 */
export const HtmlPreview = React.forwardRef<HTMLIFrameElement, HtmlPreviewProps>(
  (
    {
      content,
      title = "HTML Preview",
      sandbox = "allow-scripts",
      minHeight = 400,
      className,
      style,
      ...props
    },
    ref
  ) => {
    const minHeightValue = typeof minHeight === 'number' ? `${minHeight}px` : minHeight

    return (
      <iframe
        ref={ref}
        srcDoc={content}
        sandbox={sandbox}
        className={cn("w-full h-full border-0 bg-white", className)}
        style={{ minHeight: minHeightValue, ...style }}
        title={title}
        {...props}
      />
    )
  }
)

HtmlPreview.displayName = "HtmlPreview"
