import { ChatProvider, useChat, type ChatMessageItem, type ChatModel } from "./chat-context"
import { ChatRoot } from "./chat-root"
import { ChatHeader } from "./chat-header"
import { ChatMessageList } from "./chat-message-list"
import { ChatMessage } from "./chat-message"
import { ChatBubble } from "./chat-bubble"
import { ChatThought } from "./chat-thought"
import { ChatEvents } from "./chat-events"
import { ChatInput } from "./chat-input"
import { ChatQuestionBar } from "./chat-question-bar"
import { ChatSchemaForm } from "./chat-schema-form"
import { ChatModeIndicator } from "./chat-mode-indicator"
import { ChatModeToggle } from "./chat-mode-toggle"
import { ChatContextDialog } from "./chat-context-dialog"

export const Chat = Object.assign(ChatRoot, {
  Provider: ChatProvider,
  Header: ChatHeader,
  MessageList: ChatMessageList,
  Message: ChatMessage,
  Bubble: ChatBubble,
  Thought: ChatThought,
  Events: ChatEvents,
  Input: ChatInput,
  QuestionBar: ChatQuestionBar,
  SchemaForm: ChatSchemaForm,
  ModeIndicator: ChatModeIndicator,
  ModeToggle: ChatModeToggle,
  ContextDialog: ChatContextDialog,
})

export {
  useChat,
  type ChatMessageItem,
  type ChatModel,
  ChatQuestionBar,
  ChatSchemaForm,
  ChatModeIndicator,
  ChatModeToggle,
  ChatContextDialog,
}
