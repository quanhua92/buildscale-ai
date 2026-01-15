/**
 * API type definitions matching backend API exactly
 * @see backend/docs/REST_API_GUIDE.md
 */

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
