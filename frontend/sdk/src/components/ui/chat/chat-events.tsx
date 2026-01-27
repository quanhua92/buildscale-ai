import * as React from "react"
import { Terminal, CheckCircle2, CircleX, Loader2, ChevronDown } from "lucide-react"
import { cn } from "src/utils"

export interface ChatEventsProps extends React.HTMLAttributes<HTMLDivElement> {
  call: { tool: string; args: any; id: string }
  observation?: { output: string; success: boolean }
}

/**
 * Attempts to parse and pretty-print JSON output.
 * Only formats if output starts with '{' or '[' to avoid false positives.
 */
function formatOutput(output: string): string {
  if (!output) return output
  const trimmed = output.trim()
  // Only attempt JSON parsing if output looks like JSON
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
    try {
      const parsed = JSON.parse(output)
      if (typeof parsed === 'object' && parsed !== null) {
        return JSON.stringify(parsed, null, 2)
      }
    } catch {
      // Not valid JSON, return as-is
    }
  }
  return output
}

/**
 * Truncates a string to a specific number of lines.
 */
function truncateLines(str: string, maxLines: number = 5): string {
  const lines = str.split("\n")
  if (lines.length <= maxLines) return str
  return lines.slice(0, maxLines).join("\n") + "\n... (truncated)"
}

function formatToolArgs(tool: string, args: any) {
  if (!args) return null
  
  try {
    // 1. Special handling for 'mv' - show source and destination clearly
    if (tool === 'mv') {
      return (
        <div className="flex flex-col gap-1 text-foreground">
          <div className="flex items-center gap-1.5 flex-wrap">
            <span className="opacity-70 border-b border-dotted break-all" title={args.source}>{args.source}</span>
            <span className="shrink-0 text-primary/50">→</span>
            <span className="font-medium break-all" title={args.destination}>{args.destination}</span>
          </div>
        </div>
      )
    }

    // 2. Identify primary targets (usually paths)
    const primaryKeys = ["path", "source", "pattern"];
    const primaryKey = primaryKeys.find(k => args[k]);
    const primary = primaryKey ? args[primaryKey] : "";
    
    // 3. Identify content-heavy fields like 'content' or 'text'
    const contentKeys = ["content", "text", "body", "old_string", "new_string"];
    
    // 4. Identify remaining args
    const secondaryEntries = Object.entries(args)
      .filter(([key]) => ![...primaryKeys, ...contentKeys, "destination", "to"].includes(key))
      .map(([key, val]) => {
        const displayVal = typeof val === 'string' ? val : JSON.stringify(val);
        return `${key}=${displayVal}`;
      });

    // 5. Extract and truncate content fields
    const contentParts = contentKeys
      .filter(k => args[k] !== undefined)
      .map(k => {
        const val = args[k];
        const valStr = typeof val === 'string' ? val : JSON.stringify(val, null, 2);
        const truncated = truncateLines(valStr, 5);
        return { key: k, value: truncated, isTruncated: valStr !== truncated };
      });

    return (
      <div className="flex flex-col gap-1.5 text-foreground text-[11px] w-full">
        {/* Primary line (Path/Target) */}
        {primary && (
          <span className="font-medium opacity-90 break-all whitespace-pre-wrap leading-tight" title={primary}>
            {primary}
          </span>
        )}

        {/* Secondary metadata line */}
        {secondaryEntries.length > 0 && (
          <span className="text-[10px] opacity-50 font-mono break-all leading-tight">
            [{secondaryEntries.join(", ")}]
          </span>
        )}

        {/* Content/Code blocks */}
        {contentParts.map(part => (
          <div key={part.key} className="mt-0.5 rounded border border-muted bg-muted/30 p-1.5 font-mono text-[10px] whitespace-pre-wrap break-all leading-normal">
            <span className="opacity-40 uppercase text-[8px] block mb-0.5">{part.key}</span>
            {part.value}
          </div>
        ))}
      </div>
    )
  } catch (e) {
    return <span className="opacity-70 text-destructive text-[10px]">Format Error</span>
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
          "flex items-start gap-3 text-[11px] font-mono bg-muted/20 border rounded-lg px-2.5 py-2 text-muted-foreground group transition-all shadow-sm",
          isPending ? "border-primary/20 animate-pulse" : isSuccess ? "border-muted/50 hover:border-primary/30" : "border-destructive/30 bg-destructive/5"
        )}>
          {isPending ? (
            <Loader2 className="size-3.5 shrink-0 text-primary animate-spin mt-0.5" />
          ) : (
            <Terminal className={cn("size-3.5 shrink-0 mt-0.5", isSuccess ? "text-primary opacity-70" : "text-destructive")} />
          )}
          
          <div className="flex flex-col gap-0.5 flex-1 min-w-0">
            <span className={cn(
              "font-bold uppercase tracking-tighter text-[9px] opacity-80 leading-none mb-0.5",
              !isPending && !isSuccess && "text-destructive"
            )}>
              {isPending ? "Executing..." : isSuccess ? "Tool Call" : "Tool Failed"}
            </span>
            <div className="flex flex-col gap-1">
              <span className="text-foreground font-bold shrink-0 text-[12px]">{call.tool}</span>
              <div className="w-full">
                {formatToolArgs(call.tool, call.args)}
              </div>
            </div>
          </div>
        </div>

        {observation && (
          <div className="w-full">
            <button
              type="button"
              onClick={() => setIsOpen(!isOpen)}
              className="w-full flex items-center justify-between gap-2 text-[10px] uppercase tracking-widest font-bold text-muted-foreground/40 hover:text-primary transition-colors px-2 py-1 group"
            >
              <div className="flex items-center gap-1.5">
                {isSuccess ? (
                  <CheckCircle2 className="size-3 text-green-500/50 group-hover:text-green-500" />
                ) : (
                  <CircleX className="size-3 text-destructive/50 group-hover:text-destructive" />
                )}
                <span>{isOpen ? "Show less" : "Output"}</span>
              </div>
              <ChevronDown className={cn(
                "size-3 transition-transform opacity-0 group-hover:opacity-100",
                isOpen && "rotate-180 opacity-100"
              )} />
            </button>
            {isOpen && (
              <pre className={cn(
                "mt-1 p-2.5 rounded-lg text-[11px] font-mono overflow-x-auto border-l-2 leading-relaxed shadow-inner",
                isSuccess ? "bg-black/5 border-primary/30" : "bg-destructive/5 border-destructive/50 text-destructive/90"
              )}>
                {formatOutput(observation.output) || "No output returned."}
              </pre>
            )}
          </div>
        )}
      </div>
    )
  }
)
ChatEvents.displayName = "ChatEvents"

export { ChatEvents }
