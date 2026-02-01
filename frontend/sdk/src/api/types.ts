/**
 * API type definitions matching backend API exactly
 * @see backend/docs/REST_API_GUIDE.md
 */

// ============================================================================
// File Types
// ============================================================================

export type FileType = 
  | "folder" 
  | "document" 
  | "canvas" 
  | "chat" 
  | "whiteboard" 
  | "agent" 
  | "skill"

export type FileStatus = 
  | "pending" 
  | "uploading" 
  | "waiting" 
  | "processing" 
  | "ready" 
  | "failed"

export interface File {
  id: string
  workspace_id: string
  parent_id?: string | null
  author_id?: string | null
  file_type: FileType
  status: FileStatus
  name: string
  slug: string
  path: string
  is_virtual: boolean
  is_remote: boolean
  permission: number
  latest_version_id?: string | null
  deleted_at?: string | null
  created_at: string
  updated_at: string
}

// ============================================================================
// Tool Types
// ============================================================================

export interface LsEntry {
  name: string
  display_name: string
  path: string
  file_type: string
  updated_at: string
}

export interface LsResult {
  path: string
  entries: LsEntry[]
}

// ============================================================================
// User Types
// ============================================================================

export interface User {
  id: string
  email: string
  full_name: string | null
  created_at: string
  updated_at: string
}

// ============================================================================
// Workspace Types
// ============================================================================

export interface Workspace {
  id: string
  name: string
  owner_id: string
  role_name?: string | null
  created_at: string
  updated_at: string
}

export interface CreateWorkspaceRequest {
  name: string
}

export interface UpdateWorkspaceRequest {
  name: string
}

// ============================================================================
// Request Types
// ============================================================================

export interface RegisterRequest {
  email: string
  password: string
  confirm_password: string
  full_name?: string
}

export interface LoginRequest {
  email: string
  password: string
}

// ============================================================================
// Response Types
// ============================================================================

export interface CreateWorkspaceResponse {
  workspace: Workspace
  // roles and owner_membership are also returned but often not needed immediately by client
}

export interface ListWorkspacesResponse {
  workspaces: Workspace[]
  count: number
}

export interface GetWorkspaceResponse {
  workspace: Workspace
}

export interface UpdateWorkspaceResponse {
  workspace: Workspace
}

// ============================================================================
// Chat Types
// ============================================================================

export interface CreateChatRequest {
  goal: string
  files?: string[]
  agents?: string[]
  model?: string
  role?: string
}

export interface CreateChatResponse {
  chat_id: string
  plan_id: string | null
}

export interface PostChatMessageRequest {
  content: string
  model?: string
  metadata?: {
    question_answer?: {
      question_id: string
      answers: Record<string, any>
    }
  }
}

export interface PostChatMessageResponse {
  status: "accepted"
}

export type ChatMessageRole = "system" | "user" | "assistant" | "tool"

export interface ChatMessage {
  id: string
  file_id: string
  workspace_id: string
  role: ChatMessageRole
  content: string
  metadata: any
  created_at: string
  updated_at: string
}

export interface AgentConfig {
  agent_id?: string | null
  model: string
  temperature: number
  persona_override?: string | null
  previous_response_id?: string | null
  mode: 'plan' | 'build'
  plan_file?: string | null
}

export interface ChatSession {
  file_id: string
  agent_config: AgentConfig
  messages: ChatMessage[]
}

export type GetChatResponse = ChatSession

export interface WorkspaceMemberDetailed {
  workspace_id: string
  user_id: string
  email: string
  full_name: string | null
  role_id: string
  role_name: string
}

export interface GetMembershipResponse {
  member: WorkspaceMemberDetailed
}

export interface AuthResponse {
  user: User
  access_token: string
  refresh_token: string
  access_token_expires_at: string
  refresh_token_expires_at: string
}

export interface RefreshTokenResponse {
  access_token: string
  refresh_token: string | null // Rotated or null (within grace period)
  expires_at: string
}

export interface ErrorResponse {
  message: string
  error?: string
  code?: string
}

// ============================================================================
// Plan Mode Types
// ============================================================================

export type ChatMode = 'plan' | 'build'

export interface ChatMetadata {
  mode: ChatMode
  plan_file: string | null
}

// Question Types
export interface QuestionPendingData {
  question_id: string
  questions: Question[]  // Array of questions from backend
  created_at: string
}

export interface Question {
  id?: string  // question_id (added by frontend)
  name: string
  question: string
  schema: JSONSchema
  buttons?: QuestionButton[]
  createdAt?: Date  // added by frontend
}

export interface QuestionButton {
  label: string
  value: any
  variant?: 'primary' | 'secondary' | 'danger'
}

export interface UserAnswer {
  question_id: string
  answer: any
}

// Mode Changed Event
export interface ModeChangedData {
  mode: ChatMode
  plan_file: string | null
  timestamp: string
}

// JSON Schema Types
export type JSONSchemaType = 'string' | 'number' | 'boolean' | 'array' | 'object'

export interface JSONSchema {
  type: JSONSchemaType
  description?: string
  enum?: any[]
  pattern?: string
  minLength?: number
  maxLength?: number
  minimum?: number
  maximum?: number
  minItems?: number
  maxItems?: number
  properties?: Record<string, JSONSchema>
  items?: JSONSchema
  required?: string[]
}
