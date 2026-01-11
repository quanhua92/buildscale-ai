import { Moon, Sun, Monitor } from 'lucide-react'
import { useTheme } from '../../context/ThemeContext'
import { Button } from './button'

export function ThemeToggle() {
  const { theme, setTheme } = useTheme()

  const cycleTheme = () => {
    if (theme === 'light') {
      setTheme('dark')
    } else if (theme === 'dark') {
      setTheme('system')
    } else {
      setTheme('light')
    }
  }

  const getIcon = () => {
    switch (theme) {
      case 'light':
        return <Sun size={18} />
      case 'dark':
        return <Moon size={18} />
      case 'system':
        return <Monitor size={18} />
    }
  }

  const getLabel = () => {
    switch (theme) {
      case 'light':
        return 'Light'
      case 'dark':
        return 'Dark'
      case 'system':
        return 'System'
    }
  }

  const getNextLabel = () => {
    switch (theme) {
      case 'light':
        return 'Dark'
      case 'dark':
        return 'System'
      case 'system':
        return 'Light'
    }
  }

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={cycleTheme}
      aria-label={`Current theme: ${getLabel()}. Click to switch to ${getNextLabel()} theme.`}
      title={`Theme: ${getLabel()} (Click to change to ${getNextLabel()})`}
      className="relative"
    >
      {getIcon()}
      <span className="sr-only">{getLabel()} theme</span>
    </Button>
  )
}
