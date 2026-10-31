import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

const litReactForBrowsers = join(dirname(createRequire(import.meta.resolve('@slashome/lumen')).resolve('@lit/react')), '..', 'development', 'index.js')

export default defineConfig({
  plugins: [react()],
  base: '/',
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
    dedupe: ['react', 'react-dom', 'lit', '@lit/react'],
  },
  server: {
    proxy: { '/api': 'http://127.0.0.1:27317' },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            { name: 'react', test: /node_modules[\\/](react|react-dom|react-router|scheduler)[\\/]/ },
            { name: 'tanstack', test: /node_modules[\\/]@tanstack[\\/]/ },
            { name: 'radix', test: /node_modules[\\/]@radix-ui[\\/]/ },
            { name: 'dnd', test: /node_modules[\\/]@dnd-kit[\\/]/ },
            { name: 'lumen', test: /node_modules[\\/](@slashome[\\/]lumen|lit|@lit|@floating-ui|@atlaskit|marked|dompurify)[\\/]/ },
          ],
        },
      },
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/shared/config/test-setup.ts'],
    css: false,
    server: { deps: { inline: ['@slashome/lumen'] } },
    alias: [{ find: /^@lit\/react$/, replacement: litReactForBrowsers }],
  },
})
