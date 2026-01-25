import * as React from "react"
import { cn } from "src/utils"
import { useChatMessage } from "./chat-message"

export interface ChatBubbleProps extends React.HTMLAttributes<HTMLDivElement> {}

const ChatBubble = React.forwardRef<HTMLDivElement, ChatBubbleProps>(
  ({ className, children, ...props }, ref) => {
    const { message } = useChatMessage()
    const { role } = message

    return (
      <div
        ref={ref}
        className={cn(
          "relative text-sm leading-relaxed whitespace-pre-wrap break-words",
          role === "user" 
            ? "bg-primary text-primary-foreground px-4 py-2.5 rounded-2xl rounded-tr-sm shadow-sm" 
            : "text-foreground py-1",
          className
        )}
        {...props}
      >
        {children || message.content}
        {message.status === "streaming" && !message.content && (
          <span className="inline-block w-1 h-4 bg-primary animate-pulse ml-1 align-middle" />
        )}
      </div>
    )
  }
)
ChatBubble.displayName = "ChatBubble"

export { ChatBubble }
