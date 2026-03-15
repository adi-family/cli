import { defineConfig } from 'vite';
import { resolve } from 'node:path';

export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      formats: ['es'],
      fileName: () => 'web.js',
    },
    outDir: '../../../dist/adi.monaco-editor',
    emptyOutDir: true,
    minify: true,
    rollupOptions: {
      external: [
        '@adi-family/sdk-plugin',
        'lit',
        /^lit\//,
      ],
      output: {
        assetFileNames: 'web.[ext]',
        inlineDynamicImports: true,
      },
    },
  },
  worker: {
    format: 'es',
  },
});
