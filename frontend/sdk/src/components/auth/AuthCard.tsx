/**
 * AuthCard - Container component for auth forms using shadcn Card
 */

import type { ReactNode } from 'react'
import { Card as ShadcnCard, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card'
import { cn } from '@/utils'

interface AuthCardProps {
  children: ReactNode
  className?: string
  title?: string
  description?: string
}

export function Card({ children, className, title, description }: AuthCardProps) {
  return (
    <ShadcnCard
      className={cn(
        'animate-in fade-in-0 zoom-in-95 duration-300',
        'shadow-lg border-border/50',
        'dark:shadow-none',
        className
      )}
    >
      {(title || description) && (
        <CardHeader>
          {title && <CardTitle>{title}</CardTitle>}
          {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
      )}
      <CardContent>{children}</CardContent>
    </ShadcnCard>
  )
}
