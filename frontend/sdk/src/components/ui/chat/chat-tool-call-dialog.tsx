import * as React from "react"
import { Terminal, CheckCircle2, CircleX, Copy, Check, Code, FileOutput, Eye } from "lucide-react"
import { Button } from "../button"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "../dialog"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../tabs"
import { cn } from "@/utils"

/**
 * Formats JSON with proper indentation
 */
function formatJson(value: unknown): string {
  if (value === null || value === undefined) return ""
  if (typeof value === "string") {
    try {
      const parsed = JSON.parse(value)
      return JSON.stringify(parsed, null, 2)
    } catch {
      return value
    }
  }
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

interface ChatToolCallDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  call: { tool: string; args: any; id: string } | null
  observation?: { output: string; success: boolean }
}

function CopyButton({ text, className }: { text: string; className?: string }) {
  const [copied, setCopied] = React.useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error("Failed to copy:", err)
    }
  }

  return (
    <Button
      variant="outline"
      size="sm"
      onClick={handleCopy}
      className={cn("h-7 px-3 text-xs gap-1.5", className)}
    >
      {copied ? (
        <>
          <Check className="size-3" />
          Copied
        </>
      ) : (
        <>
          <Copy className="size-3" />
          Copy
        </>
      )}
    </Button>
  )
}

/**
 * Renders smart preview based on tool type
 */
