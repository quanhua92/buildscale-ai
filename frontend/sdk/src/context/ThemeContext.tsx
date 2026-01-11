/**
 * Theme context provider for managing dark/light mode state
 */

import { createContext, useContext, useState, useEffect, useCallback, useMemo, type ReactNode } from 'react'
import { useStorage } from './StorageContext'
import { STORAGE_KEYS } from '../utils/constants'

export type Theme = 'light' | 'dark' | 'system'

interface ThemeContextType {
  theme: Theme
  setTheme: (theme: Theme) => void
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined)

export interface ThemeProviderProps {
  children: ReactNode
  defaultTheme?: Theme
}

export function ThemeProvider({ children, defaultTheme = 'system' }: ThemeProviderProps) {
  const { getItem, setItem } = useStorage()
  const [theme, setThemeState] = useState<Theme>(() => {
    // Initialize from localStorage during SSR
    if (typeof window === 'undefined') {
      return defaultTheme
    }
    try {
      const savedTheme = getItem(STORAGE_KEYS.THEME)
      return (savedTheme as Theme) || defaultTheme
    } catch {
      return defaultTheme
    }
  })

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme)
    try {
      setItem(STORAGE_KEYS.THEME, newTheme)
    } catch (error) {
      console.warn('Failed to save theme preference:', error)
    }
  }, [setItem])

  // Apply theme class to document element
  useEffect(() => {
    const root = document.documentElement

    // Remove existing theme classes
    root.classList.remove('light', 'dark')

    // Determine the actual theme to apply
    const applyTheme = (themeValue: Theme) => {
      if (themeValue === 'system') {
        const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
        root.classList.add(systemPrefersDark ? 'dark' : 'light')
      } else {
        root.classList.add(themeValue)
      }
    }

    applyTheme(theme)

    // Listen for system theme changes when theme is 'system'
    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
      const handleChange = () => applyTheme('system')
      mediaQuery.addEventListener('change', handleChange)
      return () => mediaQuery.removeEventListener('change', handleChange)
    }
  }, [theme])

  const value: ThemeContextType = useMemo(() => ({
    theme,
    setTheme,
  }), [theme, setTheme])

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useTheme() {
  const context = useContext(ThemeContext)
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider')
  }
  return context
}

/**
 * Hook to get the resolved theme (light or dark, not 'system')
 */
export function useResolvedTheme() {
  const { theme } = useTheme()
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>(() => {
    if (theme === 'system') {
      if (typeof window === 'undefined') return 'light'
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return theme as 'light' | 'dark'
  })

  useEffect(() => {
    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
      const updateResolvedTheme = () => {
        setResolvedTheme(mediaQuery.matches ? 'dark' : 'light')
      }
      updateResolvedTheme()
      mediaQuery.addEventListener('change', updateResolvedTheme)
      return () => mediaQuery.removeEventListener('change', updateResolvedTheme)
    } else {
      setResolvedTheme(theme as 'light' | 'dark')
    }
  }, [theme])

  return { theme, resolvedTheme, isDark: resolvedTheme === 'dark' }
}
