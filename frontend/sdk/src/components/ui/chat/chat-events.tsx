import * as React from "react"
import { Terminal, Box, CheckCircle2, CircleX, Loader2 } from "lucide-react"
import { cn } from "src/utils"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "../collapsible"

export interface ChatEventsProps extends React.HTMLAttributes<HTMLDivElement> {
  call: { tool: string; args: any; id: string }
  observation?: { output: string; success: boolean }
}

function formatToolArgs(_tool: string, args: any) {
  if (!args) return null
  
  try {
    const primary = args.path || args.source || args.pattern || "";
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
  ({ className, call, observation, ...props }, ref) => {
    const [isOpen, setIsOpen] = React.useState(false)
    const isPending = !observation
    const isSuccess = observation?.success

    return (
      <div
        ref={ref}
        className={cn("w-full space-y-1.5 animate-in fade-in slide-in-from-top-1 duration-300", className)}
        {...props}
      >
        <div className={cn(
          "flex items-center gap-3 text-[11px] font-mono bg-muted/20 border rounded-lg px-2.5 py-2 text-muted-foreground group transition-all shadow-sm",
          isPending ? "border-primary/20 animate-pulse" : isSuccess ? "border-muted/50 hover:border-primary/30" : "border-destructive/30 bg-destructive/5"
        )}>
          {isPending ? (
            <Loader2 className="size-3.5 shrink-0 text-primary animate-spin" />
          ) : (
            <Terminal className={cn("size-3.5 shrink-0", isSuccess ? "text-primary opacity-70" : "text-destructive")} />
          )}
          
          <div className="flex flex-col gap-0.5 flex-1 min-w-0">
            <span className={cn(
              "font-bold uppercase tracking-tighter text-[9px] opacity-80 leading-none mb-0.5",
              !isPending && !isSuccess && "text-destructive"
            )}>
              {isPending ? "Executing..." : isSuccess ? "Tool Call" : "Tool Failed"}
            </span>
            <div className="flex items-center gap-2">
              <span className="text-foreground font-bold shrink-0">{call.tool}</span>
              <div className="flex-1 min-w-0">
                {formatToolArgs(call.tool, call.args)}
              </div>
            </div>
          </div>
        </div>

        {observation && (
          <Collapsible open={isOpen} onOpenChange={setIsOpen} className="w-full">
            <CollapsibleTrigger asChild>
              <button type="button" className="w-full flex items-center justify-between gap-2 text-[10px] uppercase tracking-widest font-bold text-muted-foreground/40 hover:text-primary transition-colors px-2 py-1 group">
                <div className="flex items-center gap-1.5">
                  {isSuccess ? (
                    <CheckCircle2 className="size-3 text-green-500/50 group-hover:text-green-500" />
                  ) : (
                    <CircleX className="size-3 text-destructive/50 group-hover:text-destructive" />
                  )}
                  <span>Output</span>
                </div>
                <Box className="size-3 opacity-0 group-hover:opacity-100 transition-opacity" />
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent className="overflow-hidden data-[state=closed]:animate-collapsible-up data-[state=open]:animate-collapsible-down">
              <pre className={cn(
                "mt-1 p-2.5 rounded-lg text-[11px] font-mono overflow-x-auto border-l-2 max-h-60 overflow-y-auto leading-relaxed shadow-inner",
                isSuccess ? "bg-black/5 border-primary/30" : "bg-destructive/5 border-destructive/50 text-destructive/90"
              )}>
                {observation.output || "No output returned."}
              </pre>
            </CollapsibleContent>
          </Collapsible>
        )}
      </div>
    )
  }
)
ChatEvents.displayName = "ChatEvents"

export { ChatEvents }
