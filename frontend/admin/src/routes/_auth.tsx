import { createFileRoute, redirect, Outlet } from '@tanstack/react-router'
import { useAuth } from '@buildscale/sdk'

export const Route = createFileRoute('/_auth')({
  component: AuthLayout,
})

function AuthLayout() {
  const auth = useAuth()

  // Show loading while restoring session
  if (auth.isRestoring) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <p>Loading...</p>
      </div>
    )
  }

  // Redirect if not authenticated after restoration completes
  if (!auth.isAuthenticated) {
    throw redirect({
      to: '/login',
      search: { redirect: window.location.pathname },
    })
  }

  return (
    <div className="admin-layout">
      <main>
        <Outlet />
      </main>
    </div>
  )
}
