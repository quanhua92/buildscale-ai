/**
 * Protected route hook - redirects to login if not authenticated
 */

import { useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useAuth } from '../context'

export function useProtectedRoute() {
  const { isAuthenticated, isRestoring } = useAuth()
  const navigate = useNavigate()

  useEffect(() => {
    if (!isRestoring && !isAuthenticated) {
      navigate({ to: '/login', replace: true })
    }
  }, [isAuthenticated, isRestoring, navigate])

  return { isAuthenticated, isRestoring }
}
