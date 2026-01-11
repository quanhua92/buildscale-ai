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
    <ShadcnCard className={cn('w-full max-w-md shadow-lg', className)}>
      {(title || description) && (
        <CardHeader className="space-y-1">
          {title && <CardTitle className="text-2xl font-bold tracking-tight">{title}</CardTitle>}
          {description && <CardDescription className="text-base">{description}</CardDescription>}
        </CardHeader>
      )}
      <CardContent className="pt-6">{children}</CardContent>
    </ShadcnCard>
  )
}
