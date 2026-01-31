import { Lightbulb, Hammer } from "lucide-react"
import type { ChatMode } from "../../../api/types"

interface ChatModeToggleProps {
  currentMode: ChatMode
  onModeChange: (mode: ChatMode) => void
  disabled?: boolean
}

export function ChatModeToggle({ currentMode, onModeChange, disabled }: ChatModeToggleProps) {
  return (
    <div className="inline-flex items-center gap-1 rounded-lg bg-muted p-1">
      <button
        onClick={() => onModeChange('plan')}
        disabled={disabled}
        className={`inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
          currentMode === 'plan'
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground'
        } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
      >
        <Lightbulb className="h-4 w-4" />
        Plan
      </button>
      <button
        onClick={() => onModeChange('build')}
        disabled={disabled}
        className={`inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
          currentMode === 'build'
            ? 'bg-background text-foreground shadow-sm'
            : 'text-muted-foreground hover:text-foreground'
        } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
      >
        <Hammer className="h-4 w-4" />
        Build
      </button>
    </div>
  )
}
