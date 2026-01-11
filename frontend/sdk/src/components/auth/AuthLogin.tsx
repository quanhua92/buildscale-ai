/**
 * Auth.Login - Pre-built login form component
 */

import { useAuth } from '../../context'
import { Card } from './AuthCard'
import { Form } from './AuthForm'
import { Input } from './AuthInput'
import { Button } from './AuthButton'

export function Login() {
  const { login, isLoading, error, clearError } = useAuth()

  const handleSubmit = async (data: Record<string, string>) => {
    clearError()
    await login(data.email, data.password)
  }

  return (
    <Card title="Sign In" description="Enter your credentials to access your account">
      <Form onSubmit={handleSubmit} externalError={error}>
        <Input
          name="email"
          type="email"
          label="Email address"
          required
          placeholder="you@example.com"
          className="delay-100"
          autoComplete="email"
        />
        <Input
          name="password"
          type="password"
          label="Password"
          required
          placeholder="••••••••"
          className="delay-200"
          autoComplete="current-password"
        />
        {error && !error.fields && (
          <div className="animate-in fade-in-0 slide-in-from-top-2 duration-300">
            <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-destructive/20 p-1">
                  <div className="h-3 w-3 rounded-full bg-destructive" />
                </div>
                <div className="flex-1">
                  <p className="text-sm font-medium text-destructive">Authentication Error</p>
                  <p className="text-sm text-destructive/80 mt-1">{error.message}</p>
                </div>
              </div>
            </div>
          </div>
        )}
        <Button isLoading={isLoading} className="delay-300">
          Sign In
        </Button>
      </Form>
    </Card>
  )
}
