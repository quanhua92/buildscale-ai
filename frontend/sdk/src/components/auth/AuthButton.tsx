/**
 * AuthButton - Submit button component with loading state
 */

import { cn } from '../../utils'

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  isLoading?: boolean
  variant?: 'primary' | 'secondary'
}

export function Button({
  children,
  isLoading,
  variant = 'primary',
  className,
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      type="submit"
      disabled={disabled || isLoading}
      className={cn(
        'w-full rounded-lg px-4 py-2 font-semibold text-white',
        'transition-colors focus:outline-none focus:ring-2 focus:ring-cyan-500',
        'disabled:cursor-not-allowed disabled:opacity-50',
        variant === 'primary' && 'bg-cyan-600 hover:bg-cyan-700',
        variant === 'secondary' && 'bg-gray-600 hover:bg-gray-700',
        className
      )}
      {...props}
    >
      {isLoading ? 'Loading...' : children}
    </button>
  )
}
