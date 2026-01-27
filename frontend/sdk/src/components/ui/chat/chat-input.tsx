import * as React from "react"
import { Send, Paperclip, StopCircle } from "lucide-react"
import { cn } from "src/utils"
import { Button } from "../button"
import { useChat } from "./chat-context"

export interface ChatInputProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  onSend?: (content: string) => void
}

const ChatInput = React.forwardRef<HTMLTextAreaElement, ChatInputProps>(
  ({ className, onSend, ...props }, ref) => {
    const { sendMessage, isStreaming, stopGeneration } = useChat()
    const [content, setContent] = React.useState("")
    const textareaRef = React.useRef<HTMLTextAreaElement>(null)

    const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault()
        handleSend()
      }
    }

    const handleSend = () => {
      if (!content.trim() || isStreaming) return
      sendMessage(content)
      setContent("")
      if (textareaRef.current) {
        textareaRef.current.style.height = "auto"
      }
    }

    const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setContent(e.target.value)
      // Auto-grow logic
      if (textareaRef.current) {
        textareaRef.current.style.height = "auto"
        textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`
      }
    }

    return (
      <div className="relative flex flex-col w-full gap-2 bg-background p-2 rounded-2xl border shadow-sm focus-within:ring-1 focus-within:ring-primary/20 transition-all mb-[env(safe-area-inset-bottom)]">
        <textarea
          ref={(node) => {
            // Merge refs
            if (typeof ref === "function") ref(node)
            else if (ref) ref.current = node
            textareaRef.current = node
          }}
          value={content}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          placeholder="Message Agentic Engine..."
          className={cn(
            "w-full resize-none bg-transparent px-3 py-2 text-sm focus:outline-none min-h-[44px] max-h-[200px] scrollbar-none",
            className
          )}
          rows={1}
          {...props}
        />
        
        <div className="flex items-center justify-between px-2 pb-1">
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="icon" className="size-8 rounded-full text-muted-foreground">
              <Paperclip className="size-4" />
            </Button>
          </div>

          {isStreaming ? (
            <Button
              type="button"
              variant="destructive"
              size="icon"
              className="size-8 rounded-full shadow-lg animate-in zoom-in-50 duration-200"
              onClick={stopGeneration}
            >
              <StopCircle className="size-4" />
            </Button>
          ) : (
            <Button
              type="button"
              size="icon"
              className={cn(
                "size-8 rounded-full transition-all duration-200",
                content.trim() ? "bg-primary scale-100 opacity-100" : "bg-muted scale-95 opacity-50 pointer-events-none"
              )}
              onClick={handleSend}
            >
              <Send className="size-4" />
            </Button>
          )}
        </div>
      </div>
    )
  }
)
ChatInput.displayName = "ChatInput"

export { ChatInput }
