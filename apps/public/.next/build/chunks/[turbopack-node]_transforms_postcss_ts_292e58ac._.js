module.exports = [
"[turbopack-node]/transforms/postcss.ts { CONFIG => \"[project]/projects/adi-family/cli/apps/public/postcss.config.mjs [postcss] (ecmascript)\" } [postcss] (ecmascript, async loader)", ((__turbopack_context__) => {

__turbopack_context__.v((parentImport) => {
    return Promise.all([
  "build/chunks/d7be5_8fabbd51._.js",
  "build/chunks/[root-of-the-server]__3105375b._.js"
].map((chunk) => __turbopack_context__.l(chunk))).then(() => {
        return parentImport("[turbopack-node]/transforms/postcss.ts { CONFIG => \"[project]/projects/adi-family/cli/apps/public/postcss.config.mjs [postcss] (ecmascript)\" } [postcss] (ecmascript)");
    });
});
}),
];