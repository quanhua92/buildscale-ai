/**
 * AgentStatusIndicator - Visual status indicator for agent sessions
 *
 * Displays color-coded status with icons for different session states:
 * - running: Green pulsing circle
 * - idle: Yellow circle
 * - paused: Orange pause icon
 * - completed: Blue circle
 * - cancelled: Gray circle
 * - error: Red X icon
 *
 * ## Usage
 *
 * ```tsx
 * <AgentStatusIndicator status="running" size="sm" />
 * <AgentStatusIndicator status="paused" size="md" showLabel />
 * ```
 */

import { Circle, Pause, X } from 'lucide-react'
import type { SessionStatus } from '../api/types'
import { cn } from '../utils'

export interface AgentStatusIndicatorProps {
  status: SessionStatus
  size?: 'sm' | 'md' | 'lg'
  showLabel?: boolean
  className?: string
}

const sizeClasses = {
  sm: 'h-2 w-2',
  md: 'h-3 w-3',
  lg: 'h-4 w-4',
}

const labelSizeClasses = {
  sm: 'text-xs',
  md: 'text-sm',
  lg: 'text-base',
}

const statusConfig = {
  running: {
    icon: Circle,
    color: 'text-green-500',
    label: 'Running',
    filled: true,
    animate: true,
  },
  idle: {
    icon: Circle,
    color: 'text-yellow-500',
    label: 'Idle',
    filled: true,
    animate: false,
  },
  paused: {
    icon: Pause,
    color: 'text-orange-500',
    label: 'Paused',
    filled: false,
    animate: false,
  },
  completed: {
    icon: Circle,
    color: 'text-blue-500',
    label: 'Completed',
    filled: true,
    animate: false,
  },
  cancelled: {
    icon: Circle,
    color: 'text-gray-500',
    label: 'Cancelled',
    filled: false,
    animate: false,
  },
  error: {
    icon: X,
    color: 'text-red-500',
    label: 'Error',
    filled: false,
    animate: false,
  },
} as const

export function AgentStatusIndicator({
  status,
  size = 'sm',
  showLabel = false,
  className,
}: AgentStatusIndicatorProps) {
  const config = statusConfig[status]
  const Icon = config.icon

  return (
    <div className={cn('flex items-center gap-1.5', config.color, className)}>
      <Icon
        className={cn(
          sizeClasses[size],
          config.filled && 'fill-current',
          config.animate && 'animate-pulse'
        )}
      />
      {showLabel && (
        <span className={cn('font-medium', labelSizeClasses[size])}>
          {config.label}
        </span>
      )}
    </div>
  )
}
