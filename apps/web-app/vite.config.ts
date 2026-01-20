import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [tailwindcss()],
  base: '/app/',
  server: {
    port: parseInt(process.env.PORT || '5173'),
    host: true,
    allowedHosts: ['adi.local'],
  },
})
