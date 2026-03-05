import path from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
  resolve: {
    alias: {
      "@adi/command-palette-web-plugin/bus": path.resolve(
        __dirname,
        "../../command-palette/web/src/bus/index.ts",
      ),
    },
  },
  build: {
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
