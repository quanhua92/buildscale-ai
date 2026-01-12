import { createFileRoute, Link } from '@tanstack/react-router'
import logo from '../../logo.svg'

export const Route = createFileRoute('/_auth/')({
  component: Dashboard,
})

function Dashboard() {
  return (
    <div className="text-center">
      <header className="min-h-screen flex flex-col items-center justify-center bg-[#282c34] text-white text-[calc(10px+2vmin)]">
        <img
          src={logo}
          className="h-[40vmin] pointer-events-none animate-[spin_20s_linear_infinite]"
          alt="logo"
        />
        <p>Admin Dashboard - Protected! ✅</p>
        <p>
          Admin Frontend - Edit <code>src/routes/_auth/index.tsx</code> and save to reload.
        </p>
        <Link to="/logout" className="text-[#61dafb] hover:underline mt-2">
          Logout
        </Link>
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
