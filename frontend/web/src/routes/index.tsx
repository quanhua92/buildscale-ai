import { createFileRoute, Link } from '@tanstack/react-router'
import { useAuth } from '@buildscale/sdk'
import logo from '../logo.svg'

export const Route = createFileRoute('/')({
  component: App,
})

function App() {
  const { user, isAuthenticated } = useAuth()

  return (
    <div className="text-center">
      <header className="min-h-screen flex flex-col items-center justify-center bg-[#282c34] text-white text-[calc(10px+2vmin)]">
        <img
          src={logo}
          className="h-[40vmin] pointer-events-none animate-[spin_20s_linear_infinite]"
          alt="logo"
        />
        <p>
          {isAuthenticated && user ? `Auth: ${user.id}` : 'Auth: unauth'}
        </p>
        {!isAuthenticated && (
          <Link to="/login" className="text-[#61dafb] hover:underline mt-2">
            Go to Login
          </Link>
        )}
        {isAuthenticated && (
          <Link to="/logout" className="text-[#61dafb] hover:underline mt-2">
            Logout
          </Link>
        )}
        <p>
          Web Frontend - Edit <code>src/routes/index.tsx</code> and save to reload.
        </p>
        <a
          className="text-[#61dafb] hover:underline"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
        <a
          className="text-[#61dafb] hover:underline"
          href="https://tanstack.com"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn TanStack
        </a>
      </header>
    </div>
  )
}
