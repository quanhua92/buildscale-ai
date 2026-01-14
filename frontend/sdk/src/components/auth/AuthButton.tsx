/**
 * AuthButton - Submit button with loading state
 */

import { Loader2 } from 'lucide-react'
import { Button as ShadcnButton, type ButtonProps as ShadcnButtonProps } from '../ui/button'
import { cn } from '@/utils'

interface ButtonProps extends ShadcnButtonProps {
  isLoading?: boolean
}

export function Button({ children, isLoading, className, disabled, type = 'submit', ...props }: ButtonProps) {
  return (
    <ShadcnButton
      type={type}
      size="lg"
      disabled={disabled || isLoading}
      className={cn('w-full', isLoading && 'gap-2', className)}
      {...props}
    >
      {isLoading ? (
        <>
          <Loader2 className="h-4 w-4 animate-spin" />
          <span>Loading...</span>
        </>
      ) : (
        children
      )}
    </ShadcnButton>
  )
}
