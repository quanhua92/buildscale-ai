/**
 * Auth.EditWorkspace - Pre-built workspace editing form component
 */

import { useState, useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { useAuth, type AuthError } from '../../context'
import { Card } from './AuthCard'
import { Form } from './AuthForm'
import { Input } from './AuthInput'
import { Button } from './AuthButton'

interface EditWorkspaceProps {
  workspaceId: string
  onSuccess?: () => void
  onCancel?: () => void
}

export function EditWorkspace({ workspaceId, onSuccess, onCancel }: EditWorkspaceProps) {
  const { getWorkspace, updateWorkspace } = useAuth()
  const navigate = useNavigate()
  const [isLoading, setIsLoading] = useState(true)
  const [isSaving, setIsSaving] = useState(false)
  const [error, setError] = useState<AuthError | null>(null)
  const [success, setSuccess] = useState(false)
  const [formData, setFormData] = useState({
    name: '',
  })

  // Fetch existing data
  useEffect(() => {
    const loadWorkspace = async () => {
      setIsLoading(true)
      const result = await getWorkspace(workspaceId)
      
      if (result.success && result.data) {
        setFormData({ name: result.data.name })
      } else if (result.error) {
        setError(result.error)
        toast.error('Failed to load workspace', {
          description: result.error.message
        })
      }
      setIsLoading(false)
    }

    loadWorkspace()
  }, [getWorkspace, workspaceId])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFormData(prev => ({
      ...prev,
      [e.target.name]: e.target.value
    }))
  }

  const handleSubmit = async (data: Record<string, string>) => {
    setIsSaving(true)
    setError(null)
    setSuccess(false)

    const result = await updateWorkspace(workspaceId, data.name)

    setIsSaving(false)

    if (result.success) {
      setSuccess(true)
      toast.success('Workspace updated', {
        description: `Workspace "${result.data?.name}" updated successfully.`,
      })
      
      if (onSuccess) {
        onSuccess()
      } else {
        // Redirect after 1 second to the details page
        setTimeout(() => {
          navigate({ to: `/workspaces/${workspaceId}` })
        }, 1000)
      }
    } else if (result.error) {
      setError(result.error)
      toast.error('Failed to update workspace', {
        description: result.error.message,
      })
    }
  }

  const handleCancel = () => {
    if (onCancel) {
      onCancel()
    } else {
      navigate({ to: `/workspaces/${workspaceId}` })
    }
  }

  if (isLoading) {
    return (
      <Card title="Edit Workspace" description="Loading workspace details...">
        <div className="flex justify-center p-8">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
        </div>
      </Card>
    )
  }

  return (
    <Card title="Edit Workspace" description="Update your workspace details">
      <Form onSubmit={handleSubmit} externalError={error}>
        <Input
          name="name"
          type="text"
          label="Workspace Name"
          required
          placeholder="My Startup"
          autoComplete="off"
          value={formData.name}
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
                  <p className="text-sm font-medium text-destructive">Error</p>
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
                  <p className="text-sm font-medium text-green-600">Success</p>
                  <p className="text-sm text-green-600/80 mt-1">Redirecting...</p>
                </div>
              </div>
            </div>
          </div>
        )}
        <div className="flex gap-3 pt-2">
          <Button 
            type="button" 
            variant="outline" 
            className="flex-1"
            onClick={handleCancel}
          >
            Cancel
          </Button>
          <Button isLoading={isSaving} className="flex-1">
            Save Changes
          </Button>
        </div>
      </Form>
    </Card>
  )
}
