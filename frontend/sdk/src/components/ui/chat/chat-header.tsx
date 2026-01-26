import * as React from "react"
import { Plus } from "lucide-react"
import { cn } from "src/utils"
import { Button } from "../button"

export interface ChatHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  modelName?: string
  onNewChat?: () => void
}

const ChatHeader = React.forwardRef<HTMLDivElement, ChatHeaderProps>(
  ({ className, modelName, onNewChat, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          "flex items-center justify-between py-2 px-4 border-b border-border/50 shrink-0",
          className
        )}
        {...props}
      >
        {/* Spacer for center alignment */}
        <div className="w-24" />

        {/* Center: Model Name */}
        <div className="flex-1 flex justify-center">
          {modelName && (
            <div className="text-xs font-mono text-muted-foreground bg-muted px-2.5 py-1 rounded">
              {modelName}
            </div>
          )}
        </div>

        {/* Right: New Chat Button */}
        <div className="w-24 flex justify-end">
          {onNewChat && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onNewChat}
              className="h-7 gap-1.5 text-xs"
            >
              <Plus className="size-3.5" />
              <span>New Chat</span>
            </Button>
          )}
        </div>
      </div>
    )
  }
)
ChatHeader.displayName = "ChatHeader"

export { ChatHeader }
