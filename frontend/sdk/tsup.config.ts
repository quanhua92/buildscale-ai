import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  external: ['react', 'react-dom', '@tanstack/react-router', 'lucide-react'],
  esbuildOptions(options) {
    options.banner = {
      js: '// @buildscale/sdk - Authentication SDK',
    }
  },
})
