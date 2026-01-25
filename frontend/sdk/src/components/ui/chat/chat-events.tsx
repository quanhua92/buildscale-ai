import * as React from "react"
import { Terminal, Box, CheckCircle2 } from "lucide-react"
import { cn } from "src/utils"
import { useChatMessage } from "./chat-message"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../collapsible"

export interface ChatEventsProps extends React.HTMLAttributes<HTMLDivElement> {}

function formatToolArgs(_tool: string, args: any) {
  if (!args) return null
  
  try {
    // 1. Identify primary paths or values
    const primary = args.path || args.source || args.pattern || "";
    
    // 2. Identify remaining args to show in brackets
    const entries = Object.entries(args)
      .filter(([key]) => !["path", "source", "pattern"].includes(key))
      .map(([key, val]) => {
        const displayVal = typeof val === 'string' ? val : JSON.stringify(val);
        return `${key}=${displayVal}`;
      });

    return (
      <div className="flex items-center gap-1.5 overflow-hidden flex-wrap text-foreground">
        {primary && (
          <span className="truncate font-medium opacity-90" title={primary}>
            {primary}
          </span>
        )}
        {entries.length > 0 && (
          <span className="text-[10px] opacity-50 font-mono truncate">
            [{entries.join(", ")}]
          </span>
        )}
      </div>
    )
  } catch (e) {
    return <span className="truncate opacity-70 text-destructive text-[10px]">Format Error</span>
  }
}

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
            <div key={stepKey} className="flex flex-col gap-1.5 animate-in fade-in slide-in-from-left-2 duration-300">
              {step.type === "call" ? (
                <div className="flex items-center gap-3 text-[11px] font-mono bg-muted/20 border border-muted/50 rounded-lg px-2.5 py-2 text-muted-foreground group hover:border-primary/30 transition-all shadow-sm">
                  <Terminal className="size-3.5 shrink-0 text-primary opacity-70" />
                  <div className="flex flex-col gap-0.5 flex-1 min-w-0">
                    <span className="text-primary font-bold uppercase tracking-tighter text-[9px] opacity-80 leading-none mb-0.5">Tool Call</span>
                    <div className="flex items-center gap-2">
                      <span className="text-foreground font-bold shrink-0">{step.tool}</span>
                      <div className="flex-1 min-w-0">
                        {formatToolArgs(step.tool, step.args)}
                      </div>
                    </div>
                  </div>
                </div>
              ) : step.type === "observation" ? (
                <Collapsible className="w-full">
                  <CollapsibleTrigger asChild>
                    <button type="button" className="w-full flex items-center justify-between gap-2 text-[10px] uppercase tracking-widest font-bold text-muted-foreground/40 hover:text-primary transition-colors px-2 py-1 group">
                      <div className="flex items-center gap-1.5">
                        <CheckCircle2 className="size-3 text-green-500/50 group-hover:text-green-500" />
                        <span>Tool Observation</span>
                      </div>
                      <Box className="size-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                    </button>
                  </CollapsibleTrigger>
                  <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
                    <pre className="mt-1 p-2.5 rounded-lg bg-black/5 text-[11px] font-mono overflow-x-auto border-l-2 border-primary/30 max-h-60 overflow-y-auto leading-relaxed shadow-inner">
                      {step.output || "No output returned from tool."}
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
