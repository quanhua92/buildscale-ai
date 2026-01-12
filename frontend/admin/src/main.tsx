import { StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { AuthProvider, StorageProvider, ThemeProvider, Toaster, useAuth } from '@buildscale/sdk'

// Import the generated route tree
import { routeTree } from './routeTree.gen'

import './styles.css'
import reportWebVitals from './reportWebVitals.ts'

// API base URL from environment (relative path for Vite proxy)
const apiBaseUrl = import.meta.env.VITE_API_BASE_URL || '/api/v1'

// Create a new router instance
const router = createRouter({
  routeTree,
  context: {
    // Auth context will be set by InnerApp component
    auth: undefined!,
  },
  defaultPreload: 'intent',
  scrollRestoration: true,
  defaultStructuralSharing: true,
  defaultPreloadStaleTime: 0,
  // Tell TanStack Router about the base path
  basepath: '/admin',
})

// Register the router instance for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Inner app component that provides auth context to router
function InnerApp() {
  const auth = useAuth()
  return <RouterProvider router={router} context={{ auth }} />
}

// Render the app
const rootElement = document.getElementById('app')
if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement)
  root.render(
    <StrictMode>
      <StorageProvider>
        <ThemeProvider>
          <AuthProvider apiBaseUrl={apiBaseUrl} redirectTarget="/">
            <InnerApp />
          </AuthProvider>
        </ThemeProvider>
      </StorageProvider>
      <Toaster />
    </StrictMode>,
  )
}

// If you want to start measuring performance in your app, pass a function
// to log results (for example: reportWebVitals(console.log))
// or send to an analytics endpoint. Learn more: https://bit.ly/CRA-vitals
reportWebVitals()
