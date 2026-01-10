/**
 * Storage key constants
 * Single source of truth for all localStorage keys used in the app
 */

// Prefix used for all app data - enables clearing all on logout
export const STORAGE_PREFIX = 'buildscale_'

export const STORAGE_KEYS = {
  // Auth
  USER_ID: `${STORAGE_PREFIX}user_id`,

  // Theme
  THEME: `${STORAGE_PREFIX}theme`,

  // Settings
  LANGUAGE: `${STORAGE_PREFIX}language`,
  TIMEZONE: `${STORAGE_PREFIX}timezone`,

  // App State
  SIDEBAR_COLLAPSED: `${STORAGE_PREFIX}sidebar_collapsed`,
  ONBOARDING_COMPLETED: `${STORAGE_PREFIX}onboarding_completed`,
} as const

