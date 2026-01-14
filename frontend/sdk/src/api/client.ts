/**
 * API client with automatic token refresh on 401 responses
 * Implements token rotation with stolen token detection
 */

import type {
  RegisterRequest,
  LoginRequest,
  AuthResponse,
  RefreshTokenResponse,
  User,
  CreateWorkspaceResponse,
} from './types'
import { ApiError, TokenTheftError } from './errors'

interface ApiClientConfig {
  baseURL: string
  timeout?: number
  // Token callbacks - passed directly from useStorage()
  getAccessToken: () => string | null | Promise<string | null>
  getRefreshToken: () => string | null | Promise<string | null>
  setTokens: (accessToken: string, refreshToken: string) => void | Promise<void>
  clearTokens: () => void | Promise<void>
}

class ApiClient {
  private baseURL: string
  private timeout: number
  private getAccessToken: () => string | null | Promise<string | null>
  private getRefreshToken: () => string | null | Promise<string | null>
  private setTokens: (access: string, refresh: string) => void | Promise<void>
  private clearTokens: () => void | Promise<void>
  private isRefreshing = false
  private refreshPromise: Promise<string | null> | null = null
  private noRefreshEndpoints: string[] = [
    '/auth/login',
    '/auth/register',
    '/auth/logout',
    '/auth/refresh',
  ]

