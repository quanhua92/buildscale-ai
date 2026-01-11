/**
 * AuthForm - Form with validation context
 */

import { createContext, useContext, useState, useEffect } from 'react'
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
  externalError?: { fields?: Record<string, string> } | null
}

export function Form({ children, className, onSubmit, externalError }: FormProps) {
  const [errors, setErrors] = useState<Record<string, string>>({})

  const clearErrors = () => setErrors({})

  // Sync external auth errors into form errors
  useEffect(() => {
    if (externalError?.fields) {
      setErrors(externalError.fields)
    }
  }, [externalError])

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    clearErrors()

    const formData = new FormData(e.currentTarget)
    const data = Object.fromEntries(formData.entries()) as Record<string, string>

    await onSubmit(data)
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