function ToolPreview({
  tool,
  args,
  output
}: {
  tool: string
  args: Record<string, any>
  output?: string
}) {
  // Write tool: show content being written
  if (tool === "write" && args?.content) {
    return (
      <div>
        <div className="px-4 py-2 text-xs font-mono text-muted-foreground bg-muted/30 border-b">
          {args.path || "untitled"}
        </div>
        <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
          {args.content}
        </pre>
      </div>
    )
  }

  // Edit tool: show old vs new
  if (tool === "edit") {
    const hasReplace = args?.old_string && args?.new_string
    const hasInsert = args?.insert_content

    return (
      <div>
        <div className="px-4 py-2 text-xs font-mono text-muted-foreground bg-muted/30 border-b">
          {args?.path || "unknown"}
        </div>
        <div className="p-4 space-y-3">
          {hasReplace && (
            <>
              <div>
                <div className="text-[10px] uppercase tracking-wider text-red-500/70 mb-1 font-medium">
                  Old String
                </div>
                <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-red-500/5 border border-red-500/20 rounded-lg text-red-900 dark:text-red-100">
                  {args.old_string}
                </pre>
              </div>
              <div>
                <div className="text-[10px] uppercase tracking-wider text-green-500/70 mb-1 font-medium">
                  New String
                </div>
                <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-green-500/5 border border-green-500/20 rounded-lg text-green-900 dark:text-green-100">
                  {args.new_string}
                </pre>
              </div>
            </>
          )}
          {hasInsert && (
            <div>
              <div className="text-[10px] uppercase tracking-wider text-blue-500/70 mb-1 font-medium">
                Insert at line {args?.insert_line}
              </div>
              <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-blue-500/5 border border-blue-500/20 rounded-lg text-blue-900 dark:text-blue-100">
                {args.insert_content}
              </pre>
            </div>
          )}
        </div>
      </div>
    )
  }

  // Read tool: show output content (extract from JSON if needed)
  if (tool === "read" && output) {
    let content = output
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.content) content = String(parsed.content)
    } catch { /* not JSON */ }
    return (
      <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
        {content}
      </pre>
    )
  }

  // Read multiple files: show each file's content with path headers
  if (tool === "read_multiple_files" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.files && Array.isArray(parsed.files)) {
        return (
          <div className="divide-y">
            {parsed.files.map((file: any, i: number) => (
              <div key={i}>
                <div className="px-4 py-2 text-xs font-mono text-muted-foreground bg-muted/30 border-b">
                  {file.path || `file ${i + 1}`}
                </div>
                <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
                  {file.content || file.error || 'No content'}
                </pre>
              </div>
            ))}
          </div>
        )
      }
    } catch { /* not JSON, fall through */ }
    return (
      <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
        {output}
      </pre>
    )
  }

  // Cat tool: show output content (extract from JSON if needed)
  if (tool === "cat" && output) {
    let content = output
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.content) content = String(parsed.content)
    } catch { /* not JSON */ }
    return (
      <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
        {content}
      </pre>
    )
  }

  // Mv tool: show source → destination
  if (tool === "mv") {
    return (
      <div className="p-4 flex items-center gap-3 text-sm font-mono">
        <span className="text-muted-foreground break-all">{args?.source || "?"}</span>
        <span className="text-primary shrink-0">→</span>
        <span className="font-medium break-all">{args?.destination || "?"}</span>
      </div>
    )
  }

  // Path-based tools (mkdir, rm, touch)
  if (["mkdir", "rm", "touch"].includes(tool)) {
    return (
      <div className="p-4 font-mono text-sm">
        <span className="text-muted-foreground">Path: </span>
        <span className="font-medium">{args?.path || "unknown"}</span>
      </div>
    )
  }

  // File info tool - show output details
  if (tool === "file_info" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      return (
        <div className="p-4 font-mono text-xs space-y-1.5">
          <div><span className="text-muted-foreground">Path:</span> {parsed.path || args?.path || "unknown"}</div>
          <div><span className="text-muted-foreground">Type:</span> {parsed.file_type || "unknown"}</div>
          {parsed.size !== undefined && <div><span className="text-muted-foreground">Size:</span> {parsed.size} bytes</div>}
          {parsed.line_count !== undefined && <div><span className="text-muted-foreground">Lines:</span> {parsed.line_count}</div>}
          <div><span className="text-muted-foreground">Synced:</span> {parsed.synced ? "Yes" : "No"}</div>
        </div>
      )
    } catch { /* fall through */ }
  }

  // Ls tool - show entries
  if (tool === "ls" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.entries && Array.isArray(parsed.entries) && parsed.entries.length > 0) {
        const lines = parsed.entries.map((e: any) => {
          const icon = e.file_type === 'folder' ? '📁' : '📄'
          return `${icon} ${e.name || e.path || 'unknown'}`
        })
        return <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">{lines.join('\n')}</pre>
      }
    } catch { /* fall through */ }
  }

  // Glob tool - show matches
  if (tool === "glob" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.matches && Array.isArray(parsed.matches) && parsed.matches.length > 0) {
        const lines = parsed.matches.map((m: any) => {
          const icon = m.file_type === 'folder' ? '📁' : '📄'
          return `${icon} ${m.path || m.name || 'unknown'}`
        })
        return <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">{lines.join('\n')}</pre>
      }
    } catch { /* fall through */ }
  }

  // Find tool - show matches
  if (tool === "find" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.matches && Array.isArray(parsed.matches) && parsed.matches.length > 0) {
        const lines = parsed.matches.map((m: any) => {
          const icon = m.file_type === 'folder' ? '📁' : '📄'
          return `${icon} ${m.path || m.name || 'unknown'}`
        })
        return <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">{lines.join('\n')}</pre>
      }
    } catch { /* fall through */ }
  }

  // Grep tool - show matches with line numbers
  if (tool === "grep" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.matches && Array.isArray(parsed.matches) && parsed.matches.length > 0) {
        const lines = parsed.matches.map((m: any) =>
          `${m.path || 'unknown'}:${m.line_number || '?'}: ${m.line_text || ''}`
        )
        return <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">{lines.join('\n')}</pre>
      }
    } catch { /* fall through */ }
  }

  // Ask user: show questions
  if (tool === "ask_user" && args?.questions) {
    const questions = Array.isArray(args.questions) ? args.questions : [args.questions]
    return (
      <div className="p-4 space-y-3">
        {questions.map((q: any, i: number) => (
          <div key={i} className="text-sm">
            <div className="font-medium">{q.question || String(q)}</div>
            {q.buttons && (
              <div className="flex flex-wrap gap-1.5 mt-2">
                {q.buttons.map((btn: any, j: number) => (
                  <span key={j} className="px-2 py-1 text-xs bg-muted rounded">
                    {btn.label || String(btn)}
                  </span>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    )
  }

  // Exit plan mode: simple status
  if (tool === "exit_plan_mode") {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        Exit plan mode
      </div>
    )
  }

  // Plan tools previews
  if (tool === "plan_write") {
    const hasPath = args?.path
    return (
      <div>
        <div className="px-4 py-2 text-xs font-mono text-muted-foreground bg-muted/30 border-b">
          {hasPath ? args.path : "Auto-generated: /plans/word-word-word.plan"}
        </div>
        <div className="p-4 space-y-3">
          <div>
            <div className="text-[10px] uppercase tracking-wider text-blue-500/70 mb-1 font-medium">
              Title
            </div>
            <div className="text-sm font-medium">{args?.title || "Untitled"}</div>
          </div>
          {args?.status && (
            <div>
              <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
                Status
              </div>
              <span className={cn(
                "px-2 py-0.5 text-xs rounded-full",
                args.status === "draft" && "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
                args.status === "approved" && "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300",
                args.status === "implemented" && "bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300",
                args.status === "archived" && "bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300"
              )}>
                {args.status}
              </span>
            </div>
          )}
          <div>
            <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
              Content Preview
            </div>
            <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-muted/50 rounded-lg">
              {args?.content || "No content"}
            </pre>
          </div>
        </div>
      </div>
    )
  }

  if (tool === "plan_read" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.metadata) {
        return (
          <div className="p-4 space-y-3">
            <div>
              <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
                Path
              </div>
              <div className="font-mono text-sm">{parsed.path}</div>
            </div>
            {parsed.metadata && (
              <div className="grid grid-cols-3 gap-3">
                <div>
                  <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
                    Title
                  </div>
                  <div className="text-sm">{parsed.metadata.title}</div>
                </div>
                <div>
                  <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
                    Status
                  </div>
                  <span className={cn(
                    "px-2 py-0.5 text-xs rounded-full",
                    parsed.metadata.status === "draft" && "bg-gray-100 text-gray-700",
                    parsed.metadata.status === "approved" && "bg-green-100 text-green-700",
                    parsed.metadata.status === "implemented" && "bg-blue-100 text-blue-700",
                    parsed.metadata.status === "archived" && "bg-yellow-100 text-yellow-700"
                  )}>
                    {parsed.metadata.status}
                  </span>
                </div>
                <div>
                  <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
                    Created
                  </div>
                  <div className="text-xs">{new Date(parsed.metadata.created_at).toLocaleDateString()}</div>
                </div>
              </div>
            )}
            <div>
              <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1 font-medium">
                Content
              </div>
              <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-muted/50 rounded-lg max-h-60 overflow-auto">
                {parsed.content || "No content"}
              </pre>
            </div>
          </div>
        )
      }
    } catch { /* fall through */ }
  }

  if (tool === "plan_edit") {
    return (
      <div>
        <div className="px-4 py-2 text-xs font-mono text-muted-foreground bg-muted/30 border-b">
          {args?.path || "unknown"}
        </div>
        <div className="p-4 space-y-3">
          {args?.old_string && args?.new_string && (
            <>
              <div>
                <div className="text-[10px] uppercase tracking-wider text-red-500/70 mb-1 font-medium">
                  Old String
                </div>
                <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-red-500/5 border border-red-500/20 rounded-lg text-red-900 dark:text-red-100">
                  {args.old_string}
                </pre>
              </div>
              <div>
                <div className="text-[10px] uppercase tracking-wider text-green-500/70 mb-1 font-medium">
                  New String
                </div>
                <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-green-500/5 border border-green-500/20 rounded-lg text-green-900 dark:text-green-100">
                  {args.new_string}
                </pre>
              </div>
            </>
          )}
          {args?.insert_content && (
            <div>
              <div className="text-[10px] uppercase tracking-wider text-blue-500/70 mb-1 font-medium">
                Insert at line {args?.insert_line}
              </div>
              <pre className="p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all bg-blue-500/5 border border-blue-500/20 rounded-lg text-blue-900 dark:text-blue-100">
                {args.insert_content}
              </pre>
            </div>
          )}
          <div className="text-xs text-muted-foreground italic">
            Preserves YAML frontmatter during edit
          </div>
        </div>
      </div>
    )
  }

  if (tool === "plan_list" && output) {
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.plans && Array.isArray(parsed.plans)) {
        return (
          <div className="p-4">
            <div className="text-sm text-muted-foreground mb-3">
              {parsed.total} plan{parsed.total !== 1 ? 's' : ''} found
            </div>
            <div className="space-y-2">
              {parsed.plans.map((plan: any, i: number) => (
                <div key={i} className="p-3 bg-muted/30 rounded-lg flex items-center justify-between">
                  <div>
                    <div className="font-mono text-sm">{plan.name || plan.path}</div>
                    {plan.metadata && (
                      <div className="text-xs text-muted-foreground mt-1">
                        {plan.metadata.title}
                      </div>
                    )}
                  </div>
                  {plan.metadata?.status && (
                    <span className={cn(
                      "px-2 py-0.5 text-xs rounded-full",
                      plan.metadata.status === "draft" && "bg-gray-100 text-gray-700",
                      plan.metadata.status === "approved" && "bg-green-100 text-green-700",
                      plan.metadata.status === "implemented" && "bg-blue-100 text-blue-700",
                      plan.metadata.status === "archived" && "bg-yellow-100 text-yellow-700"
                    )}>
                      {plan.metadata.status}
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>
        )
      }
    } catch { /* fall through */ }
  }

  // Default: show output if available (extract content from JSON if possible)
  if (output) {
    let content = output
    try {
      const parsed = typeof output === 'string' ? JSON.parse(output) : output
      if (parsed?.content) content = String(parsed.content)
    } catch { /* not JSON */ }
    return (
      <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
        {content}
      </pre>
    )
  }

  // Fallback: show args as JSON
  return (
    <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all text-muted-foreground">
      {formatJson(args) || "No preview available"}
    </pre>
  )
}

export function ChatToolCallDialog({
  open,
  onOpenChange,
  call,
  observation,
}: ChatToolCallDialogProps) {
  const [activeTab, setActiveTab] = React.useState("preview")

  // Reset to preview when dialog opens
  React.useEffect(() => {
    if (open) setActiveTab("preview")
  }, [open])

  if (!call) return null

  const isPending = !observation
  const isSuccess = observation?.success

  const argsJson = formatJson(call.args)
  const outputText = observation?.output || ""
  const outputJson = formatJson(observation?.output)
  const isOutputJson = outputJson !== observation?.output ||
    observation?.output?.trim().startsWith('{') ||
    observation?.output?.trim().startsWith('[')

  // Determine copy text based on active tab
  const getCopyText = () => {
    switch (activeTab) {
      case "preview":
        if (call.tool === "write" && call.args?.content) return call.args.content
        if (call.tool === "edit") {
          const parts = []
          if (call.args?.old_string) parts.push(`--- Old ---\n${call.args.old_string}`)
          if (call.args?.new_string) parts.push(`--- New ---\n${call.args.new_string}`)
          if (call.args?.insert_content) parts.push(`--- Insert ---\n${call.args.insert_content}`)
          return parts.join('\n\n') || argsJson
        }
        return outputText || argsJson
      case "args":
        return argsJson
      case "output":
        return outputText
      default:
        return ""
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl max-h-[85vh] flex flex-col p-0 gap-0">
        <DialogHeader className="px-6 py-4 border-b flex-shrink-0">
          <DialogTitle className="flex items-center gap-2">
            {isPending ? (
              <Terminal className="size-5 animate-pulse text-primary" />
            ) : isSuccess ? (
              <CheckCircle2 className="size-5 text-green-500" />
            ) : (
              <CircleX className="size-5 text-destructive" />
            )}
            <span className="font-mono">{call.tool}</span>
          </DialogTitle>
          <DialogDescription>
            {isPending
              ? "Executing..."
              : isSuccess
                ? "Completed successfully"
                : "Execution failed"}
          </DialogDescription>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0 overflow-hidden">
          <div className="px-6 pt-3 border-b flex-shrink-0">
            <TabsList className="bg-transparent p-0 h-auto">
              <TabsTrigger
                value="preview"
                className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4 py-2 text-sm font-medium"
              >
                <Eye className="size-4 mr-1.5" />
                Preview
              </TabsTrigger>
              <TabsTrigger
                value="args"
                className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4 py-2 text-sm font-medium"
              >
                <Code className="size-4 mr-1.5" />
                Arguments
              </TabsTrigger>
              {!isPending && (
                <TabsTrigger
                  value="output"
                  className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4 py-2 text-sm font-medium"
                >
                  <FileOutput className="size-4 mr-1.5" />
                  Output
                </TabsTrigger>
              )}
            </TabsList>
          </div>

          <div className="flex-1 overflow-auto">
            <TabsContent value="preview" className="m-0">
              <ToolPreview
                tool={call.tool}
                args={call.args}
                output={outputText}
              />
            </TabsContent>

            <TabsContent value="args" className="m-0">
              <pre className="p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
                {argsJson || <span className="text-muted-foreground italic">No arguments</span>}
              </pre>
            </TabsContent>

            {!isPending && (
              <TabsContent value="output" className="m-0">
                <pre
                  className={cn(
                    "p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all",
                    !isSuccess && "text-destructive/90"
                  )}
                >
                  {isOutputJson && outputJson ? outputJson : outputText || (
                    <span className="text-muted-foreground italic">No output returned.</span>
                  )}
                </pre>
              </TabsContent>
            )}
          </div>
        </Tabs>

        <div className="flex-shrink-0 flex justify-between px-6 py-3 border-t bg-muted/30">
          <CopyButton text={getCopyText()} />
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            Close
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
