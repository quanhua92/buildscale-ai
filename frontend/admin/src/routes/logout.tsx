import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { Auth, useAuth } from '@buildscale/sdk'
import { useEffect } from 'react'

export const Route = createFileRoute('/logout')({
  component: Logout,
})

function Logout() {
  const { isAuthenticated } = useAuth()
  const navigate = useNavigate()

  // Redirect to login after logout completes (when !isAuthenticated)
  useEffect(() => {
    if (!isAuthenticated) {
      navigate({ to: '/login', replace: true })
    }
  }, [isAuthenticated, navigate])

  return (
    <Auth>
      <Auth.Logout />
    </Auth>
  )
}
