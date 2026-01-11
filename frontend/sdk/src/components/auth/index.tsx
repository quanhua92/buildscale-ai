/**
 * Auth - Main compound component
 * Provides namespace for all auth sub-components
 */

import { cn } from '@/utils'
import { Card } from './AuthCard'
import { Form } from './AuthForm'
import { Input } from './AuthInput'
import { Button } from './AuthButton'
import { Login } from './AuthLogin'
import { Register } from './AuthRegister'
import { Logout } from './AuthLogout'

interface AuthProps {
  children: React.ReactNode
  className?: string
}

function Auth({ children, className }: AuthProps) {
  return (
    <div
      className={cn(
        'flex min-h-screen items-center justify-center bg-background px-4',
        className
      )}
    >
      {children}
    </div>
  )
}

// Attach sub-components as compound components
Auth.Card = Card
Auth.Form = Form
Auth.Input = Input
Auth.Button = Button
Auth.Login = Login
Auth.Register = Register
Auth.Logout = Logout

export default Auth
