import { Link } from '@tanstack/react-router'

import { useState } from 'react'
import { Home } from 'lucide-react'
import { MobileMenu, ThemeToggle } from '@buildscale/sdk'
import { SheetClose } from '@buildscale/sdk'
import tanstackLogo from '/tanstack-word-logo-white.svg'

export default function Header() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <>
      <header className="p-4 flex items-center bg-background text-foreground border-b border-border shadow-sm">
        <MobileMenu open={isOpen} onOpenChange={setIsOpen}>
          <SheetClose asChild>
            <Link
              to="/"
              className="flex items-center gap-3 p-3 rounded-lg hover:bg-accent transition-colors mb-2"
              activeProps={{
                className:
                  'flex items-center gap-3 p-3 rounded-lg bg-primary hover:bg-primary/90 text-primary-foreground transition-colors mb-2',
              }}
            >
              <Home size={20} />
              <span className="font-medium">Home</span>
            </Link>
          </SheetClose>

          {/* Demo Links Start */}

          {/* Demo Links End */}
        </MobileMenu>

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
