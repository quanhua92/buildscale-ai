import { Lightbulb, Hammer, FileText } from "lucide-react"
import type { ChatMode } from "../../../api/types"

interface ChatModeIndicatorProps {
  mode: ChatMode
  planFile?: string | null
}

export function ChatModeIndicator({ mode, planFile }: ChatModeIndicatorProps) {
  // Extract filename from path for display
  const basename = (path: string) => {
    if (!path) return ''
    const parts = path.split('/')
    return parts[parts.length - 1] || path
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium">
      {mode === 'plan' ? (
        <>
          <Lightbulb className="h-4 w-4 text-blue-500" />
          <span className="text-blue-700 dark:text-blue-300">Plan Mode</span>
        </>
      ) : (
        <>
          <Hammer className="h-4 w-4 text-green-500" />
          <span className="text-green-700 dark:text-green-300">Build Mode</span>
          {planFile && (
            <>
              <FileText className="h-3 w-3 text-muted-foreground ml-1" />
              <span className="text-muted-foreground truncate max-w-[150px]">
                {basename(planFile)}
              </span>
            </>
          )}
        </>
      )}
    </div>
  )
}
