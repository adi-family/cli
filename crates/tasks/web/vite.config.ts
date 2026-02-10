import { defineConfig } from "vite";

export default defineConfig({
  build: {
    lib: {
      entry: "src/index.ts",
      formats: ["es"],
      fileName: () => "web.js",
    },
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
      },
    },
    target: "es2022",
    minify: true,
  },
});
