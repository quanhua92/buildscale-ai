import { createFileRoute, redirect, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/_auth')({
  // Runs BEFORE any child route in _auth/ folder loads
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: '/login',
        search: { redirect: location.pathname },
      })
    }
  },

  // Renders shared layout for all protected routes
  component: () => (
    <div className="admin-layout">
      <main>
        <Outlet />
      </main>
    </div>
  ),
})
