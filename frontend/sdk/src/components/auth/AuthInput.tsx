/**
 * AuthInput - Input component with label using shadcn components
 */

import { Input as ShadcnInput } from '../ui/input'
import { Label } from '../ui/label'
import { cn } from '../../utils'

interface AuthInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string
  error?: string
}

export function Input({ label, error, className, id, ...props }: AuthInputProps) {
  return (
    <div className="space-y-2">
      {label && <Label htmlFor={id}>{label}</Label>}
      <ShadcnInput
        id={id}
        className={cn(error && 'border-destructive focus-visible:ring-destructive', className)}
        {...props}
      />
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}
