import * as React from "react"
import { Info, ChevronDown, ChevronRight, FileText, MessageSquare, Wrench, Paperclip, Cpu } from "lucide-react"
import { Button } from "../button"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "../dialog"
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../collapsible"
import { Skeleton } from "../skeleton"
import { Separator } from "../separator"
import { useAuth } from "../../../context/AuthContext"
import { cn } from "@/utils"
import type { ChatContextResponse } from "../../../api/types"

// Utility for formatting token counts
const formatTokens = (tokens: number): string => {
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}k`
  return tokens.toString()
}

// Progress bar component for utilization
const UtilizationBar = ({ percent, className }: { percent: number; className?: string }) => {
  const getColor = (p: number) => {
    if (p < 50) return "bg-green-500"
    if (p < 80) return "bg-yellow-500"
    return "bg-red-500"
  }

  return (
    <div className={cn("h-2 w-full bg-muted rounded-full overflow-hidden", className)}>
      <div
        className={cn("h-full transition-all duration-300", getColor(percent))}
        style={{ width: `${Math.min(percent, 100)}%` }}
      />
    </div>
  )
}

interface ChatContextDialogProps {
  workspaceId: string
  chatId?: string
}

export function ChatContextDialog({ workspaceId, chatId }: ChatContextDialogProps) {
  const { apiClient } = useAuth()
  const [open, setOpen] = React.useState(false)
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  const [data, setData] = React.useState<ChatContextResponse | null>(null)

  // Expanded state for each section
  const [expanded, setExpanded] = React.useState({
    summary: true,
    systemPrompt: true,
    history: false,
    tools: false,
    attachments: false,
  })

  // Fetch context when dialog opens
  React.useEffect(() => {
    if (!open || !chatId) return

    const fetchContext = async () => {
      setLoading(true)
      setError(null)
      try {
        const response = await apiClient.get<ChatContextResponse>(
          `/workspaces/${workspaceId}/chats/${chatId}/context`
        )
        setData(response)
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load context")
      } finally {
        setLoading(false)
      }
    }

    fetchContext()
  }, [open, chatId, workspaceId, apiClient])

  const toggleSection = (section: keyof typeof expanded) => {
    setExpanded(prev => ({ ...prev, [section]: !prev[section] }))
  }

  return (
    <>
      {/* Trigger Button */}
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setOpen(true)}
        disabled={!chatId}
        className="h-7 gap-1.5 text-xs"
        title="View AI Context"
      >
        <Info className="size-3.5" />
        <span>Context</span>
      </Button>

      {/* Dialog */}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl max-h-[85vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Cpu className="size-5" />
              AI Context
            </DialogTitle>
            <DialogDescription>
              Everything sent to the AI for this chat session
            </DialogDescription>
          </DialogHeader>

          {loading && (
            <div className="space-y-4 py-4">
              <Skeleton className="h-20 w-full" />
              <Skeleton className="h-32 w-full" />
              <Skeleton className="h-24 w-full" />
            </div>
          )}

          {error && (
            <div className="text-sm text-destructive py-4">
              Error: {error}
            </div>
          )}

          {data && (
            <div className="space-y-4 py-4">
              {/* Summary Card */}
              <Collapsible open={expanded.summary} onOpenChange={() => toggleSection('summary')}>
                <CollapsibleTrigger className="flex items-center gap-2 w-full text-left">
                  {expanded.summary ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                  <span className="font-semibold">Summary</span>
                  <span className="ml-auto text-sm text-muted-foreground">
                    {formatTokens(data.summary.total_tokens)} / {formatTokens(data.summary.token_limit)} tokens
                  </span>
                </CollapsibleTrigger>
                <CollapsibleContent className="pt-3 space-y-3">
                  {/* Model & Mode */}
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-muted-foreground">Model</span>
                    <code className="text-xs bg-muted px-2 py-0.5 rounded">{data.summary.model}</code>
                  </div>

                  {/* Utilization */}
                  <div className="space-y-1">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-muted-foreground">Utilization</span>
                      <span className={cn(
                        "font-medium",
                        data.summary.utilization_percent >= 80 && "text-red-500"
                      )}>
                        {data.summary.utilization_percent.toFixed(1)}%
                      </span>
                    </div>
                    <UtilizationBar percent={data.summary.utilization_percent} />
                  </div>

                  {/* Token Breakdown */}
                  <div className="grid grid-cols-4 gap-2 text-xs">
                    {Object.entries({
                      System: data.summary.breakdown.system_prompt_tokens,
                      History: data.summary.breakdown.history_tokens,
                      Tools: data.summary.breakdown.tools_tokens,
                      Attachments: data.summary.breakdown.attachments_tokens,
                    }).map(([label, tokens]) => (
                      <div key={label} className="bg-muted rounded p-2 text-center">
                        <div className="font-medium">{formatTokens(tokens)}</div>
                        <div className="text-muted-foreground">{label}</div>
                      </div>
                    ))}
                  </div>
                </CollapsibleContent>
              </Collapsible>

              <Separator />

              {/* System Prompt Section */}
              <Collapsible open={expanded.systemPrompt} onOpenChange={() => toggleSection('systemPrompt')}>
                <CollapsibleTrigger className="flex items-center gap-2 w-full text-left">
                  {expanded.systemPrompt ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                  <FileText className="size-4" />
                  <span className="font-semibold">System Prompt</span>
                  <span className="ml-auto text-sm text-muted-foreground">
                    {formatTokens(data.system_prompt.token_count)} tokens
                  </span>
                </CollapsibleTrigger>
                <CollapsibleContent className="pt-3">
                  <div className="bg-muted rounded p-3 text-xs font-mono whitespace-pre-wrap max-h-48 overflow-y-auto">
                    {data.system_prompt.content}
                  </div>
                  <div className="flex gap-4 mt-2 text-xs text-muted-foreground">
                    <span>Type: {data.system_prompt.persona_type}</span>
                    <span>Mode: {data.system_prompt.mode}</span>
                    <span>Chars: {data.system_prompt.char_count.toLocaleString()}</span>
                  </div>
                </CollapsibleContent>
              </Collapsible>

              <Separator />

              {/* History Section */}
              <Collapsible open={expanded.history} onOpenChange={() => toggleSection('history')}>
                <CollapsibleTrigger className="flex items-center gap-2 w-full text-left">
                  {expanded.history ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                  <MessageSquare className="size-4" />
                  <span className="font-semibold">History</span>
                  <span className="ml-auto text-sm text-muted-foreground">
                    {data.history.message_count} messages, {formatTokens(data.history.total_tokens)} tokens
                  </span>
                </CollapsibleTrigger>
                <CollapsibleContent className="pt-3 space-y-2">
                  {data.history.messages.length === 0 ? (
                    <div className="text-sm text-muted-foreground italic">No messages yet</div>
                  ) : (
                    data.history.messages.map((msg, i) => (
                      <div key={i} className="bg-muted rounded p-2 text-xs">
                        <div className="flex items-center gap-2 mb-1">
                          <span className={cn(
                            "px-1.5 py-0.5 rounded font-medium uppercase",
                            msg.role === "user" && "bg-blue-500/20 text-blue-600",
                            msg.role === "assistant" && "bg-green-500/20 text-green-600",
                            msg.role === "tool" && "bg-orange-500/20 text-orange-600",
                            msg.role === "system" && "bg-gray-500/20 text-gray-600",
                          )}>
                            {msg.role}
                          </span>
                          <span className="text-muted-foreground">{formatTokens(msg.token_count)} tokens</span>
                        </div>
                        <div className="text-muted-foreground line-clamp-2">
                          {msg.content_preview}
                        </div>
                      </div>
                    ))
                  )}
                </CollapsibleContent>
              </Collapsible>

              <Separator />

              {/* Tools Section */}
              <Collapsible open={expanded.tools} onOpenChange={() => toggleSection('tools')}>
                <CollapsibleTrigger className="flex items-center gap-2 w-full text-left">
                  {expanded.tools ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                  <Wrench className="size-4" />
                  <span className="font-semibold">Tools</span>
                  <span className="ml-auto text-sm text-muted-foreground">
                    {data.tools.tool_count} tools, ~{formatTokens(data.tools.estimated_schema_tokens)} tokens
                  </span>
                </CollapsibleTrigger>
                <CollapsibleContent className="pt-3">
                  <div className="flex flex-wrap gap-1">
                    {data.tools.tools.map((tool) => (
                      <span key={tool.name} className="bg-muted px-2 py-1 rounded text-xs font-mono">
                        {tool.name}
                      </span>
                    ))}
                  </div>
                </CollapsibleContent>
              </Collapsible>

              <Separator />

              {/* Attachments Section */}
              <Collapsible open={expanded.attachments} onOpenChange={() => toggleSection('attachments')}>
                <CollapsibleTrigger className="flex items-center gap-2 w-full text-left">
                  {expanded.attachments ? <ChevronDown className="size-4" /> : <ChevronRight className="size-4" />}
                  <Paperclip className="size-4" />
                  <span className="font-semibold">Attachments</span>
                  <span className="ml-auto text-sm text-muted-foreground">
                    {data.attachments.attachment_count} files, {formatTokens(data.attachments.total_tokens)} tokens
                  </span>
                </CollapsibleTrigger>
                <CollapsibleContent className="pt-3 space-y-2">
                  {data.attachments.attachments.length === 0 ? (
                    <div className="text-sm text-muted-foreground italic">No attachments</div>
                  ) : (
                    data.attachments.attachments.map((att, i) => (
                      <div key={i} className="bg-muted rounded p-2 text-xs">
                        <div className="flex items-center justify-between mb-1">
                          <span className="font-medium">{att.attachment_type}</span>
                          <span className="text-muted-foreground">{formatTokens(att.token_count)} tokens</span>
                        </div>
                        <div className="text-muted-foreground line-clamp-2">
                          {att.content_preview}
                        </div>
                      </div>
                    ))
                  )}
                </CollapsibleContent>
              </Collapsible>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  )
}
