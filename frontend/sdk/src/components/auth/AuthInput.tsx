/**
 * AuthInput - Form input component with error handling
 */

import { cn } from '../../utils'

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  name: string
  label?: string
  error?: string
}

export function Input({ name, label, error, className, ...props }: InputProps) {
  return (
    <div>
      {label && (
        <label
          htmlFor={name}
          className="mb-2 block text-sm font-medium text-gray-700"
        >
          {label}
        </label>
      )}
      <input
        id={name}
        name={name}
        className={cn(
          'w-full rounded-lg border border-gray-300 px-3 py-2',
          'focus:border-cyan-500 focus:outline-none focus:ring-1 focus:ring-cyan-500',
          error && 'border-red-500 focus:border-red-500 focus:ring-red-500',
          className
        )}
        {...props}
      />
      {error && (
        <p className="mt-1 text-sm text-red-600">{error}</p>
      )}
    </div>
  )
}
