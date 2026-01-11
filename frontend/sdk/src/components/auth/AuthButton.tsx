/**
 * AuthButton - Submit button component with loading state using shadcn Button
 */

import { Loader2 } from 'lucide-react'
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
      className={cn(
        'w-full',
        'relative overflow-hidden',
        'transition-all duration-200',
        'active:scale-[0.98]',
        'disabled:opacity-70 disabled:cursor-not-allowed',
        isLoading && 'gap-2',
        className
      )}
      {...props}
    >
      {isLoading ? (
        <>
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="inline-block animate-pulse">Loading...</span>
        </>
      ) : (
        children
      )}
    </ShadcnButton>
  )
}
