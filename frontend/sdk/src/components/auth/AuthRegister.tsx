/**
 * Auth.Register - Pre-built register form component
 */

import { useAuth } from '../../context'
import { Card } from './AuthCard'
import { Form } from './AuthForm'
import { Input } from './AuthInput'
import { Button } from './AuthButton'

export function Register() {
  const { register, isLoading, error, clearError, success } = useAuth()

  const handleSubmit = async (data: Record<string, string>) => {
    clearError()
    await register({
      email: data.email,
      password: data.password,
      confirm_password: data.confirm_password,
      full_name: data.full_name || undefined,
    })
  }

  return (
    <Card title="Create Account" description="Sign up to get started">
      <Form onSubmit={handleSubmit} externalError={error}>
        <Input
          name="full_name"
          type="text"
          label="Full Name"
          placeholder="John Doe"
          className="delay-100"
          autoComplete="name"
        />
        <Input
          name="email"
          type="email"
          label="Email address"
          required
          placeholder="you@example.com"
          className="delay-200"
          autoComplete="email"
        />
        <Input
          name="password"
          type="password"
          label="Password"
          required
          placeholder="••••••••"
          minLength={12}
          className="delay-300"
          autoComplete="new-password"
        />
        <Input
          name="confirm_password"
          type="password"
          label="Confirm Password"
          required
          placeholder="••••••••"
          minLength={12}
          className="delay-[400ms]"
          autoComplete="new-password"
        />
        {error && !error.fields && (
          <div className="animate-in fade-in-0 slide-in-from-top-2 duration-300">
            <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-full bg-destructive/20 p-1">
                  <div className="h-3 w-3 rounded-full bg-destructive" />
                </div>
                <div className="flex-1">
                  <p className="text-sm font-medium text-destructive">Registration Error</p>
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
                  <p className="text-sm font-medium text-green-600">Registration Successful</p>
                  <p className="text-sm text-green-600/80 mt-1">Redirecting you now...</p>
                </div>
              </div>
            </div>
          </div>
        )}
        <Button isLoading={isLoading} className="delay-[500ms]">
          Create Account
        </Button>
      </Form>
    </Card>
  )
}
