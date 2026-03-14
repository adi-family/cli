import path from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
  resolve: {
    alias: {
      "@adi-family/plugin-actions-feed/bus": path.resolve(
        __dirname,
        "../../actions-feed/web/src/bus/index.ts",
      ),
      "@adi-family/plugin-debug-screen/bus": path.resolve(
        __dirname,
        "../../debug-screen/web/src/bus/index.ts",
      ),
    },
  },
  build: {
    outDir: "../../../../dist/adi.signaling",
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
