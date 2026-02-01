import { Button } from "../button"
import { X } from "lucide-react"
import { ChatSchemaForm } from "./chat-schema-form"
import type { Question } from "../../../api/types"

interface ChatQuestionBarProps {
  question: Question
  onSubmit: (answer: any) => void
  onDismiss: () => void
}

export function ChatQuestionBar({ question, onSubmit, onDismiss }: ChatQuestionBarProps) {
  // For array-type questions (multi-select), always use schema form (checkboxes)
  // even if AI incorrectly provided buttons. Buttons are for single-select only.
  const isArrayQuestion = question.schema?.type === 'array'

  // If buttons provided AND not an array question, render button bar
  if (question.buttons && question.buttons.length > 0 && !isArrayQuestion) {
    return (
      <div className="border-b bg-blue-50 dark:bg-blue-950 p-4">
        <div className="flex items-start gap-3">
          {/* Question text takes full width */}
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium pr-8">{question.question}</p>
            {/* Buttons wrap in responsive grid below question */}
            <div className="flex flex-wrap gap-2 mt-3">
              {question.buttons.map((button, i) => (
                <Button
                  key={i}
                  size="sm"
                  variant={button.variant === 'danger' ? 'destructive' : 'default'}
                  onClick={() => onSubmit(button.value)}
                >
                  {button.label}
                </Button>
              ))}
            </div>
          </div>
          {/* Dismiss button absolutely positioned on top right */}
          <Button
            variant="ghost"
            size="icon"
            className="shrink-0"
            onClick={onDismiss}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>
    )
  }

  // Otherwise render schema form (for arrays, complex types, or when no buttons provided)
  return (
    <div className="border-b bg-blue-50 dark:bg-blue-950 p-4">
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium mb-3 pr-8">{question.question}</p>
          <ChatSchemaForm
            schema={question.schema}
            onSubmit={onSubmit}
            onCancel={onDismiss}
          />
        </div>
        <Button variant="ghost" size="icon" className="shrink-0" onClick={onDismiss}>
          <X className="h-4 w-4" />
        </Button>
      </div>
    </div>
  )
}
