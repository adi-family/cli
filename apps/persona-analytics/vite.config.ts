import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    target: 'es2021',
  },
  server: {
    port: parseInt(process.env.PORT || '5173'),
    host: true,
    allowedHosts: ['persona-analytics.adi.local'],
  },
});
