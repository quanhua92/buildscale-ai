import * as React from "react"
import { cn } from "src/utils"
import { useChatMessage } from "./chat-message"
import { ChatThought } from "./chat-thought"
import { ChatEvents } from "./chat-events"

export interface ChatBubbleProps extends React.HTMLAttributes<HTMLDivElement> {}

const ChatBubble = React.forwardRef<HTMLDivElement, ChatBubbleProps>(
  ({ className, children, ...props }, ref) => {
    const { message } = useChatMessage()
    const { role, parts } = message

    // User messages are simple bubbles
    if (role === "user") {
      return (
        <div
          ref={ref}
          className={cn(
            "relative text-sm leading-relaxed whitespace-pre-wrap break-words bg-primary text-primary-foreground px-4 py-2.5 rounded-2xl rounded-tr-sm shadow-sm",
            className
          )}
          {...props}
        >
          {children || parts.map(p => p.type === 'text' ? p.content : '').join('')}
        </div>
      )
    }

    // Assistant messages are interleaved sequences
    return (
      <div
        ref={ref}
        className={cn("w-full space-y-4", className)}
        {...props}
      >
        {parts.map((part, idx) => {
          const key = `${message.id}-part-${idx}`
          
          switch (part.type) {
            case "thought":
              return <ChatThought key={key} content={part.content} />
            
            case "call": {
              // Look for the next part if it's an observation
              const nextPart = parts[idx + 1]
              const observation = nextPart?.type === "observation" ? nextPart : undefined
              return (
                <ChatEvents 
                  key={key} 
                  call={part} 
                  observation={observation} 
                />
              )
            }
            
            case "observation":
              // Handled by the preceding "call" part
              return null
            
            case "text":
              return (
                <div 
                  key={key} 
                  className="text-sm leading-relaxed whitespace-pre-wrap break-words text-foreground py-1"
                >
                  {part.content}
                  {message.status === "streaming" && idx === parts.length - 1 && (
                    <span className="inline-block w-1 h-4 bg-primary animate-pulse ml-1 align-middle" />
                  )}
                </div>
              )
            
            default:
              return null
          }
        })}
        
        {/* If no parts yet but streaming, show cursor */}
        {parts.length === 0 && message.status === "streaming" && (
          <span className="inline-block w-1 h-4 bg-primary animate-pulse align-middle" />
        )}
      </div>
    )
  }
)
ChatBubble.displayName = "ChatBubble"

export { ChatBubble }
