import * as React from "react"
import { Terminal, Box, CheckCircle2 } from "lucide-react"
import { cn } from "src/utils"
import { useChatMessage } from "./chat-message"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../collapsible"

export interface ChatEventsProps extends React.HTMLAttributes<HTMLDivElement> {}

const ChatEvents = React.forwardRef<HTMLDivElement, ChatEventsProps>(
  ({ className, ...props }, ref) => {
    const { message } = useChatMessage()

    if (!message.steps || message.steps.length === 0) return null

    return (
      <div
        ref={ref}
        className={cn("w-full space-y-2 my-2", className)}
        {...props}
      >
        {message.steps.map((step, index) => {
          const stepKey = `${message.id}-step-${index}`
          return (
            <div key={stepKey} className="flex flex-col gap-1.5">
              {step.type === "call" ? (
                <div className="flex items-center gap-2 text-xs font-mono bg-muted/30 border rounded-md px-2 py-1.5 text-muted-foreground">
                  <Terminal className="size-3.5" />
                  <span className="text-primary font-semibold">{step.tool}</span>
                  <span className="truncate opacity-70">{step.path}</span>
                </div>
              ) : step.type === "observation" ? (
                <Collapsible className="w-full">
                  <CollapsibleTrigger asChild>
                    <button type="button" className="w-full flex items-center justify-between gap-2 text-[10px] uppercase tracking-wider font-bold text-muted-foreground/60 hover:text-muted-foreground transition-colors px-2">
                      <div className="flex items-center gap-1.5">
                        <CheckCircle2 className="size-3 text-green-500" />
                        <span>Observation</span>
                      </div>
                      <Box className="size-3" />
                    </button>
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <pre className="mt-1 p-2 rounded bg-black/5 text-xs font-mono overflow-x-auto border-l-2 border-primary/20 max-h-40 overflow-y-auto">
                      {step.output}
                    </pre>
                  </CollapsibleContent>
                </Collapsible>
              ) : null}
            </div>
          )
        })}
      </div>
    )
  }
)
ChatEvents.displayName = "ChatEvents"

export { ChatEvents }
