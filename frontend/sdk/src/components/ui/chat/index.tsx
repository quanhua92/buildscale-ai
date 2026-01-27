import { ChatProvider, useChat, type ChatMessageItem, type ChatModel, CHAT_MODELS, DEFAULT_MODEL } from "./chat-context"
import { ChatRoot } from "./chat-root"
import { ChatHeader } from "./chat-header"
import { ChatMessageList } from "./chat-message-list"
import { ChatMessage } from "./chat-message"
import { ChatBubble } from "./chat-bubble"
import { ChatThought } from "./chat-thought"
import { ChatEvents } from "./chat-events"
import { ChatInput } from "./chat-input"

export const Chat = Object.assign(ChatRoot, {
  Provider: ChatProvider,
  Header: ChatHeader,
  MessageList: ChatMessageList,
  Message: ChatMessage,
  Bubble: ChatBubble,
  Thought: ChatThought,
  Events: ChatEvents,
  Input: ChatInput,
})

export { useChat, type ChatMessageItem, type ChatModel, CHAT_MODELS, DEFAULT_MODEL }
