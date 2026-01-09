import { createFileRoute } from '@tanstack/react-router'
import { Auth } from '@buildscale/sdk'

export const Route = createFileRoute('/register')({
  component: Register,
})

function Register() {
  return (
    <Auth>
      <Auth.Register />
    </Auth>
  )
}
