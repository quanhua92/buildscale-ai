import { defineConfig } from 'vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

import { tanstackRouter } from '@tanstack/router-plugin/vite'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig({
  base: '/',
  plugins: [
    // NOTE: devtools disabled - focusing on admin project for now
    // TODO: investigate eventBusPort configuration for multi-project setup
    // devtools({ eventBusPort: 42070 }),
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    viteReact(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  optimizeDeps: {
    exclude: ['@buildscale/sdk'],
  },
  server: {
    proxy: {
      '/api/v1': {
        target: 'http://localhost:3000',
        changeOrigin: true,
        secure: false,
      },
    },
    watch: {
      ignored: ['!**/node_modules/@buildscale/sdk/dist/**'],
    },
  },
})
