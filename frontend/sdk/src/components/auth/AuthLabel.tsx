/**
 * AuthLabel - Form label component
 */

import { cn } from '../../utils'

interface LabelProps extends React.LabelHTMLAttributes<HTMLLabelElement> {
  htmlFor: string
  children: React.ReactNode
}

export function Label({ htmlFor, children, className }: LabelProps) {
  return (
    <label
      htmlFor={htmlFor}
      className={cn('mb-2 block text-sm font-medium text-gray-700', className)}
    >
      {children}
    </label>
  )
}
