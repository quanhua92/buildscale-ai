/**
 * Safe localStorage wrapper with error handling
 * Prevents crashes when localStorage is blocked or unavailable
 */

let isLocalStorageAvailable = false

// Test localStorage availability on module load
try {
  const testKey = '__localStorage_test__'
  localStorage.setItem(testKey, 'test')
  localStorage.removeItem(testKey)
  isLocalStorageAvailable = true
} catch {
  isLocalStorageAvailable = false
}

export const safeLocalStorage = {
  get available(): boolean {
    return isLocalStorageAvailable
  },

  getItem(key: string): string | null {
    if (!isLocalStorageAvailable) return null
    try {
      return localStorage.getItem(key)
    } catch {
      return null
    }
  },

  setItem(key: string, value: string): void {
    if (!isLocalStorageAvailable) return
    try {
      localStorage.setItem(key, value)
    } catch (error) {
      console.warn('Failed to save to localStorage:', error)
    }
  },

  removeItem(key: string): void {
    if (!isLocalStorageAvailable) return
    try {
      localStorage.removeItem(key)
    } catch (error) {
      console.warn('Failed to remove from localStorage:', error)
    }
  },

  clear(): void {
    if (!isLocalStorageAvailable) return
    try {
      localStorage.clear()
    } catch (error) {
      console.warn('Failed to clear localStorage:', error)
    }
  },
}
