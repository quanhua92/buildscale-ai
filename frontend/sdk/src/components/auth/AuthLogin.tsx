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
        />
        <Input
          name="password"
          type="password"
          label="Password"
          required
          placeholder="••••••••"
        />
        {error && !error.fields && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-800">
            {error.message}
          </div>
        )}
        <Button isLoading={isLoading}>Sign In</Button>
      </Form>
    </Card>
  )
}
