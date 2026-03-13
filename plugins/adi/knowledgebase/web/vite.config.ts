import { defineConfig } from "vite";

export default defineConfig({
  build: {
    outDir: "../../../../dist/adi.knowledgebase",
    lib: {
      entry: "src/index.ts",
      formats: ["es"],
      fileName: () => "web.js",
    },
    rollupOptions: {
      external: ["@adi-family/sdk-plugin"],
      output: {
        inlineDynamicImports: true,
        assetFileNames: "style[extname]",
      },
    },
    target: "es2022",
    minify: true,
  },
});
