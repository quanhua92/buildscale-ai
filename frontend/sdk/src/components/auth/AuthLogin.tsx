/**
 * Auth.Login - Pre-built login form component
 */

import { useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { useAuth, type AuthError } from '../../context'
import { Card } from './AuthCard'
import { Form } from './AuthForm'
import { Input } from './AuthInput'
import { Button } from './AuthButton'

export function Login() {
  const { login, redirectTarget } = useAuth()
  const navigate = useNavigate()
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState(false)
  const [formData, setFormData] = useState({
    email: '',
    password: ''
  })

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFormData(prev => ({
      ...prev,
      [e.target.name]: e.target.value
    }))
  }

  const handleSubmit = async (data: Record<string, string>) => {
    setIsLoading(true)
    setError(null)
    setSuccess(false)

    const result = await login(data.email, data.password)

    setIsLoading(false)

    if (result.success) {
      setSuccess(true)
      toast.success('Login successful', {
        description: 'Welcome back!',
      })
      // Redirect after 1 second
      setTimeout(() => {
        navigate({ to: redirectTarget, replace: true })
      }, 1000)
    } else if (result.error) {
      setError(result.error)
      toast.error('Login failed', {
        description: result.error.message,
      })
    }
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
          value={formData.email}
          onChange={handleChange}
        />
        <Input
          name="password"
          type="password"
          label="Password"
          required
          placeholder="••••••••"
          className="delay-200"
          autoComplete="current-password"
          value={formData.password}
          onChange={handleChange}
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
                  <p className="text-sm font-medium text-green-600">Login Successful</p>
                  <p className="text-sm text-green-600/80 mt-1">Redirecting you now...</p>
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
