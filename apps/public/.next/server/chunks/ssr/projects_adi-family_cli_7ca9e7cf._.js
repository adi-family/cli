module.exports = [
"[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/loading-skeleton.js [app-ssr] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "AdiLoadingSkeleton",
    ()=>AdiLoadingSkeleton
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2f$index$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit/index.js [app-ssr] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-element/development/lit-element.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/node/development/lit-html.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2f$decorators$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit/decorators.js [app-ssr] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$custom$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/node/development/decorators/custom-element.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/node/development/decorators/property.js [app-ssr] (ecmascript)");
var __decorate = ("TURBOPACK compile-time value", void 0) && ("TURBOPACK compile-time value", void 0).__decorate || function(decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for(var i = decorators.length - 1; i >= 0; i--)if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
;
;
/// Shimmer placeholder skeleton. Sizing via ADID AX system (--l, --t, --r).
let AdiLoadingSkeleton = class AdiLoadingSkeleton extends __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["LitElement"] {
    constructor(){
        super(...arguments);
        this.label = "";
        this.variant = "card";
    }
    createRenderRoot() {
        return this;
    }
    renderCard() {
        // Card: 9.375 x 6.25 --l units
        return __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["html"]`
      <div style="
        width: calc(var(--l) * 9.375);
        height: calc(var(--l) * 6.25);
        background: var(--adi-surface);
        border-radius: var(--r);
        overflow: hidden;
        position: relative;
      " class="skeleton-shimmer">
        <div style="padding:calc(var(--l) * 0.75);display:flex;flex-direction:column;gap:calc(var(--l) * 0.5);height:100%;box-sizing:border-box;">
          <div style="display:flex;gap:calc(var(--l) * 0.625);align-items:center;">
            <div style="width:calc(var(--l) * 1.5);height:calc(var(--l) * 1.5);border-radius:50%;background:var(--adi-surface-alt);flex-shrink:0;"></div>
            <div style="height:calc(var(--t) * 0.75);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);flex:1;"></div>
          </div>
          <div style="height:calc(var(--t) * 0.5);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:80%;"></div>
          <div style="height:calc(var(--t) * 0.5);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:60%;"></div>
        </div>
      </div>
    `;
    }
    renderText() {
        // Text: 9.375 --l wide, three lines
        return __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["html"]`
      <div style="
        width: calc(var(--l) * 9.375);
        display: flex;
        flex-direction: column;
        gap: calc(var(--l) * 0.5);
        position: relative;
        overflow: hidden;
        border-radius: var(--r);
      " class="skeleton-shimmer">
        <div style="height:calc(var(--t) * 0.625);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:100%;"></div>
        <div style="height:calc(var(--t) * 0.625);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:90%;"></div>
        <div style="height:calc(var(--t) * 0.625);background:var(--adi-surface-alt);border-radius:calc(var(--r) * 0.5);width:70%;"></div>
      </div>
    `;
    }
    renderAvatar() {
        // Avatar: 4 * --l diameter
        return __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["html"]`
      <div style="
        width: calc(var(--l) * 4);
        height: calc(var(--l) * 4);
        border-radius: 50%;
        background: var(--adi-surface);
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
        overflow: hidden;
      " class="skeleton-shimmer">
        <div style="width:70%;height:70%;border-radius:50%;background:var(--adi-surface-alt);"></div>
      </div>
    `;
    }
    render() {
        let content;
        switch(this.variant){
            case "text":
                content = this.renderText();
                break;
            case "avatar":
                content = this.renderAvatar();
                break;
            default:
                content = this.renderCard();
        }
        return __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["html"]`
      <div style="display:flex;flex-direction:column;align-items:center;gap:calc(var(--l) * 0.75);">
        ${content}
        ${this.label ? __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["html"]`<span style="font-size:calc(var(--t) * 0.875);color:var(--adi-text-muted);">${this.label}</span>` : ""}
      </div>
    `;
    }
};
__decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["property"])({
        type: String
    })
], AdiLoadingSkeleton.prototype, "label", void 0);
__decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["property"])({
        type: String
    })
], AdiLoadingSkeleton.prototype, "variant", void 0);
AdiLoadingSkeleton = __decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$custom$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["customElement"])("adi-loading-skeleton")
], AdiLoadingSkeleton);
;
 //# sourceMappingURL=loading-skeleton.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/under-construction.js [app-ssr] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "AdiUnderConstruction",
    ()=>AdiUnderConstruction
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2f$index$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit/index.js [app-ssr] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-element/development/lit-element.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/node/development/lit-html.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2f$decorators$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit/decorators.js [app-ssr] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$custom$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/node/development/decorators/custom-element.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/node/development/decorators/property.js [app-ssr] (ecmascript)");
var __decorate = ("TURBOPACK compile-time value", void 0) && ("TURBOPACK compile-time value", void 0).__decorate || function(decorators, target, key, desc) {
    var c = arguments.length, r = c < 3 ? target : desc === null ? desc = Object.getOwnPropertyDescriptor(target, key) : desc, d;
    if (typeof Reflect === "object" && typeof Reflect.decorate === "function") r = Reflect.decorate(decorators, target, key, desc);
    else for(var i = decorators.length - 1; i >= 0; i--)if (d = decorators[i]) r = (c < 3 ? d(r) : c > 3 ? d(target, key, r) : d(target, key)) || r;
    return c > 3 && r && Object.defineProperty(target, key, r), r;
};
;
;
/// Placeholder for pages or sections not yet built.
let AdiUnderConstruction = class AdiUnderConstruction extends __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["LitElement"] {
    constructor(){
        super(...arguments);
        this.heading = "Under Construction";
        this.description = "This section is being built. Check back soon.";
        this.badge = "In Progress";
    }
    createRenderRoot() {
        return this;
    }
    render() {
        return __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$node$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["html"]`
      <div style="
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        text-align: center;
        padding: calc(var(--l) * 4) calc(var(--l) * 2);
        min-height: calc(var(--l) * 16);
        gap: calc(var(--l) * 1.5);
      ">
        <div style="
          display: flex;
          align-items: center;
          justify-content: center;
          width: calc(var(--l) * 5);
          height: calc(var(--l) * 5);
          border-radius: 50%;
          border: 1px solid var(--adi-border);
          background: var(--adi-surface);
        ">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="var(--adi-accent)"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            style="width:calc(var(--l) * 2.5);height:calc(var(--l) * 2.5);"
          >
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" />
          </svg>
        </div>

        <div style="display:flex;flex-direction:column;gap:calc(var(--l) * 0.5);max-width:calc(var(--l) * 25);">
          <h2 style="
            font-size: calc(var(--t) * 1.953);
            font-weight: 600;
            color: var(--adi-text);
            margin: 0;
            line-height: 1.2;
          ">${this.heading}</h2>

          <p style="
            font-size: calc(var(--t) * 0.875);
            color: var(--adi-text-muted);
            margin: 0;
            line-height: 1.6;
          ">${this.description}</p>
        </div>

        <div style="
          display: inline-flex;
          align-items: center;
          gap: calc(var(--l) * 0.5);
          padding: calc(var(--l) * 0.375) var(--l);
          border-radius: calc(var(--r) * 2);
          border: 1px solid var(--adi-accent);
          background: color-mix(in srgb, var(--adi-accent) 6%, transparent);
        ">
          <span style="
            width: calc(var(--l) * 0.375);
            height: calc(var(--l) * 0.375);
            border-radius: 50%;
            background: var(--adi-accent);
          "></span>
          <span style="
            font-size: calc(var(--t) * 0.75);
            font-family: monospace;
            text-transform: uppercase;
            letter-spacing: 0.1em;
            color: var(--adi-accent);
          ">${this.badge}</span>
        </div>
      </div>
    `;
    }
};
__decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["property"])({
        type: String
    })
], AdiUnderConstruction.prototype, "heading", void 0);
__decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["property"])({
        type: String
    })
], AdiUnderConstruction.prototype, "description", void 0);
__decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["property"])({
        type: String
    })
], AdiUnderConstruction.prototype, "badge", void 0);
AdiUnderConstruction = __decorate([
    (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$node$2f$development$2f$decorators$2f$custom$2d$element$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["customElement"])("adi-under-construction")
], AdiUnderConstruction);
;
 //# sourceMappingURL=under-construction.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/index.js [app-ssr] (ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$dist$2f$feedback$2f$loading$2d$skeleton$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/loading-skeleton.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$dist$2f$feedback$2f$under$2d$construction$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/under-construction.js [app-ssr] (ecmascript)"); //# sourceMappingURL=index.js.map
