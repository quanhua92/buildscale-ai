/**
 * AuthButton - Submit button component with loading state using shadcn Button
 */

import { Button as ShadcnButton } from '../ui/button'
import { cn } from '@/utils'

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  isLoading?: boolean
}

export function Button({
  children,
  isLoading,
  className,
  disabled,
  ...props
}: ButtonProps) {
  return (
    <ShadcnButton
      type="submit"
      disabled={disabled || isLoading}
      className={cn('w-full', className)}
      {...props}
    >
      {isLoading ? (
        <>
          <span className="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          Loading...
        </>
      ) : (
        children
      )}
    </ShadcnButton>
  )
}
