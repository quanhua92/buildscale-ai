/**
 * AuthCard - Container component for auth forms
 */

import { cn } from '../../utils'

interface CardProps {
  children: React.ReactNode
  className?: string
  title?: string
  description?: string
}

export function Card({ children, className, title, description }: CardProps) {
  return (
    <div
      className={cn(
        'w-full max-w-md space-y-8 rounded-lg bg-white p-8 shadow-md',
        className
      )}
    >
      {(title || description) && (
        <div className="text-center">
          {title && (
            <h2 className="text-3xl font-bold tracking-tight text-gray-900">
              {title}
            </h2>
          )}
          {description && (
            <p className="mt-2 text-sm text-gray-600">{description}</p>
          )}
        </div>
      )}
      {children}
    </div>
  )
}
