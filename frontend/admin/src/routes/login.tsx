import { createFileRoute } from '@tanstack/react-router'
import { Auth } from '@buildscale/sdk'

export const Route = createFileRoute('/login')({
  component: Login,
})

function Login() {
  return (
    <Auth>
      <Auth.Login />
    </Auth>
  )
}