;
;
}),
"[project]/projects/adi-family/cli/apps/public/src/components/feedback/under-construction.tsx [app-ssr] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "UnderConstruction",
    ()=>UnderConstruction
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$apps$2f$public$2f$node_modules$2f40$lit$2f$react$2f$node$2f$development$2f$index$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/apps/public/node_modules/@lit/react/node/development/index.js [app-ssr] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$apps$2f$public$2f$node_modules$2f40$lit$2f$react$2f$node$2f$development$2f$create$2d$component$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/apps/public/node_modules/@lit/react/node/development/create-component.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$dist$2f$feedback$2f$index$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/index.js [app-ssr] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$dist$2f$feedback$2f$under$2d$construction$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/dist/feedback/under-construction.js [app-ssr] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$apps$2f$public$2f$node_modules$2f$next$2f$dist$2f$server$2f$route$2d$modules$2f$app$2d$page$2f$vendored$2f$ssr$2f$react$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/apps/public/node_modules/next/dist/server/route-modules/app-page/vendored/ssr/react.js [app-ssr] (ecmascript)");
"use client";
;
;
;
const UnderConstruction = (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$apps$2f$public$2f$node_modules$2f40$lit$2f$react$2f$node$2f$development$2f$create$2d$component$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["createComponent"])({
    tagName: "adi-under-construction",
    elementClass: __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$dist$2f$feedback$2f$under$2d$construction$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["AdiUnderConstruction"],
    react: __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$apps$2f$public$2f$node_modules$2f$next$2f$dist$2f$server$2f$route$2d$modules$2f$app$2d$page$2f$vendored$2f$ssr$2f$react$2e$js__$5b$app$2d$ssr$5d$__$28$ecmascript$29$__["default"]
});
}),
];

//# sourceMappingURL=projects_adi-family_cli_7ca9e7cf._.js.map