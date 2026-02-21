import * as React from "react"
import { MoreVertical, Info } from "lucide-react"
import { cn } from "src/utils"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../select"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../dropdown-menu"
import type { ChatModel, AiProvider } from "./chat-context"
import type { ChatMode } from "../../../api/types"
import { useChat } from "./chat-context"
import { ChatModeToggle } from "./chat-mode-toggle"
import { ChatContextDialogContent } from "./chat-context-dialog-content"

export interface ChatHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  modelName?: string
  model?: ChatModel
  onModelChange?: (model: ChatModel) => void
  workspaceId?: string
  chatId?: string
  mode?: ChatMode
  onModeChange?: (mode: ChatMode) => void
  isChangingMode?: boolean
}

// Provider display names
const PROVIDER_NAMES: Record<AiProvider, string> = {
  openai: "OpenAI",
  openrouter: "OpenRouter"
}

const ChatHeader = React.forwardRef<HTMLDivElement, ChatHeaderProps>(
  ({ className, modelName, model, onModelChange, workspaceId, chatId, mode, onModeChange, isChangingMode, ...props }, ref) => {
    // Get available models from chat context
    const { availableModels } = useChat()

    // State for context dialog
    const [contextDialogOpen, setContextDialogOpen] = React.useState(false)

    // Group models by provider
    const groupedModels = React.useMemo(() => {
      const grouped: Record<string, ChatModel[]> = { openai: [], openrouter: [] }
      console.log('[ChatHeader] availableModels:', availableModels)
      for (const m of availableModels) {
        if (!grouped[m.provider]) {
          grouped[m.provider] = []
        }
        grouped[m.provider].push(m)
      }
      console.log('[ChatHeader] groupedModels:', grouped)
      return grouped
    }, [availableModels])

    // Get all available provider names
    const availableProviders = Object.keys(groupedModels).filter(
      provider => groupedModels[provider as AiProvider]?.length > 0
    ) as AiProvider[]

    return (
      <div
        ref={ref}
        className={cn(
          "flex items-center py-2 px-3 border-b border-border/50 shrink-0 gap-2",
          className
        )}
        {...props}
      >
        {/* Left: Model Selector */}
        <div className="flex items-center shrink-0">
          {model && onModelChange ? (
            <Select value={model.id} onValueChange={(value) => {
              // Find the model object by id from availableModels
              const selectedModel = availableModels.find(m => m.id === value)
              if (selectedModel) {
                onModelChange(selectedModel)
              }
            }}>
              <SelectTrigger className="w-[140px] h-7 text-xs">
                <SelectValue placeholder="Select model">
                  <div className="flex items-center gap-2 truncate">
                    <span className="truncate">{model.name}</span>
                    {model.is_free && (
                      <span className="text-[10px] bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded font-medium shrink-0">
                        FREE
                      </span>
                    )}
                  </div>
                </SelectValue>
              </SelectTrigger>
              <SelectContent className="max-h-96">
                {availableProviders.map((provider, providerIndex) => {
                  const providerModels = groupedModels[provider]
                  if (!providerModels || providerModels.length === 0) return null

                  return (
                    <React.Fragment key={provider}>
                      {/* Provider Label */}
                      <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">
                        {PROVIDER_NAMES[provider]}
                      </div>
                      {/* Models for this provider */}
                      {providerModels.map((modelOption) => (
                        <SelectItem
                          key={modelOption.id}
                          value={modelOption.id}
                          className="text-xs pl-6"
                        >
                          <div className="flex items-center gap-2">
                            <span>{modelOption.name}</span>
                            {modelOption.is_free && (
                              <span className="text-[10px] bg-green-500/20 text-green-600 px-1.5 py-0.5 rounded font-medium">
                                FREE
                              </span>
                            )}
                          </div>
                        </SelectItem>
                      ))}
                      {/* Separator between providers (except last) */}
                      {providerIndex < availableProviders.length - 1 && (
                        <div className="my-1 border-t border-border/50" />
                      )}
                    </React.Fragment>
                  )
                })}
              </SelectContent>
            </Select>
          ) : modelName ? (
            <div className="text-xs font-mono text-muted-foreground bg-muted px-2.5 py-1 rounded">
              {modelName}
            </div>
          ) : null}
        </div>

        {/* Center: Mode Toggle (grows to fill space) */}
        {mode && onModeChange && (
          <div className="flex-1 flex justify-center">
            <ChatModeToggle
              currentMode={mode}
              onModeChange={onModeChange}
              disabled={isChangingMode}
            />
          </div>
        )}

        {/* Right: 3-dot Menu */}
        <div className="flex items-center shrink-0">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button className="h-7 w-7 flex items-center justify-center rounded-md border border-input bg-background hover:bg-accent hover:text-accent-foreground">
                <MoreVertical className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => setContextDialogOpen(true)}>
                <Info className="h-4 w-4 mr-2" />
                Context
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          {/* Context Dialog (controlled) */}
          {workspaceId && (
            <ChatContextDialogContent
              workspaceId={workspaceId}
              chatId={chatId}
              open={contextDialogOpen}
              onOpenChange={setContextDialogOpen}
            />
          )}
        </div>
      </div>
    )
  }
)
ChatHeader.displayName = "ChatHeader"

export { ChatHeader }
