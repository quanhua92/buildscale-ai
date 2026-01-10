/**
 * Storage key constants
 * Single source of truth for all localStorage keys used in the app
 */

export const STORAGE_KEYS = {
  // Auth
  USER_ID: 'buildscale_user_id',

  // Theme
  THEME: 'buildscale_theme',

  // Settings
  LANGUAGE: 'buildscale_language',
  TIMEZONE: 'buildscale_timezone',

  // App State
  SIDEBAR_COLLAPSED: 'buildscale_sidebar_collapsed',
  ONBOARDING_COMPLETED: 'buildscale_onboarding_completed',
} as const

// Prefix used for all app data - enables clearing all on logout
export const STORAGE_PREFIX = 'buildscale_'
