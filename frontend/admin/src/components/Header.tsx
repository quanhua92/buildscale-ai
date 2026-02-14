import { Link, useMatches } from '@tanstack/react-router'

import { useState } from 'react'
import { Home, LogOut, LogIn, UserPlus, LayoutDashboard, Images, Users, File, Settings, MessageSquare, Trash2, Brain } from 'lucide-react'
import { NavigationMenu, ThemeToggle, useAuth } from '@buildscale/sdk'
import tanstackLogo from '/tanstack-word-logo-white.svg'

export default function Header() {
  const [isOpen, setIsOpen] = useState(false)
  const auth = useAuth()
  const matches = useMatches()
  
  // Find workspaceId in route params
  const workspaceMatch = matches.find((m) => m.params && 'workspaceId' in m.params);
  const workspaceId = (workspaceMatch?.params as { workspaceId?: string })?.workspaceId;

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

              {workspaceId && (
                <>
                  <NavigationMenu.Section title="Current Workspace" defaultOpen={true}>
                    <NavigationMenu.Item 
                      to="/workspaces/$workspaceId" 
                      params={{ workspaceId }}
                      icon={<LayoutDashboard size={20} />}
                    >
                      Overview
                    </NavigationMenu.Item>
                    <NavigationMenu.Item 
                      to="/workspaces/$workspaceId/chat" 
                      params={{ workspaceId }}
                      search={{}} // Explicitly clear search params to start a new chat
                      icon={<MessageSquare size={20} />}
                    >
                      Chat
                    </NavigationMenu.Item>
                    <NavigationMenu.Item
                      to="/workspaces/$workspaceId/files"
                      params={{ workspaceId }}
                      icon={<File size={20} />}
                    >
                      Files
                    </NavigationMenu.Item>
                    <NavigationMenu.Item
                      to="/workspaces/$workspaceId/memories"
                      params={{ workspaceId }}
                      icon={<Brain size={20} />}
                    >
                      Memories
                    </NavigationMenu.Item>
                    <NavigationMenu.Item
                      disabled
                      icon={<Images size={20} />}
                    >
                      Images
                    </NavigationMenu.Item>
                    <NavigationMenu.Item 
                      disabled 
                      icon={<Users size={20} />}
                    >
                      Members
                    </NavigationMenu.Item>
                    <NavigationMenu.Item 
                      to="/workspaces/$workspaceId/settings" 
                      params={{ workspaceId }}
                      icon={<Settings size={20} />}
                    >
                      Settings
                    </NavigationMenu.Item>
                    <NavigationMenu.Item 
                      to="/workspaces/$workspaceId/deleted" 
                      params={{ workspaceId }}
                      icon={<Trash2 size={20} />}
                    >
                      Recently Deleted
                    </NavigationMenu.Item>
                  </NavigationMenu.Section>
                  <NavigationMenu.Separator />
                </>
              )}

              <NavigationMenu.Section title="Workspaces" defaultOpen={true}>
                <NavigationMenu.Item to="/workspaces/all">
                  All Workspaces
                </NavigationMenu.Item>
                <NavigationMenu.Item to="/workspaces/new">
                  Create Workspace
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
