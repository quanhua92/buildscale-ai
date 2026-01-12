import { Link } from '@tanstack/react-router'

import { useState } from 'react'
import { Home, LogOut, LogIn, UserPlus } from 'lucide-react'
import { NavigationMenu, ThemeToggle, useAuth } from '@buildscale/sdk'
import tanstackLogo from '/tanstack-word-logo-white.svg'

export default function Header() {
  const [isOpen, setIsOpen] = useState(false)
  const auth = useAuth()

  const handleLogout = () => {
    auth.logout()
  }

  return (
    <>
      <header className="p-4 flex items-center bg-background text-foreground border-b border-border shadow-sm">
        <NavigationMenu open={isOpen} onOpenChange={setIsOpen}>
          <NavigationMenu.Item to="/" icon={<Home size={20} />}>
            Home
          </NavigationMenu.Item>

          {auth.isAuthenticated ? (
            <>
              <NavigationMenu.Separator />

              <NavigationMenu.Section title="Workspaces" defaultOpen={true}>
                <NavigationMenu.Item to="/workspaces/all">
                  All Workspaces
                </NavigationMenu.Item>
              </NavigationMenu.Section>

              <NavigationMenu.Separator />

              <NavigationMenu.Item
                onClick={handleLogout}
                icon={<LogOut size={20} />}
              >
                Logout
              </NavigationMenu.Item>
            </>
          ) : (
            <>
              <NavigationMenu.Separator />

              <NavigationMenu.Item to="/login" icon={<LogIn size={20} />}>
                Login
              </NavigationMenu.Item>

              <NavigationMenu.Item to="/register" icon={<UserPlus size={20} />}>
                Register
              </NavigationMenu.Item>
            </>
          )}
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
