/**
 * API error types for authentication and token management
 */

export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

export class TokenTheftError extends ApiError {
  constructor(message: string) {
    super(message, 403, 'TOKEN_THEFT')
    this.name = 'TokenTheftError'
  }
}
