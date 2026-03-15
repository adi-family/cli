import { defineConfig, build as viteBuild, type Plugin } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { dirname, resolve, join } from 'node:path';
import { createRequire } from 'node:module';
import {
  existsSync,
  createReadStream,
  readdirSync,
  mkdirSync,
  copyFileSync,
  writeFileSync,
} from 'node:fs';

const requireEnv = (name: string): string => {
  const value = process.env[name];
  if (!value) throw new Error(`${name} env variable is required`);
  return value;
};

const LIB_ROUTE = '/lib';

const SDK_DIST = dirname(
  createRequire(import.meta.url).resolve('@adi-family/sdk-plugin'),
);

const LIT_CACHE = resolve('node_modules/.cache/lit-bundle');

const LIT_ENTRIES: Record<string, string> = {
  'lit.js': 'lit',
  'lit-decorators.js': 'lit/decorators.js',
  'lit-unsafe-html.js': 'lit/directives/unsafe-html.js',
};

const buildLit = async () => {
  if (existsSync(join(LIT_CACHE, 'lit.js'))) return;
  mkdirSync(LIT_CACHE, { recursive: true });

  for (const [outFile, entry] of Object.entries(LIT_ENTRIES)) {
    const entryFile = join(LIT_CACHE, `_entry_${outFile.replace('.js', '.ts')}`);
    writeFileSync(entryFile, `export * from "${entry}";`);

    await viteBuild({
      configFile: false,
      logLevel: 'warn',
      build: {
        lib: { entry: entryFile, formats: ['es'], fileName: () => outFile },
        outDir: LIT_CACHE,
        emptyOutDir: false,
        minify: true,
        rollupOptions: { output: { inlineDynamicImports: true } },
      },
    });
  }
};

const sharedLibs = (): Plugin => {
  let litReady: Promise<void> | undefined;

  const ensureLitBuilt = () => {
    if (!litReady) litReady = buildLit();
    return litReady;
  };

  return {
    name: 'shared-libs',

    configureServer: (server) => {
      ensureLitBuilt();

      server.middlewares.use((req, res, next) => {
        if (!req.url?.startsWith(`${LIB_ROUTE}/`)) return next();
        const file = req.url.slice(LIB_ROUTE.length + 1);

        // Try sdk-plugin files — resolve .js to .ts for Vite transform
        if (file.startsWith('sdk-plugin/')) {
          const relative = file.replace('sdk-plugin/', '');
          const jsPath = join(SDK_DIST, relative);
          const tsPath = jsPath.replace(/\.js$/, '.ts');
          const resolved = existsSync(jsPath)
            ? jsPath
            : existsSync(tsPath)
              ? tsPath
              : null;
          if (resolved) {
            const moduleId = `/@fs/${resolved}`;
            server
              .transformRequest(moduleId)
              .then((result) => {
                if (!result) {
                  next();
                  return;
                }
                res.setHeader('Content-Type', 'application/javascript');
                res.setHeader('Access-Control-Allow-Origin', '*');
                res.end(result.code);
              })
              .catch(() => next());
            return;
          }
        }

        // Wait for lit build to finish before serving lit files
        ensureLitBuilt()
          .then(() => {
            const litPath = join(LIT_CACHE, file);
            if (existsSync(litPath)) {
              res.setHeader('Content-Type', 'application/javascript');
              res.setHeader('Access-Control-Allow-Origin', '*');
              createReadStream(litPath).pipe(res);
              return;
            }
            next();
          })
          .catch(() => next());
      });
    },

    async closeBundle() {
      await ensureLitBuilt();
      const out = resolve('dist', 'lib');

      // Copy sdk-plugin
      const sdkOut = join(out, 'sdk-plugin');
      mkdirSync(sdkOut, { recursive: true });
      for (const f of readdirSync(SDK_DIST).filter((f) => f.endsWith('.js'))) {
        copyFileSync(join(SDK_DIST, f), join(sdkOut, f));
      }

      // Copy lit bundles
      mkdirSync(out, { recursive: true });
      for (const f of readdirSync(LIT_CACHE).filter(
        (f) => f.endsWith('.js') && !f.startsWith('_'),
      )) {
        copyFileSync(join(LIT_CACHE, f), join(out, f));
      }
    },
  };
};

export default defineConfig({
  plugins: [tailwindcss(), sharedLibs()],
  resolve: {
    alias: {
      '@adi-family/plugin-debug-screen/bus': resolve(
        '../../crates/debug-screen/web/src/bus/index.ts',
      ),
    },
  },
  server: {
    port: parseInt(requireEnv('PORT')),
    host: true,
    allowedHosts: ['app.adi.test'],
  },
});
