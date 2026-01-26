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
          "flex flex-col flex-1 w-full bg-background overflow-hidden",
          className
        )}
        {...props}
      >
        <div
          className={cn(
            "flex flex-col mx-auto w-full max-w-3xl lg:max-w-4xl px-4 flex-1 overflow-hidden",
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
