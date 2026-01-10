/**
 * Auth.Register - Pre-built register form component
 */

import { useAuth } from '../../context'
import { Card } from './AuthCard'
import { Form } from './AuthForm'
import { Input } from './AuthInput'
import { Button } from './AuthButton'

export function Register() {
  const { register, isLoading, error, clearError } = useAuth()

  const handleSubmit = async (data: Record<string, string>) => {
    clearError()
    await register({
      email: data.email,
      password: data.password,
      confirm_password: data.confirm_password,
      full_name: data.full_name,
    })
  }

  return (
    <Card title="Create Account" description="Sign up to get started">
      <Form onSubmit={handleSubmit}>
        <Input
          name="full_name"
          type="text"
          label="Full Name"
          placeholder="John Doe"
        />
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
          minLength={8}
        />
        <Input
          name="confirm_password"
          type="password"
          label="Confirm Password"
          required
          placeholder="••••••••"
          minLength={8}
        />
        {error && (
          <div className="rounded-md bg-red-50 p-3 text-sm text-red-800">
            {error.message}
          </div>
        )}
        <Button isLoading={isLoading}>Create Account</Button>
      </Form>
    </Card>
  )
}
