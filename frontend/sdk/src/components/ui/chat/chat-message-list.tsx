import * as React from "react"
import { cn } from "src/utils"

export interface ChatMessageListProps extends React.HTMLAttributes<HTMLDivElement> {
  autoScroll?: boolean
}

const ChatMessageList = React.forwardRef<HTMLDivElement, ChatMessageListProps>(
  ({ className, children, autoScroll = true, ...props }, ref) => {
    const scrollRef = React.useRef<HTMLDivElement>(null)
    const [isAtBottom, setIsAtBottom] = React.useState(true)

    const scrollToBottom = React.useCallback(() => {
      if (scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight
      }
    }, [])

    const handleScroll = React.useCallback(() => {
      if (scrollRef.current) {
        const { scrollTop, scrollHeight, clientHeight } = scrollRef.current
        const atBottom = scrollHeight - scrollTop <= clientHeight + 100
        setIsAtBottom(atBottom)
      }
    }, [])

    const lastChildrenCount = React.useRef(React.Children.count(children))
    const lastChildrenString = React.useRef(JSON.stringify(children))

    React.useEffect(() => {
      const currentCount = React.Children.count(children)
      const currentString = JSON.stringify(children)

      // Scroll to bottom if:
      // 1. autoScroll enabled AND at bottom AND new message added
      // 2. OR children content changed (streaming updates)
      if (autoScroll && ((isAtBottom && currentCount > lastChildrenCount.current) || currentString !== lastChildrenString.current)) {
        scrollToBottom()
      }

      lastChildrenCount.current = currentCount
      lastChildrenString.current = currentString
    }, [children, autoScroll, isAtBottom, scrollToBottom])

    return (
      <div
        ref={(node) => {
          // Merge refs
          if (typeof ref === "function") ref(node)
          else if (ref) ref.current = node
          scrollRef.current = node
        }}
        onScroll={handleScroll}
        className={cn(
          "flex-1 min-h-0 overflow-y-auto py-4 space-y-6 scrollbar-thin scrollbar-thumb-muted-foreground/20",
          className
        )}
        {...props}
      >
        {children}
        <div className="h-4 w-full flex-shrink-0" /> {/* Bottom spacer */}
      </div>
    )
  }
)
ChatMessageList.displayName = "ChatMessageList"

export { ChatMessageList }
