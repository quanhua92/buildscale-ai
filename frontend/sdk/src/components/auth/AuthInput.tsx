/**
 * AuthInput - Input component with label using shadcn components
 */

import { useState } from 'react'
import type { LucideIcon } from 'lucide-react'
import { Mail, Lock, User, Eye, EyeOff } from 'lucide-react'
import { Input as ShadcnInput } from '../ui/input'
import { Label } from '../ui/label'
import { cn } from '@/utils'
import { useFormErrors } from './AuthForm'

interface AuthInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string
  error?: string
}

// Helper function to get the appropriate icon based on input type/name
const getInputIcon = (type: string, name: string): LucideIcon | null => {
  if (type === 'password' || name === 'password') return Lock
  if (type === 'email' || name === 'email') return Mail
  if (name === 'full_name' || name === 'username') return User
  return null
}

export function Input({ label, error, className, id, name, type = 'text', ...props }: AuthInputProps) {
  const { errors } = useFormErrors()
  // Use manual error prop if provided, otherwise get field error from context
  const fieldError = error || (name ? errors[name] : undefined)

  const [showPassword, setShowPassword] = useState(false)
  const Icon = getInputIcon(type, name || '')

  const isPassword = type === 'password' || name?.includes('password')
  const inputType = isPassword && showPassword ? 'text' : type

  return (
    <div className="space-y-2 animate-in fade-in-0 slide-in-from-bottom-2 duration-300">
      {label && (
        <Label htmlFor={id} className="text-sm font-medium text-foreground">
          {label}
        </Label>
      )}
      <div className="relative group">
        {/* Icon on left side */}
        {Icon && (
          <div className="absolute left-3 top-1/2 -translate-y-1/2 z-10">
            <Icon className="h-4 w-4 text-muted-foreground group-focus-within:text-foreground transition-colors duration-200" />
          </div>
        )}

        <ShadcnInput
          id={id}
          name={name}
          type={inputType}
          className={cn(
            'transition-all duration-200',
            'focus-visible:ring-2 focus-visible:ring-ring/20',
            'placeholder:text-muted-foreground/60',
            Icon && 'pl-9', // Left padding for icon (36px)
            isPassword && 'pr-10', // Right padding for password toggle
            fieldError && 'border-destructive focus-visible:ring-destructive/20',
            className
          )}
          {...props}
        />

        {/* Password visibility toggle on right side */}
        {isPassword && (
          <button
            type="button"
            onClick={() => setShowPassword(!showPassword)}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors duration-200 focus:outline-none focus:ring-2 focus:ring-ring rounded-sm"
            tabIndex={-1}
          >
            {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
          </button>
        )}
      </div>

      {/* Error message with bullet indicator */}
      {fieldError && (
        <p className="text-sm text-destructive animate-in fade-in-0 slide-in-from-top-1 duration-200 flex items-center gap-1">
          <span className="inline-block w-1 h-1 rounded-full bg-destructive" />
          {fieldError}
        </p>
      )}
    </div>
  )
}
