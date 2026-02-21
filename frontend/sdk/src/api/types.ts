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
  id?: string | null  // null for filesystem-only files (not in database)
  synced: boolean  // true = in database, false = filesystem-only
  name: string
  display_name: string
  path: string
  file_type: string
  is_virtual: boolean
  updated_at: string
}

export interface LsResult {
  path: string
  entries: LsEntry[]
}

export interface ReadResult {
  path: string
  content: any  // Can be string or JSON object
  hash: string
  synced: boolean  // true = in database, false = filesystem-only
  total_lines?: number | null
  truncated?: boolean | null
  offset?: number | null
  limit?: number | null
  cursor?: number | null
}

export interface GlobMatch {
  path: string
  name: string
  synced: boolean  // true = in database, false = filesystem-only
  file_type: string
  is_virtual: boolean
  size?: number | null
  updated_at: string
}

export interface GlobResult {
  pattern: string
  base_path: string
  matches: GlobMatch[]
}

export interface FindMatch {
  path: string
  name: string
  synced: boolean  // true = in database, false = filesystem-only
  file_type: string
  size?: number | null
  updated_at: string
}

export interface FindResult {
  matches: FindMatch[]
}

export interface FileInfoResult {
  path: string
  file_type: string
  size?: number | null
  line_count?: number | null
  synced: boolean  // true = in database, false = filesystem-only
  created_at: string
  updated_at: string
  hash: string
}

export interface ReadFileResult {
  path: string
  success: boolean
  content?: any | null
  hash?: string | null
  synced: boolean  // true = in database, false = filesystem-only
  error?: string | null
  total_lines?: number | null
  truncated?: boolean | null
}

export interface ReadMultipleFilesResult {
  files: ReadFileResult[]
}

export interface CatFileEntry {
  path: string
  content: string
  line_count: number
  synced: boolean  // true = in database, false = filesystem-only
  offset?: number | null
  limit?: number | null
  total_lines?: number | null
}

export interface CatResult {
  content: string
  files: CatFileEntry[]
}

export interface RmResult {
  path: string
  file_id?: string | null  // null for filesystem-only files (not in database)
}

export interface WriteResult {
  path: string
  file_id: string
  version_id: string
  hash: string
}

export interface MkdirResult {
  path: string
  file_id?: string | null
}

export interface TouchResult {
  path: string
  file_id: string
}

export interface MvResult {
  from_path: string
  to_path: string
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

export type AiProvider = "openai" | "openrouter"

export interface ChatModelInfo {
  id: string              // "openai:gpt-4o"
  provider: AiProvider
  model: string           // "gpt-4o"
  display_name: string    // "GPT-4o"
  description?: string
  context_window?: number
}

export interface ProviderInfo {
  provider: AiProvider
  display_name: string    // "OpenAI" or "OpenRouter"
  configured: boolean
  models: ChatModelInfo[]
}

export interface ProvidersResponse {
  providers: ProviderInfo[]
  default_provider: AiProvider
}

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

export interface ChatMessageMetadata {
  message_type?: "reasoning_chunk" | "reasoning_complete" | "tool_call" | "tool_result"
  reasoning_id?: string
  tool_name?: string
  tool_arguments?: any
  tool_output?: string
  tool_success?: boolean
  question_answer?: {
    question_id: string
    answers: Record<string, any>
  }
}

export interface ChatMessage {
  id: string
  file_id: string
  workspace_id: string
  role: ChatMessageRole
  content: string
  metadata: ChatMessageMetadata
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

// ============================================================================
// Context API Response Types
// ============================================================================

export interface ChatContextResponse {
  system_prompt: SystemPromptSection
  history: HistorySection
  tools: ToolsSection
  attachments: AttachmentsSection
  summary: ContextSummary
}

export interface SystemPromptSection {
  content: string
  char_count: number
  token_count: number
  persona_type: string
  mode: string
}

export interface HistorySection {
  messages: HistoryMessageInfo[]
  message_count: number
  total_tokens: number
}

export interface HistoryMessageInfo {
  role: string
  content_preview: string
  content_length: number
  token_count: number
  metadata: HistoryMessageMetadata | null
  created_at: string
}

export interface HistoryMessageMetadata {
  message_type?: string
  reasoning_id?: string
  tool_name?: string
  model?: string
}

export interface ToolsSection {
  tools: ToolDefinition[]
  tool_count: number
  estimated_schema_tokens: number
}

export interface ToolDefinition {
  name: string
  description: string
  parameters: Record<string, any>
}

export interface AttachmentsSection {
  attachments: AttachmentInfo[]
  attachment_count: number
  total_tokens: number
}

export interface AttachmentInfo {
  attachment_type: string
  id: string
  content_preview: string
  content_length: number
  token_count: number
  priority: number
  is_essential: boolean
  // When the attachment was added to the context
  created_at: string
  // When the source content was last modified
  updated_at: string | null
}

export interface ContextSummary {
  total_tokens: number
  utilization_percent: number
  model: string
  token_limit: number
  breakdown: TokenBreakdown
}

export interface TokenBreakdown {
  system_prompt_tokens: number
  history_tokens: number
  tools_tokens: number
  attachments_tokens: number
}

// ============================================================================
// Agent Session Types
// ============================================================================

export type AgentType = 'assistant' | 'planner' | 'builder'

export type SessionStatus = 'idle' | 'running' | 'paused' | 'completed' | 'error'

export interface AgentSession {
  id: string
  workspace_id: string
  chat_id: string
  user_id: string
  agent_type: AgentType
  status: SessionStatus
  model: string
  mode: string
  current_task: string | null
  created_at: string
  updated_at: string
  last_heartbeat: string
  completed_at: string | null
  chat_name: string | null
}

export interface AgentSessionsListResponse {
  sessions: AgentSession[]
  total: number
}

export interface PauseSessionRequest {
  reason?: string
}

export interface ResumeSessionRequest {
  task?: string
}

export interface SessionActionResponse {
  session: AgentSession
  message: string
}

// ============================================================================
// Chat File Types
// ============================================================================

export interface ChatFile {
  id: string
  name: string
  path: string
  created_at: string
  updated_at: string
  chat_id: string
}
