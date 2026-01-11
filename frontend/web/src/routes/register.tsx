import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { Auth, useAuth } from '@buildscale/sdk'
import { useEffect } from 'react'

export const Route = createFileRoute('/register')({
  component: Register,
})

function Register() {
  const { isAuthenticated, success, redirectTarget } = useAuth()
  const navigate = useNavigate()

  // Redirect if already authenticated
  useEffect(() => {
    if (isAuthenticated) {
      navigate({ to: redirectTarget, replace: true })
    }
  }, [isAuthenticated, navigate, redirectTarget])

  // Redirect after successful registration (1 second delay)
  useEffect(() => {
    if (success && isAuthenticated) {
      const timer = setTimeout(() => {
        navigate({ to: redirectTarget, replace: true })
      }, 1000)
      return () => clearTimeout(timer)
    }
  }, [success, isAuthenticated, navigate, redirectTarget])

  return (
    <Auth>
      <Auth.Register />
    </Auth>
  )
}
