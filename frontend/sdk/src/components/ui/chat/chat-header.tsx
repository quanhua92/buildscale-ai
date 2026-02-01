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
import type { ChatModel, AiProvider } from "./chat-context"
import { getAvailableModels, groupModelsByProvider, LEGACY_CHAT_MODELS, DEFAULT_MODEL } from "./chat-context"

export interface ChatHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  modelName?: string
  onNewChat?: () => void
  model?: ChatModel
  onModelChange?: (model: ChatModel) => void
  children?: React.ReactNode
}

// Provider display names
const PROVIDER_NAMES: Record<AiProvider, string> = {
  openai: "OpenAI",
  openrouter: "OpenRouter"
}

const ChatHeader = React.forwardRef<HTMLDivElement, ChatHeaderProps>(
  ({ className, modelName, onNewChat, model, onModelChange, children, ...props }, ref) => {
    // Group models by provider
    const groupedModels = React.useMemo(() => {
      return groupModelsByProvider()
    }, [])

    // For backward compatibility, if model is a string (legacy), convert it
    const currentModel = typeof model === 'string'
      ? LEGACY_CHAT_MODELS.find(m => m.legacyId === model) || DEFAULT_MODEL
      : model

    // Get all available provider names
    const availableProviders = Object.keys(groupedModels).filter(
      provider => groupedModels[provider as AiProvider]?.length > 0
    ) as AiProvider[]

    return (
      <div
        ref={ref}
        className={cn(
          "flex flex-col items-center justify-between py-2 px-4 border-b border-border/50 shrink-0 gap-2",
          className
        )}
        {...props}
      >
        {/* Main header row */}
        <div className="flex items-center justify-between w-full">
          {/* Spacer for center alignment */}
          <div className="w-24" />

          {/* Center: Model Selector */}
          <div className="flex-1 flex justify-center">
            {currentModel && onModelChange ? (
              <Select value={currentModel.id} onValueChange={(value) => {
                // Find the model object by id
                const allModels = getAvailableModels()
                const selectedModel = allModels.find(m => m.id === value)
                if (selectedModel) {
                  onModelChange(selectedModel)
                }
              }}>
                <SelectTrigger className="w-[220px] h-7 text-xs">
                  <SelectValue placeholder="Select model">
                    {currentModel.name}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
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
                            {modelOption.name}
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

        {/* Optional: Children row (e.g., Mode Toggle, Mode Indicator) */}
        {children && (
          <div className="flex items-center justify-center w-full">
            {children}
          </div>
        )}
      </div>
    )
  }
)
ChatHeader.displayName = "ChatHeader"

export { ChatHeader }
