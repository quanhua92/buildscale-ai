import { createFileRoute } from '@tanstack/react-router'
import { Auth, useAuthRedirects } from '@buildscale/sdk'

export const Route = createFileRoute('/login')({
  component: Login,
})

function Login() {
  useAuthRedirects()

  return (
    <Auth>
      <Auth.Login />
    </Auth>
  )
}
