import { createFileRoute } from '@tanstack/react-router'
import { Auth, useAuthRedirects } from '@buildscale/sdk'

export const Route = createFileRoute('/register')({
  component: Register,
})

function Register() {
  useAuthRedirects()

  return (
    <Auth>
      <Auth.Register />
    </Auth>
  )
}
