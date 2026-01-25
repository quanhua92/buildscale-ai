import * as React from "react"
import { Brain, ChevronDown, ChevronUp } from "lucide-react"
import { cn } from "src/utils"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../collapsible"
import { useChatMessage } from "./chat-message"

export interface ChatThoughtProps extends React.HTMLAttributes<HTMLDivElement> {
  defaultExpanded?: boolean
}

const ChatThought = React.forwardRef<HTMLDivElement, ChatThoughtProps>(
  ({ className, defaultExpanded = true, ...props }, ref) => {
    const { message } = useChatMessage()
    const [isOpen, setIsOpen] = React.useState(defaultExpanded)

    if (!message.thinking) return null

    return (
      <div ref={ref} className={cn("w-full", className)} {...props}>
        <Collapsible
          open={isOpen}
          onOpenChange={setIsOpen}
          className="group space-y-2"
        >
          <CollapsibleTrigger asChild>
            <button type="button" className="flex items-center gap-2 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">
              <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-muted/50 border border-transparent group-hover:border-border transition-all">
                <Brain className={cn("size-3.5", message.status === "streaming" && "animate-pulse text-primary")} />
                <span>Thinking...</span>
                {isOpen ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
              </div>
            </button>
          </CollapsibleTrigger>
          <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
            <div className="text-sm text-muted-foreground/80 italic pl-4 border-l-2 border-muted/30 py-1 leading-relaxed">
              {message.thinking}
            </div>
          </CollapsibleContent>
        </Collapsible>
      </div>
    )
  }
)
ChatThought.displayName = "ChatThought"

export { ChatThought }
