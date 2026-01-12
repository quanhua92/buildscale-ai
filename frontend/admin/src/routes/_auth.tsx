import { createFileRoute, Outlet, useNavigate, useLocation } from '@tanstack/react-router'
import { useAuth } from '@buildscale/sdk'
import { useEffect } from 'react'

export const Route = createFileRoute('/_auth')({
  component: AuthLayout,
})

function AuthLayout() {
  const auth = useAuth()
  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    // Redirect if not authenticated after restoration completes
    if (!auth.isRestoring && !auth.isAuthenticated) {
      navigate({
        to: '/login',
        search: { redirect: location.href },
      })
    }
  }, [auth.isRestoring, auth.isAuthenticated, navigate, location.href])

  // Show loading while restoring session
  if (auth.isRestoring) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <p>Loading...</p>
      </div>
    )
  }

  // Don't render content if not authenticated
  if (!auth.isAuthenticated) {
    return null
  }

  return (
    <div className="admin-layout">
      <main>
        <Outlet />
      </main>
    </div>
  )
}
