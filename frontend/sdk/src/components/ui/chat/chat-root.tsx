import * as React from "react"
import { cn } from "src/utils"

export interface ChatRootProps extends React.HTMLAttributes<HTMLDivElement> {
  containerClassName?: string
}

const ChatRoot = React.forwardRef<HTMLDivElement, ChatRootProps>(
  ({ className, containerClassName, children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          "flex flex-col h-full w-full bg-background overflow-hidden relative",
          className
        )}
        {...props}
      >
        <div
          className={cn(
            "flex-1 flex flex-col mx-auto w-full max-w-3xl lg:max-w-4xl px-4",
            containerClassName
          )}
        >
          {children}
        </div>
      </div>
    )
  }
)
ChatRoot.displayName = "ChatRoot"

export { ChatRoot }
