/**
 * Auth.Logout - Pre-built logout confirmation component
 */

import { useState } from 'react'
import { toast } from 'sonner'
import { useAuth, type AuthError } from '../../context'
import { Card } from './AuthCard'
import { Button } from './AuthButton'

export function Logout() {
  const { logout } = useAuth()
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState(false)

  const handleLogout = async () => {
    setIsLoading(true)
    setError(null)
    setSuccess(false)

    const result = await logout()

    setIsLoading(false)

    if (result.success) {
      setSuccess(true)
      toast.success('Logged out successfully', {
        description: 'See you next time!',
      })
    } else if (result.error) {
      setError(result.error)
      toast.error('Logout failed', {
        description: result.error.message,
      })
    }
  }

  return (
    <Card title="Sign Out" description="Are you sure you want to sign out?">
      <form onSubmit={(e) => { e.preventDefault(); handleLogout() }}>
        <div className="space-y-4">
          {error && !error.fields && (
            <div className="animate-in fade-in-0 slide-in-from-top-2 duration-300">
              <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-destructive/20 p-1">
                    <div className="h-3 w-3 rounded-full bg-destructive" />
                  </div>
                  <div className="flex-1">
                    <p className="text-sm font-medium text-destructive">Logout Error</p>
                    <p className="text-sm text-destructive/80 mt-1">{error.message}</p>
                  </div>
                </div>
              </div>
            </div>
          )}
          {success && (
            <div className="animate-in fade-in-0 slide-in-from-top-2 duration-300">
              <div className="rounded-lg border border-green-500/50 bg-green-500/10 p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/20 p-1">
                    <svg className="h-3 w-3 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                  </div>
                  <div className="flex-1">
                    <p className="text-sm font-medium text-green-600">Logout Successful</p>
                    <p className="text-sm text-green-600/80 mt-1">Redirecting you to login...</p>
                  </div>
                </div>
              </div>
            </div>
          )}
          <Button isLoading={isLoading}>
            Sign Out
          </Button>
        </div>
      </form>
    </Card>
  )
}
