import path from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
  resolve: {
    alias: {
      "@adi-family/plugin-command-palette/bus": path.resolve(
        __dirname,
        "../../command-palette/web/src/bus/index.ts",
      ),
    },
  },
  build: {
    outDir: "../../../dist/router",
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
