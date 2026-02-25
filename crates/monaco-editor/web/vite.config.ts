import { defineConfig, type Plugin } from "vite";

/** Injects extracted CSS into the JS bundle as a <style> tag at runtime. */
function cssInjectedByJs(): Plugin {
  return {
    name: "css-injected-by-js",
    apply: "build",
    enforce: "post",
    generateBundle(_opts, bundle) {
      let cssCode = "";
      const cssKeys: string[] = [];

      for (const [key, chunk] of Object.entries(bundle)) {
        if (key.endsWith(".css") && chunk.type === "asset") {
          cssCode += chunk.source;
          cssKeys.push(key);
        }
      }

      for (const key of cssKeys) delete bundle[key];

      if (!cssCode) return;

      const escaped = cssCode.replace(/\\/g, "\\\\").replace(/`/g, "\\`").replace(/\$/g, "\\$");
      const injection = `(function(){const s=document.createElement("style");s.textContent=\`${escaped}\`;document.head.appendChild(s)})();`;

      for (const chunk of Object.values(bundle)) {
        if (chunk.type === "chunk" && chunk.isEntry) {
          chunk.code = injection + chunk.code;
          break;
        }
      }
    },
  };
}

export default defineConfig({
  plugins: [cssInjectedByJs()],
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
