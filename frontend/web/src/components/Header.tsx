import { Link } from '@tanstack/react-router'

import { useState } from 'react'
import { Home } from 'lucide-react'
import { NavigationMenu, ThemeToggle } from '@buildscale/sdk'
import tanstackLogo from '/tanstack-word-logo-white.svg'

export default function Header() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <>
      <header className="p-4 flex items-center bg-background text-foreground border-b border-border shadow-sm">
        <NavigationMenu open={isOpen} onOpenChange={setIsOpen}>
          <NavigationMenu.Item to="/" icon={<Home size={20} />}>
            Home
          </NavigationMenu.Item>
        </NavigationMenu>

        <h1 className="ml-4 text-xl font-semibold">
          <Link to="/">
            <img
              src={tanstackLogo}
              alt="TanStack Logo"
              className="h-10 dark:invert-0 invert"
            />
          </Link>
        </h1>
        <div className="ml-auto">
          <ThemeToggle />
        </div>
      </header>
    </>
  )
}