  constructor(config: ApiClientConfig) {
    this.baseURL = config.baseURL
    this.timeout = config.timeout || 10000
    this.getAccessToken = config.getAccessToken
    this.getRefreshToken = config.getRefreshToken
    this.setTokens = config.setTokens
    this.clearTokens = config.clearTokens
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseURL}${endpoint}`
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.timeout)

    try {
      let headers: HeadersInit = {
        'Content-Type': 'application/json',
        ...options.headers,
      }

      // Add authorization header if access token exists
      const accessToken = await this.getAccessToken()
      if (accessToken) {
        headers = {
          ...headers,
          Authorization: `Bearer ${accessToken}`,
        }
      }

      const response = await this.fetchWithAuth(url, options, headers, controller.signal)

      clearTimeout(timeoutId)

      // Handle 401 - try token refresh (except for auth endpoints)
      if (response.status === 401 && !this.shouldSkipRefresh(endpoint)) {
        const newToken = await this.refreshAccessToken()
        if (newToken) {
          // Retry original request with new token
          const retryHeaders = {
            ...headers,
            Authorization: `Bearer ${newToken}`,
          }
          const retryResponse = await this.fetchWithAuth(url, options, retryHeaders, controller.signal)
          return this.handleResponse<T>(retryResponse)
        }
      }

      return this.handleResponse<T>(response)
    } catch (error) {
      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new ApiError('Request timeout', 408)
        }
      }
      throw error
    }
  }

  private async fetchWithAuth(
    url: string,
    options: RequestInit,
    headers: HeadersInit,
    signal: AbortSignal
  ): Promise<Response> {
    return fetch(url, {
      ...options,
      headers,
      signal,
      credentials: 'include', // Include cookies for browser clients
    })
  }

  private shouldSkipRefresh(endpoint: string): boolean {
    return this.noRefreshEndpoints.some(path => endpoint.includes(path))
  }

  private async handleResponse<T>(response: Response): Promise<T> {
    const text = await response.text()
    let data: unknown
    try {
      data = text ? JSON.parse(text) : {}
    } catch (error) {
      // If parsing fails, throw an error with the raw text
      throw new ApiError(`Failed to parse server response: ${text}`, response.status)
    }

    if (!response.ok) {
      const errorData = data as { code?: string; message?: string; error?: string; fields?: Record<string, string> }
      // Handle token theft detection (403 from refresh endpoint)
      if (response.status === 403 && errorData.code === 'TOKEN_THEFT') {
        throw new TokenTheftError(errorData.message || 'Token theft detected')
      }

      throw new ApiError(
        errorData.error || errorData.message || 'Request failed',
        response.status,
        errorData.code,
        errorData.fields  // Extract field-specific errors from backend
      )
    }

    return data as T
  }

  private refreshAccessToken(): Promise<string | null> {
    // Prevent multiple refresh attempts by returning the existing promise
    if (this.isRefreshing && this.refreshPromise) {
      return this.refreshPromise
    }

    // Set flag synchronously to prevent race condition
    this.isRefreshing = true

    // Check if we have a stored refresh token (token-based auth)
    const storedRefreshToken = this.getRefreshToken()

    this.refreshPromise = (async () => {
      try {
        // For token-based auth: use the stored token
        // For cookie-based auth: storedRefreshToken will be null, browser sends cookie automatically
        const token = storedRefreshToken ? await storedRefreshToken : null
        const result = await this.performRefresh(token)
        return result.access_token
      } catch (error) {
        // Clear tokens on refresh failure
        await this.clearTokens()
        throw error
      } finally {
        // Reset state for the next refresh cycle
        this.isRefreshing = false
        this.refreshPromise = null
      }
    })()

    return this.refreshPromise
  }

  private async performRefresh(
    refreshToken: string | null
  ): Promise<RefreshTokenResponse> {
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.timeout)

    try {
      const headers: HeadersInit = {
        'Content-Type': 'application/json',
      }

      // Add Authorization header only if we have a token (API/Mobile clients)
      if (refreshToken) {
        headers['Authorization'] = `Bearer ${refreshToken}`
      }

      const response = await fetch(`${this.baseURL}/auth/refresh`, {
        method: 'POST',
        headers,
        credentials: 'include', // Browser sends cookies automatically
        signal: controller.signal,
      })

      const data = await response.json()

      if (!response.ok) {
        // Token theft detected
        if (response.status === 403) {
          throw new TokenTheftError(data.message || 'Token theft detected')
        }
        throw new ApiError(data.message || 'Refresh failed', response.status)
      }

      // Token rotation: store new refresh token
      if (data.refresh_token) {
        // New refresh token provided (normal rotation)
        await this.setTokens(data.access_token, data.refresh_token)
      } else if (refreshToken) {
        // Grace period: keep existing refresh token
        await this.setTokens(data.access_token, refreshToken)
      } else {
        // Cookie auth: backend handles storage via HttpOnly cookies
        // Frontend can't access HttpOnly cookies
      }

      return data
    } finally {
      clearTimeout(timeoutId)
    }
  }

  // ============================================================================
  // Public API Methods
  // ============================================================================

  async register(data: RegisterRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('/auth/register', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async login(data: LoginRequest): Promise<AuthResponse> {
    return this.request<AuthResponse>('/auth/login', {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async logout(): Promise<{ message: string }> {
    const refreshToken = await this.getRefreshToken()
    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.timeout)

    try {
      const headers: HeadersInit = { 'Content-Type': 'application/json' }

      // Add Authorization header if token exists (API/Mobile clients)
      if (refreshToken) {
        headers['Authorization'] = `Bearer ${refreshToken}`
      }

      const response = await fetch(`${this.baseURL}/auth/logout`, {
        method: 'POST',
        headers,
        credentials: 'include', // Always include for cookie-based auth
        signal: controller.signal,
      })

      if (!response.ok) {
        const data = await response.json()
        throw new ApiError(data.message || 'Logout failed', response.status)
      }

      return await response.json()
    } finally {
      clearTimeout(timeoutId)
      // Always clear local tokens, regardless of API call success
      await this.clearTokens()
    }
  }

  async refreshToken(): Promise<RefreshTokenResponse> {
    const refreshToken = await this.getRefreshToken()
    if (!refreshToken) {
      throw new ApiError('No refresh token available', 401)
    }
    return this.performRefresh(refreshToken)
  }

  async getProfile(): Promise<{ user: User }> {
    return this.request<{ user: User }>('/auth/me')
  }

  async createWorkspace(name: string): Promise<CreateWorkspaceResponse> {
    return this.request<CreateWorkspaceResponse>('/workspaces', {
      method: 'POST',
      body: JSON.stringify({ name }),
    })
  }

  // Generic methods for other API calls
  async get<T>(endpoint: string): Promise<T> {
    return this.request<T>(endpoint, { method: 'GET' })
  }

  async post<T>(endpoint: string, data: unknown): Promise<T> {
    return this.request<T>(endpoint, {
      method: 'POST',
      body: JSON.stringify(data),
    })
  }

  async put<T>(endpoint: string, data: unknown): Promise<T> {
    return this.request<T>(endpoint, {
      method: 'PUT',
      body: JSON.stringify(data),
    })
  }

  async delete<T>(endpoint: string): Promise<T> {
    return this.request<T>(endpoint, { method: 'DELETE' })
  }
}

export default ApiClient
