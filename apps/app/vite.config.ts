import { defineConfig, build as viteBuild, type Plugin } from "vite";
import tailwindcss from "@tailwindcss/vite";
import { dirname, resolve, join } from "node:path";
import { createRequire } from "node:module";
import { existsSync, createReadStream, readdirSync, mkdirSync, copyFileSync, writeFileSync, readFileSync } from "node:fs";

const requireEnv = (name: string): string => {
  const value = process.env[name];
  if (!value) throw new Error(`${name} env variable is required`);
  return value;
};

const SDK_DIST = dirname(createRequire(import.meta.url).resolve("@adi-family/sdk-plugin"));
const SDK_ROUTE = "/vendor/sdk-plugin";

const sdkExternal = (): Plugin => ({
  name: "sdk-external",

  configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (!req.url?.startsWith(`${SDK_ROUTE}/`)) return next();
      const file = req.url.slice(SDK_ROUTE.length + 1);
      const filePath = join(SDK_DIST, file);
      if (!existsSync(filePath)) return next();
      res.setHeader("Content-Type", "application/javascript");
      createReadStream(filePath).pipe(res);
    });
  },

  closeBundle() {
    const out = resolve("dist", SDK_ROUTE.slice(1));
    mkdirSync(out, { recursive: true });
    for (const f of readdirSync(SDK_DIST).filter((f) => f.endsWith(".js"))) {
      copyFileSync(join(SDK_DIST, f), join(out, f));
    }
  },
});

const LIT_VENDOR_DIR = resolve("node_modules/.cache/lit-vendor");
const LIT_ROUTE = "/vendor/lit";

const LIT_ENTRIES: Record<string, string> = {
  "lit.js": "lit",
  "decorators.js": "lit/decorators.js",
};

const litVendor = (): Plugin => {
  let built = false;

  const ensureBuilt = async () => {
    if (built && existsSync(join(LIT_VENDOR_DIR, "lit.js"))) return;
    mkdirSync(LIT_VENDOR_DIR, { recursive: true });

    for (const [outFile, entry] of Object.entries(LIT_ENTRIES)) {
      const entryFile = join(LIT_VENDOR_DIR, `_entry_${outFile.replace(".js", ".ts")}`);
      writeFileSync(entryFile, `export * from "${entry}";`);

      await viteBuild({
        configFile: false,
        logLevel: "warn",
        build: {
          lib: { entry: entryFile, formats: ["es"], fileName: () => outFile },
          outDir: LIT_VENDOR_DIR,
          emptyOutDir: false,
          minify: true,
          rollupOptions: { output: { inlineDynamicImports: true } },
        },
      });
    }
    built = true;
  };

  return {
    name: "lit-vendor",

    async configureServer(server) {
      await ensureBuilt();
      server.middlewares.use((req, res, next) => {
        if (!req.url?.startsWith(`${LIT_ROUTE}/`)) return next();
        const file = req.url.slice(LIT_ROUTE.length + 1);
        const filePath = join(LIT_VENDOR_DIR, file);
        if (!existsSync(filePath)) return next();
        res.setHeader("Content-Type", "application/javascript");
        createReadStream(filePath).pipe(res);
      });
    },

    async closeBundle() {
      await ensureBuilt();
      const out = resolve("dist", LIT_ROUTE.slice(1));
      mkdirSync(out, { recursive: true });
      for (const f of readdirSync(LIT_VENDOR_DIR).filter((f) => f.endsWith(".js") && !f.startsWith("_"))) {
        copyFileSync(join(LIT_VENDOR_DIR, f), join(out, f));
      }
    },
  };
};

export default defineConfig({
  plugins: [tailwindcss(), sdkExternal(), litVendor()],
  server: {
    port: parseInt(requireEnv("PORT")),
    host: true,
    allowedHosts: ['app.adi.test'],
  },
});
