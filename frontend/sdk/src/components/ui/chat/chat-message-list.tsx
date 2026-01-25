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

    React.useEffect(() => {
      const currentCount = React.Children.count(children)
      if (autoScroll && isAtBottom && currentCount > lastChildrenCount.current) {
        scrollToBottom()
      }
      lastChildrenCount.current = currentCount
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
          "flex-1 overflow-y-auto py-4 space-y-6 scrollbar-thin scrollbar-thumb-muted-foreground/20",
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
