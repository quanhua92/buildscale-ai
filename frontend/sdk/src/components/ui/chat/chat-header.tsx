import * as React from "react"
import { Plus } from "lucide-react"
import { cn } from "src/utils"
import { Button } from "../button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../select"
import type { ChatModel } from "./chat-context"
import { CHAT_MODELS } from "./chat-context"

export interface ChatHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  modelName?: string
  onNewChat?: () => void
  model?: ChatModel
  onModelChange?: (model: ChatModel) => void
}

const ChatHeader = React.forwardRef<HTMLDivElement, ChatHeaderProps>(
  ({ className, modelName, onNewChat, model, onModelChange, ...props }, ref) => {
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

        {/* Center: Model Selector */}
        <div className="flex-1 flex justify-center">
          {model && onModelChange ? (
            <Select value={model} onValueChange={onModelChange}>
              <SelectTrigger className="w-[180px] h-7 text-xs">
                <SelectValue placeholder="Select model" />
              </SelectTrigger>
              <SelectContent>
                {CHAT_MODELS.map((modelOption) => (
                  <SelectItem key={modelOption} value={modelOption} className="text-xs">
                    {modelOption}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : modelName ? (
            <div className="text-xs font-mono text-muted-foreground bg-muted px-2.5 py-1 rounded">
              {modelName}
            </div>
          ) : null}
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
