import * as React from "react"
import { Button } from "../button"
import { Input } from "../input"
import { Label } from "../label"
import type { JSONSchema } from "../../../api/types"

interface ChatSchemaFormProps {
  schema: JSONSchema
  onSubmit: (value: any) => void
  onCancel: () => void
}

export function ChatSchemaForm({ schema, onSubmit, onCancel }: ChatSchemaFormProps) {
  const [value, setValue] = React.useState<any>(() => getDefaultValue(schema))
  const [error, setError] = React.useState<string>()

  const validate = (val: any): boolean => {
    if (schema.pattern && typeof val === 'string' && !new RegExp(schema.pattern).test(val)) {
      setError(`Invalid format`)
      return false
    }
    if (schema.minLength && typeof val === 'string' && val.length < schema.minLength) {
      setError(`Minimum ${schema.minLength} characters`)
      return false
    }
    if (schema.maxLength && typeof val === 'string' && val.length > schema.maxLength) {
      setError(`Maximum ${schema.maxLength} characters`)
      return false
    }
    setError(undefined)
    return true
  }

  const handleSubmit = () => {
    if (validate(value)) onSubmit(value)
  }

  return (
    <div className="space-y-3">
      {renderInput(schema, value, setValue, error)}
      <div className="flex gap-2">
        <Button size="sm" onClick={handleSubmit}>Submit</Button>
        <Button size="sm" variant="ghost" onClick={onCancel}>Cancel</Button>
      </div>
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}

function renderInput(
  schema: JSONSchema,
  value: any,
  onChange: (val: any) => void,
  error?: string
): React.ReactNode {
  switch (schema.type) {
    case 'boolean':
      return (
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={value}
            onChange={(e) => onChange(e.target.checked)}
            className="h-4 w-4"
          />
          <Label className="text-sm">{schema.description || 'Yes'}</Label>
        </div>
      )

    case 'string':
      if (schema.enum && schema.enum.length > 0) {
        return (
          <div className="space-y-2">
            {schema.enum.map((opt: any) => (
              <div key={String(opt)} className="flex items-center gap-2">
                <input
                  type="radio"
                  id={`radio-${String(opt)}`}
                  name="enum-radio"
                  value={String(opt)}
                  checked={value === opt}
                  onChange={() => onChange(opt)}
                  className="h-4 w-4"
                />
                <Label htmlFor={`radio-${String(opt)}`}>{String(opt)}</Label>
              </div>
            ))}
          </div>
        )
      }
      return (
        <Input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={schema.description}
          className={error ? 'border-destructive' : ''}
        />
      )

    case 'number':
      return (
        <Input
          type="number"
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          placeholder={schema.description}
        />
      )

    case 'array':
      if (schema.items?.enum && Array.isArray(schema.items.enum)) {
        return (
          <div className="space-y-2">
            {schema.items.enum.map((opt: any) => (
              <div key={String(opt)} className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id={`array-${String(opt)}`}
                  checked={Array.isArray(value) && value.includes(opt)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      onChange([...(value || []), opt])
                    } else {
                      onChange((value || []).filter((v: any) => v !== opt))
                    }
                  }}
                  className="h-4 w-4"
                />
                <Label htmlFor={`array-${String(opt)}`}>{String(opt)}</Label>
              </div>
            ))}
          </div>
        )
      }
      return (
        <textarea
          value={Array.isArray(value) ? value.join('\n') : ''}
          onChange={(e) => onChange(e.target.value.split('\n'))}
          placeholder={schema.description}
          className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        />
      )

    case 'object':
      if (schema.properties) {
        return (
          <div className="space-y-2">
            {Object.entries(schema.properties).map(([key, propSchema]) => (
              <div key={key}>
                <Label className="text-xs">{key}</Label>
                {renderInput(
                  propSchema as JSONSchema,
                  value?.[key],
                  (val) => onChange({ ...value, [key]: val })
                )}
              </div>
            ))}
          </div>
        )
      }
      return (
        <textarea
          value={typeof value === 'string' ? value : JSON.stringify(value)}
          onChange={(e) => onChange(e.target.value)}
          placeholder={schema.description}
          className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        />
      )

    default:
      return (
        <textarea
          value={typeof value === 'string' ? value : JSON.stringify(value)}
          onChange={(e) => onChange(e.target.value)}
          placeholder="Enter value..."
          className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        />
      )
  }
}

function getDefaultValue(schema: JSONSchema): any {
  switch (schema.type) {
    case 'boolean': return false
    case 'string': return schema.enum?.[0] || ''
    case 'number': return 0
    case 'array': return []
    case 'object': return {}
    default: return ''
  }
}
