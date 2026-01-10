/**
 * AuthInput - Input component with label using shadcn components
 */

import { Input as ShadcnInput } from '../ui/input'
import { Label } from '../ui/label'
import { cn } from '@/utils'
import { useFormErrors } from './AuthForm'

interface AuthInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string
  error?: string
}

export function Input({ label, error, className, id, name, ...props }: AuthInputProps) {
  const { errors } = useFormErrors()
  // Use manual error prop if provided, otherwise get field error from context
  const fieldError = error || (name ? errors[name] : undefined)

  return (
    <div className="space-y-2">
      {label && <Label htmlFor={id}>{label}</Label>}
      <ShadcnInput
        id={id}
        name={name}
        className={cn(fieldError && 'border-destructive focus-visible:ring-destructive', className)}
        {...props}
      />
      {fieldError && <p className="text-sm text-destructive">{fieldError}</p>}
    </div>
  )
}
