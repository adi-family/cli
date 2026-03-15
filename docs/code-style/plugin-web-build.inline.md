- **Web plugin codegen** (`lib-plugin-web-build`): build.rs generates TypeScript from Cargo.toml metadata + optional `.tsp` eventbus
  - Generates: `config.ts`, `index.ts` (PluginApiRegistry augmentation), `bus-types.ts` + `bus-events.ts` (from `.tsp`)
  - 3-line build.rs:
    ```rust
    fn main() {
        lib_plugin_web_build::PluginWebBuild::new().run();
    }
    ```
  - Cargo.toml build-dep:
    ```toml
    [build-dependencies]
    lib-plugin-web-build = { path = "../../_lib/lib-plugin-web-build" }
    ```
  - Required metadata in Cargo.toml: `[package.metadata.plugin]` with `id`, `name`, `type`
  - Optional: `[package.metadata.plugin.web_ui]` with `plugin_class` (otherwise derived from name: "ADI Router" → "RouterPlugin")
  - Place `api.tsp` at `crates/<component>/api.tsp` for eventbus generation (skipped if absent)
  - Builder overrides: `.tsp_path()`, `.output_dir()`, `.bus_subdir()`, `.plugin_class()`
  - Defaults: tsp=`../api.tsp`, output=`../web/src`, bus files go into the `generated/` output dir
  - Trigger from npm: `"generate": "cargo check -p <plugin-crate>"`
- **Cross-plugin bus type imports**: when a web plugin needs bus types from another plugin (e.g. router using command-palette bus):
  - Add the dependency in `package.json`: `"@adi-family/<dep>": "workspace:*"`
  - In the dependency's `package.json`, add `main`, `types`, and `exports` with a `./bus` subpath:
    ```json
    "main": "src/index.ts",
    "types": "src/index.ts",
    "exports": {
      ".": "./src/index.ts",
      "./bus": "./src/bus/index.ts"
    }
    ```
  - **Always import from the `/bus` subpath** to avoid bundling the plugin's UI dependencies (e.g. Lit):
    ```ts
    import { SomeBusKey } from '@adi-family/<dep>/bus';
    ```
  - **Vite resolve alias required**: rolldown-vite does not resolve `exports` subpaths through workspace symlinks. Add a resolve alias in `vite.config.ts`:
    ```ts
    resolve: {
      alias: {
        "@adi-family/<dep>/bus": resolve("../../<dep-path>/web/src/bus/index.ts"),
      },
    },
    ```
  - No `tsconfig.json` paths needed — `moduleResolution: "bundler"` resolves via `node_modules` + package.json `exports` field
- **Externalize shared dependencies in vite**: plugins must not bundle shared libs (Lit, sdk-plugin) — they are provided at runtime by the host app
  - `@adi-family/sdk-plugin` is always external
  - `lit` and its subpaths (`lit/decorators.js`, etc.) must be externalized with a regex:
    ```ts
    rollupOptions: {
      external: ["@adi-family/sdk-plugin", /^lit(\/.*)?$/],
    },
    ```
