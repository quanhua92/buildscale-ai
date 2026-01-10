/**
 * AuthForm - Form with validation context
 */

import { createContext, useContext, useState } from 'react'
import type { ReactNode } from 'react'
import { cn } from '@/utils'

interface FormContextType {
  errors: Record<string, string>
  setErrors: (errors: Record<string, string>) => void
  clearErrors: () => void
}

const FormContext = createContext<FormContextType | undefined>(undefined)

interface FormProps {
  children: ReactNode
  className?: string
  onSubmit: (data: Record<string, string>) => unknown
}

export function Form({ children, className, onSubmit }: FormProps) {
  const [errors, setErrors] = useState<Record<string, string>>({})

  const clearErrors = () => setErrors({})

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    clearErrors()

    const formData = new FormData(e.currentTarget)
    const data = Object.fromEntries(formData.entries()) as Record<string, string>

    try {
      await onSubmit(data)
    } catch (err) {
      // Handle validation errors from API
      if (err instanceof Error) {
        setErrors({ form: err.message })
      }
    }
  }

  return (
    <FormContext.Provider value={{ errors, setErrors, clearErrors }}>
      <form onSubmit={handleSubmit} className={cn('space-y-6', className)}>
        {children}
      </form>
    </FormContext.Provider>
  )
}

export function useFormErrors() {
  const context = useContext(FormContext)
  if (!context) {
    throw new Error('useFormErrors must be used within Auth.Form')
  }
  return context
}
