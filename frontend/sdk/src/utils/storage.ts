/**
 * Token storage abstraction for different client types
 */

export interface TokenStorage {
  getAccessToken(): string | null
  getRefreshToken(): string | null
  setTokens(accessToken: string, refreshToken: string): void
  clearTokens(): void
}

/**
 * Browser storage using localStorage
 * For API clients, mobile apps, or manual token management
 */
export class BrowserTokenStorage implements TokenStorage {
  private ACCESS_TOKEN_KEY = 'buildscale_access_token'
  private REFRESH_TOKEN_KEY = 'buildscale_refresh_token'

  getAccessToken(): string | null {
    return localStorage.getItem(this.ACCESS_TOKEN_KEY)
  }

  getRefreshToken(): string | null {
    return localStorage.getItem(this.REFRESH_TOKEN_KEY)
  }

  setTokens(accessToken: string, refreshToken: string): void {
    localStorage.setItem(this.ACCESS_TOKEN_KEY, accessToken)
    localStorage.setItem(this.REFRESH_TOKEN_KEY, refreshToken)
  }

  clearTokens(): void {
    localStorage.removeItem(this.ACCESS_TOKEN_KEY)
    localStorage.removeItem(this.REFRESH_TOKEN_KEY)
  }
}

/**
 * Cookie storage placeholder
 * Note: Actual cookies are HttpOnly and managed by the backend
 * This class tracks cookie state for the client
 */
export class CookieTokenStorage implements TokenStorage {
  // Cookies are HttpOnly, managed by browser
  // This provides a consistent interface for the API client

  getAccessToken(): string | null {
    // Token is in HttpOnly cookie, managed by backend
    return null // Backend handles cookie-based auth
  }

  getRefreshToken(): string | null {
    return null // Backend handles cookie-based auth
  }

  setTokens(): void {
    // Cookies are set by backend via Set-Cookie header
    // No-op here
  }

  clearTokens(): void {
    // Cookies are cleared by backend via Set-Cookie header
    // No-op here
  }
}
