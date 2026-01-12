/**
 * Custom hook for authentication redirect logic
 * Encapsulates common redirect patterns for auth routes
 */

import { useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useAuth } from '../context/AuthContext'

export function useAuthRedirects() {
  const { isAuthenticated, redirectTarget } = useAuth()
  const navigate = useNavigate()

  // Redirect if already authenticated
  useEffect(() => {
    if (isAuthenticated) {
      navigate({ to: redirectTarget, replace: true })
    }
  }, [isAuthenticated, navigate, redirectTarget])
}
