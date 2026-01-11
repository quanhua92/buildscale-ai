/**
 * AuthInput - Input component with label, inline icon, and password toggle
 */

import { useState } from 'react'
import type { LucideIcon } from 'lucide-react'
import { Mail, Lock, User, Eye, EyeOff } from 'lucide-react'
import { cn } from '@/utils'
import { useFormErrors } from './AuthForm'

interface AuthInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string
  error?: string
}

const getInputIcon = (type: string, name: string): LucideIcon | null => {
  if (type === 'password' || name === 'password') return Lock
  if (type === 'email' || name === 'email') return Mail
  if (name === 'full_name' || name === 'username') return User
  return null
}

export function Input({ label, error, className, id, name, type = 'text', disabled, ...props }: AuthInputProps) {
  const { errors } = useFormErrors()
  const fieldError = error || (name ? errors[name] : undefined)
  const [showPassword, setShowPassword] = useState(false)
  const Icon = getInputIcon(type, name || '')
  const isPassword = type === 'password' || name?.includes('password')
  const inputType = isPassword && showPassword ? 'text' : type

  return (
    <div className="space-y-2">
      {label && (
        <label className="text-sm font-medium text-foreground" htmlFor={id}>
          {label}
        </label>
      )}

      <div
        className={cn(
          'flex items-center w-full h-11 rounded-md border border-input bg-background px-3 text-sm shadow-sm transition-colors',
          'focus-within:outline-none focus-within:ring-1 focus-within:ring-ring',
          'disabled:cursor-not-allowed disabled:opacity-50',
          fieldError && 'border-destructive focus-within:ring-destructive',
          className
        )}
      >
        {Icon && (
          <div className="flex items-center justify-center pr-3 text-muted-foreground shrink-0">
            <Icon size={16} />
          </div>
        )}

        <input
          id={id}
          name={name}
          type={inputType}
          disabled={disabled}
          className="flex-1 bg-transparent border-0 outline-none text-sm placeholder:text-muted-foreground min-w-0"
          {...props}
        />

        {isPassword && (
          <button
            type="button"
            onClick={() => setShowPassword(!showPassword)}
            className="flex items-center justify-center pl-3 text-muted-foreground hover:text-foreground transition-colors focus:outline-none focus:ring-2 focus:ring-ring rounded shrink-0"
            tabIndex={-1}
          >
            {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        )}
      </div>

      {fieldError && (
        <p className="text-sm text-destructive flex items-center gap-2">
          <span className="w-1 h-1 rounded-full bg-destructive shrink-0" />
          {fieldError}
        </p>
      )}
    </div>
  )
}
