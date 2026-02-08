import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "src/utils"
import { type ChatMessageItem } from "./chat-context"

const messageVariants = cva("flex w-full gap-4 px-1 py-2 transition-all", {
  variants: {
    variant: {
      user: "flex-row-reverse",
      assistant: "flex-row",
      system: "justify-center",
      tool: "flex-row",
    },
  },
  defaultVariants: {
    variant: "assistant",
  },
})

interface ChatMessageContextValue {
  message: ChatMessageItem
}

const ChatMessageContext = React.createContext<ChatMessageContextValue | null>(null)

export function useChatMessage() {
  const context = React.useContext(ChatMessageContext)
  if (!context) {
    throw new Error("useChatMessage must be used within a ChatMessage component")
  }
  return context
}

export interface ChatMessageProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof messageVariants> {
  message: ChatMessageItem
}

const ChatMessage = React.forwardRef<HTMLDivElement, ChatMessageProps>(
  ({ className, variant, message, children, ...props }, ref) => {
    const role = message.role
    return (
      <ChatMessageContext.Provider value={{ message }}>
        <div
          ref={ref}
          className={cn(messageVariants({ variant: variant || role, className }))}
          {...props}
        >
          {/* Avatar Placeholder */}
          <div className={cn(
            "h-8 w-8 rounded-full flex items-center justify-center text-xs font-medium shrink-0",
            role === "user" ? "bg-primary text-primary-foreground" :
            role === "tool" ? "bg-secondary text-secondary-foreground" :
            "bg-muted text-muted-foreground border"
          )}>
            {role === "user" ? "U" : role === "tool" ? "T" : "AI"}
          </div>
          
          <div className={cn(
            "flex flex-col gap-2 max-w-[85%] md:max-w-[75%]",
            role === "user" ? "items-end" : "items-start"
          )}>
            {children}
          </div>
        </div>
      </ChatMessageContext.Provider>
    )
  }
)
ChatMessage.displayName = "ChatMessage"

export { ChatMessage }
