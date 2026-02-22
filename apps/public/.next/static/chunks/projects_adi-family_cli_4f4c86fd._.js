(globalThis.TURBOPACK || (globalThis.TURBOPACK = [])).push([typeof document === "object" ? document.currentScript : undefined,
"[project]/projects/adi-family/cli/apps/public/node_modules/@lit/react/development/create-component.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2018 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "createComponent",
    ()=>createComponent
]);
const NODE_MODE = false;
const DEV_MODE = true;
const reservedReactProperties = new Set([
    'children',
    'localName',
    'ref',
    'style',
    'className'
]);
const listenedEvents = new WeakMap();
/**
 * Adds an event listener for the specified event to the given node. In the
 * React setup, there should only ever be one event listener. Thus, for
 * efficiency only one listener is added and the handler for that listener is
 * updated to point to the given listener function.
 */ const addOrUpdateEventListener = (node, event, listener)=>{
    let events = listenedEvents.get(node);
    if (events === undefined) {
        listenedEvents.set(node, events = new Map());
    }
    let handler = events.get(event);
    if (listener !== undefined) {
        // If necessary, add listener and track handler
        if (handler === undefined) {
            events.set(event, handler = {
                handleEvent: listener
            });
            node.addEventListener(event, handler);
        // Otherwise just update the listener with new value
        } else {
            handler.handleEvent = listener;
        }
    // Remove listener if one exists and value is undefined
    } else if (handler !== undefined) {
        events.delete(event);
        node.removeEventListener(event, handler);
    }
};
/**
 * Sets properties and events on custom elements. These properties and events
 * have been pre-filtered so we know they should apply to the custom element.
 */ const setProperty = (node, name, value, old, events)=>{
    const event = events === null || events === void 0 ? void 0 : events[name];
    // Dirty check event value.
    if (event !== undefined) {
        if (value !== old) {
            addOrUpdateEventListener(node, event, value);
        }
        return;
    }
    // But don't dirty check properties; elements are assumed to do this.
    node[name] = value;
    // This block is to replicate React's behavior for attributes of native
    // elements where `undefined` or `null` values result in attributes being
    // removed.
    // https://github.com/facebook/react/blob/899cb95f52cc83ab5ca1eb1e268c909d3f0961e7/packages/react-dom-bindings/src/client/DOMPropertyOperations.js#L107-L141
    //
    // It's only needed here for native HTMLElement properties that reflect
    // attributes of the same name but don't have that behavior like "id" or
    // "draggable".
    if ((value === undefined || value === null) && name in HTMLElement.prototype) {
        node.removeAttribute(name);
    }
};
const createComponent = (param)=>{
    let { react: React, tagName, elementClass, events, displayName } = param;
    const eventProps = new Set(Object.keys(events !== null && events !== void 0 ? events : {}));
    if ("TURBOPACK compile-time truthy", 1) {
        for (const p of reservedReactProperties){
            if (p in elementClass.prototype && !(p in HTMLElement.prototype)) {
                // Note, this effectively warns only for `ref` since the other
                // reserved props are on HTMLElement.prototype. To address this
                // would require crawling down the prototype, which doesn't feel worth
                // it since implementing these properties on an element is extremely
                // rare.
                console.warn("".concat(tagName, " contains property ").concat(p, " which is a React reserved ") + "property. It will be used by React and not set on the element.");
            }
        }
    }
    const ReactComponent = React.forwardRef((props, ref)=>{
        const prevElemPropsRef = React.useRef(new Map());
        const elementRef = React.useRef(null);
        // Props to be passed to React.createElement
        const reactProps = {};
        // Props to be set on element with setProperty
        const elementProps = {};
        for (const [k, v] of Object.entries(props)){
            if (reservedReactProperties.has(k)) {
                // React does *not* handle `className` for custom elements so
                // coerce it to `class` so it's handled correctly.
                reactProps[k === 'className' ? 'class' : k] = v;
                continue;
            }
            if (eventProps.has(k) || k in elementClass.prototype) {
                elementProps[k] = v;
                continue;
            }
            reactProps[k] = v;
        }
        // useLayoutEffect produces warnings during server rendering.
        if ("TURBOPACK compile-time truthy", 1) {
            // This one has no dependency array so it'll run on every re-render.
            React.useLayoutEffect({
                "createComponent.ReactComponent.useLayoutEffect": ()=>{
                    if (elementRef.current === null) {
                        return;
                    }
                    const newElemProps = new Map();
                    for(const key in elementProps){
                        setProperty(elementRef.current, key, props[key], prevElemPropsRef.current.get(key), events);
                        prevElemPropsRef.current.delete(key);
                        newElemProps.set(key, props[key]);
                    }
                    // "Unset" any props from previous render that no longer exist.
                    // Setting to `undefined` seems like the correct thing to "unset"
                    // but currently React will set it as `null`.
                    // See https://github.com/facebook/react/issues/28203
                    for (const [key, value] of prevElemPropsRef.current){
                        setProperty(elementRef.current, key, undefined, value, events);
                    }
                    prevElemPropsRef.current = newElemProps;
                }
            }["createComponent.ReactComponent.useLayoutEffect"]);
            // Empty dependency array so this will only run once after first render.
            React.useLayoutEffect({
                "createComponent.ReactComponent.useLayoutEffect": ()=>{
                    var _elementRef_current;
                    (_elementRef_current = elementRef.current) === null || _elementRef_current === void 0 ? void 0 : _elementRef_current.removeAttribute('defer-hydration');
                }
            }["createComponent.ReactComponent.useLayoutEffect"], []);
        }
        if ("TURBOPACK compile-time falsy", 0) //TURBOPACK unreachable
        ;
        else {
            // Suppress hydration warning for server-rendered attributes.
            // This property needs to remain unminified.
            reactProps['suppressHydrationWarning'] = true;
        }
        return React.createElement(tagName, {
            ...reactProps,
            ref: React.useCallback({
                "createComponent.ReactComponent.useCallback": (node)=>{
                    elementRef.current = node;
                    if (typeof ref === 'function') {
                        ref(node);
                    } else if (ref !== null) {
                        ref.current = node;
                    }
                }
            }["createComponent.ReactComponent.useCallback"], [
                ref
            ])
        });
    });
    ReactComponent.displayName = displayName !== null && displayName !== void 0 ? displayName : elementClass.name;
    return ReactComponent;
}; //# sourceMappingURL=create-component.js.map
}),
"[project]/projects/adi-family/cli/apps/public/node_modules/@lit/react/development/index.js [app-client] (ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$apps$2f$public$2f$node_modules$2f40$lit$2f$react$2f$development$2f$create$2d$component$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/apps/public/node_modules/@lit/react/development/create-component.js [app-client] (ecmascript)"); //# sourceMappingURL=index.js.map
;
}),
"[project]/projects/adi-family/cli/apps/public/node_modules/@swc/helpers/esm/_tagged_template_literal.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "_",
    ()=>_tagged_template_literal
]);
function _tagged_template_literal(strings, raw) {
    if (!raw) raw = strings.slice(0);
    return Object.freeze(Object.defineProperties(strings, {
        raw: {
            value: Object.freeze(raw)
        }
    }));
}
;
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/css-tag.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2019 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "CSSResult",
    ()=>CSSResult,
    "adoptStyles",
    ()=>adoptStyles,
    "css",
    ()=>css,
    "getCompatibleStyle",
    ()=>getCompatibleStyle,
    "supportsAdoptingStyleSheets",
    ()=>supportsAdoptingStyleSheets,
    "unsafeCSS",
    ()=>unsafeCSS
]);
const NODE_MODE = false;
// Allows minifiers to rename references to globalThis
const global = globalThis;
const supportsAdoptingStyleSheets = global.ShadowRoot && (global.ShadyCSS === undefined || global.ShadyCSS.nativeShadow) && 'adoptedStyleSheets' in Document.prototype && 'replace' in CSSStyleSheet.prototype;
const constructionToken = Symbol();
const cssTagCache = new WeakMap();
class CSSResult {
    // This is a getter so that it's lazy. In practice, this means stylesheets
    // are not created until the first element instance is made.
    get styleSheet() {
        // If `supportsAdoptingStyleSheets` is true then we assume CSSStyleSheet is
        // constructable.
        let styleSheet = this._styleSheet;
        const strings = this._strings;
        if (supportsAdoptingStyleSheets && styleSheet === undefined) {
            const cacheable = strings !== undefined && strings.length === 1;
            if (cacheable) {
                styleSheet = cssTagCache.get(strings);
            }
            if (styleSheet === undefined) {
                (this._styleSheet = styleSheet = new CSSStyleSheet()).replaceSync(this.cssText);
                if (cacheable) {
                    cssTagCache.set(strings, styleSheet);
                }
            }
        }
        return styleSheet;
    }
    toString() {
        return this.cssText;
    }
    constructor(cssText, strings, safeToken){
        // This property needs to remain unminified.
        this['_$cssResult$'] = true;
        if (safeToken !== constructionToken) {
            throw new Error('CSSResult is not constructable. Use `unsafeCSS` or `css` instead.');
        }
        this.cssText = cssText;
        this._strings = strings;
    }
}
const textFromCSSResult = (value)=>{
    // This property needs to remain unminified.
    if (value['_$cssResult$'] === true) {
        return value.cssText;
    } else if (typeof value === 'number') {
        return value;
    } else {
        throw new Error("Value passed to 'css' function must be a 'css' function result: " + "".concat(value, ". Use 'unsafeCSS' to pass non-literal values, but take care ") + "to ensure page security.");
    }
};
const unsafeCSS = (value)=>new CSSResult(typeof value === 'string' ? value : String(value), undefined, constructionToken);
const css = function(strings) {
    for(var _len = arguments.length, values = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
        values[_key - 1] = arguments[_key];
    }
    const cssText = strings.length === 1 ? strings[0] : values.reduce((acc, v, idx)=>acc + textFromCSSResult(v) + strings[idx + 1], strings[0]);
    return new CSSResult(cssText, strings, constructionToken);
};
const adoptStyles = (renderRoot, styles)=>{
    if (supportsAdoptingStyleSheets) {
        renderRoot.adoptedStyleSheets = styles.map((s)=>s instanceof CSSStyleSheet ? s : s.styleSheet);
    } else {
        for (const s of styles){
            const style = document.createElement('style');
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            const nonce = global['litNonce'];
            if (nonce !== undefined) {
                style.setAttribute('nonce', nonce);
            }
            style.textContent = s.cssText;
            renderRoot.appendChild(style);
        }
    }
};
const cssResultFromStyleSheet = (sheet)=>{
    let cssText = '';
    for (const rule of sheet.cssRules){
        cssText += rule.cssText;
    }
    return unsafeCSS(cssText);
};
const getCompatibleStyle = supportsAdoptingStyleSheets || NODE_MODE && global.CSSStyleSheet === undefined ? (s)=>s : (s)=>s instanceof CSSStyleSheet ? cssResultFromStyleSheet(s) : s; //# sourceMappingURL=css-tag.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /**
 * Use this module if you want to create your own base class extending
 * {@link ReactiveElement}.
 * @packageDocumentation
 */ __turbopack_context__.s([
    "ReactiveElement",
    ()=>ReactiveElement,
    "defaultConverter",
    ()=>defaultConverter,
    "notEqual",
    ()=>notEqual
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/css-tag.js [app-client] (ecmascript)");
var // Ensure metadata is enabled. TypeScript does not polyfill
// Symbol.metadata, so we must ensure that it exists.
_Symbol, // Map from a class's metadata object to property options
// Note that we must use nullish-coalescing assignment so that we only use one
// map even if we load multiple version of this module.
_global, _global1;
;
;
// TODO (justinfagnani): Add `hasOwn` here when we ship ES2022
const { is, defineProperty, getOwnPropertyDescriptor, getOwnPropertyNames, getOwnPropertySymbols, getPrototypeOf } = Object;
const NODE_MODE = false;
// Lets a minifier replace globalThis references with a minified name
const global = globalThis;
if ("TURBOPACK compile-time falsy", 0) //TURBOPACK unreachable
{
    var _global2;
    var _customElements;
}
const DEV_MODE = true;
let issueWarning;
const trustedTypes = global.trustedTypes;
// Temporary workaround for https://crbug.com/993268
// Currently, any attribute starting with "on" is considered to be a
// TrustedScript source. Such boolean attributes must be set to the equivalent
// trusted emptyScript value.
const emptyStringForBooleanAttribute = trustedTypes ? trustedTypes.emptyScript : '';
const polyfillSupport = ("TURBOPACK compile-time truthy", 1) ? global.reactiveElementPolyfillSupportDevMode : "TURBOPACK unreachable";
if ("TURBOPACK compile-time truthy", 1) {
    var // Ensure warnings are issued only 1x, even if multiple versions of Lit
    // are loaded.
    _global3;
    var _litIssuedWarnings;
    (_litIssuedWarnings = (_global3 = global).litIssuedWarnings) !== null && _litIssuedWarnings !== void 0 ? _litIssuedWarnings : _global3.litIssuedWarnings = new Set();
    /**
     * Issue a warning if we haven't already, based either on `code` or `warning`.
     * Warnings are disabled automatically only by `warning`; disabling via `code`
     * can be done by users.
     */ issueWarning = (code, warning)=>{
        warning += " See https://lit.dev/msg/".concat(code, " for more information.");
        if (!global.litIssuedWarnings.has(warning) && !global.litIssuedWarnings.has(code)) {
            console.warn(warning);
            global.litIssuedWarnings.add(warning);
        }
    };
    queueMicrotask(()=>{
        var _global_ShadyDOM;
        issueWarning('dev-mode', "Lit is in dev mode. Not recommended for production!");
        // Issue polyfill support warning.
        if (((_global_ShadyDOM = global.ShadyDOM) === null || _global_ShadyDOM === void 0 ? void 0 : _global_ShadyDOM.inUse) && polyfillSupport === undefined) {
            issueWarning('polyfill-support-missing', "Shadow DOM is being polyfilled via `ShadyDOM` but " + "the `polyfill-support` module has not been loaded.");
        }
    });
}
/**
 * Useful for visualizing and logging insights into what the Lit template system is doing.
 *
 * Compiled out of prod mode builds.
 */ const debugLogEvent = ("TURBOPACK compile-time truthy", 1) ? (event)=>{
    const shouldEmit = global.emitLitDebugLogEvents;
    if (!shouldEmit) {
        return;
    }
    global.dispatchEvent(new CustomEvent('lit-debug', {
        detail: event
    }));
} : "TURBOPACK unreachable";
/*
 * When using Closure Compiler, JSCompiler_renameProperty(property, object) is
 * replaced at compile time by the munged name for object[property]. We cannot
 * alias this function, so we have to use a small shim that has the same
 * behavior when not compiling.
 */ /*@__INLINE__*/ const JSCompiler_renameProperty = (prop, _obj)=>prop;
const defaultConverter = {
    toAttribute (value, type) {
        switch(type){
            case Boolean:
                value = value ? emptyStringForBooleanAttribute : null;
                break;
            case Object:
            case Array:
                // if the value is `null` or `undefined` pass this through
                // to allow removing/no change behavior.
                value = value == null ? value : JSON.stringify(value);
                break;
        }
        return value;
    },
    fromAttribute (value, type) {
        let fromValue = value;
        switch(type){
            case Boolean:
                fromValue = value !== null;
                break;
            case Number:
                fromValue = value === null ? null : Number(value);
                break;
            case Object:
            case Array:
                // Do *not* generate exception when invalid JSON is set as elements
                // don't normally complain on being mis-configured.
                // TODO(sorvell): Do generate exception in *dev mode*.
                try {
                    // Assert to adhere to Bazel's "must type assert JSON parse" rule.
                    fromValue = JSON.parse(value);
                } catch (e) {
                    fromValue = null;
                }
                break;
        }
        return fromValue;
    }
};
const notEqual = (value, old)=>!is(value, old);
const defaultPropertyDeclaration = {
    attribute: true,
    type: String,
    converter: defaultConverter,
    reflect: false,
    useDefault: false,
    hasChanged: notEqual
};
var _metadata;
(_metadata = (_Symbol = Symbol).metadata) !== null && _metadata !== void 0 ? _metadata : _Symbol.metadata = Symbol('metadata');
var _litPropertyMetadata;
(_litPropertyMetadata = (_global = global).litPropertyMetadata) !== null && _litPropertyMetadata !== void 0 ? _litPropertyMetadata : _global.litPropertyMetadata = new WeakMap();
class ReactiveElement extends HTMLElement {
    /**
     * Adds an initializer function to the class that is called during instance
     * construction.
     *
     * This is useful for code that runs against a `ReactiveElement`
     * subclass, such as a decorator, that needs to do work for each
     * instance, such as setting up a `ReactiveController`.
     *
     * ```ts
     * const myDecorator = (target: typeof ReactiveElement, key: string) => {
     *   target.addInitializer((instance: ReactiveElement) => {
     *     // This is run during construction of the element
     *     new MyController(instance);
     *   });
     * }
     * ```
     *
     * Decorating a field will then cause each instance to run an initializer
     * that adds a controller:
     *
     * ```ts
     * class MyElement extends LitElement {
     *   @myDecorator foo;
     * }
     * ```
     *
     * Initializers are stored per-constructor. Adding an initializer to a
     * subclass does not add it to a superclass. Since initializers are run in
     * constructors, initializers will run in order of the class hierarchy,
     * starting with superclasses and progressing to the instance's class.
     *
     * @nocollapse
     */ static addInitializer(initializer) {
        this.__prepare();
        var _this__initializers;
        ((_this__initializers = this._initializers) !== null && _this__initializers !== void 0 ? _this__initializers : this._initializers = []).push(initializer);
    }
    /**
     * Returns a list of attributes corresponding to the registered properties.
     * @nocollapse
     * @category attributes
     */ static get observedAttributes() {
        // Ensure we've created all properties
        this.finalize();
        // this.__attributeToPropertyMap is only undefined after finalize() in
        // ReactiveElement itself. ReactiveElement.observedAttributes is only
        // accessed with ReactiveElement as the receiver when a subclass or mixin
        // calls super.observedAttributes
        return this.__attributeToPropertyMap && [
            ...this.__attributeToPropertyMap.keys()
        ];
    }
    /**
     * Creates a property accessor on the element prototype if one does not exist
     * and stores a {@linkcode PropertyDeclaration} for the property with the
     * given options. The property setter calls the property's `hasChanged`
     * property option or uses a strict identity check to determine whether or not
     * to request an update.
     *
     * This method may be overridden to customize properties; however,
     * when doing so, it's important to call `super.createProperty` to ensure
     * the property is setup correctly. This method calls
     * `getPropertyDescriptor` internally to get a descriptor to install.
     * To customize what properties do when they are get or set, override
     * `getPropertyDescriptor`. To customize the options for a property,
     * implement `createProperty` like this:
     *
     * ```ts
     * static createProperty(name, options) {
     *   options = Object.assign(options, {myOption: true});
     *   super.createProperty(name, options);
     * }
     * ```
     *
     * @nocollapse
     * @category properties
     */ static createProperty(name) {
        let options = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : defaultPropertyDeclaration;
        // If this is a state property, force the attribute to false.
        if (options.state) {
            options.attribute = false;
        }
        this.__prepare();
        // Whether this property is wrapping accessors.
        // Helps control the initial value change and reflection logic.
        if (this.prototype.hasOwnProperty(name)) {
            options = Object.create(options);
            options.wrapped = true;
        }
        this.elementProperties.set(name, options);
        if (!options.noAccessor) {
            const key = ("TURBOPACK compile-time truthy", 1) ? // when doing HMR.
            Symbol.for("".concat(String(name), " (@property() cache)")) : "TURBOPACK unreachable";
            const descriptor = this.getPropertyDescriptor(name, key, options);
            if (descriptor !== undefined) {
                defineProperty(this.prototype, name, descriptor);
            }
        }
    }
    /**
     * Returns a property descriptor to be defined on the given named property.
     * If no descriptor is returned, the property will not become an accessor.
     * For example,
     *
     * ```ts
     * class MyElement extends LitElement {
     *   static getPropertyDescriptor(name, key, options) {
     *     const defaultDescriptor =
     *         super.getPropertyDescriptor(name, key, options);
     *     const setter = defaultDescriptor.set;
     *     return {
     *       get: defaultDescriptor.get,
     *       set(value) {
     *         setter.call(this, value);
     *         // custom action.
     *       },
     *       configurable: true,
     *       enumerable: true
     *     }
     *   }
     * }
     * ```
     *
     * @nocollapse
     * @category properties
     */ static getPropertyDescriptor(name, key, options) {
        var _getOwnPropertyDescriptor;
        const { get, set } = (_getOwnPropertyDescriptor = getOwnPropertyDescriptor(this.prototype, name)) !== null && _getOwnPropertyDescriptor !== void 0 ? _getOwnPropertyDescriptor : {
            get () {
                return this[key];
            },
            set (v) {
                this[key] = v;
            }
        };
        if (DEV_MODE && get == null) {
            var _getOwnPropertyDescriptor1;
            if ('value' in ((_getOwnPropertyDescriptor1 = getOwnPropertyDescriptor(this.prototype, name)) !== null && _getOwnPropertyDescriptor1 !== void 0 ? _getOwnPropertyDescriptor1 : {})) {
                throw new Error("Field ".concat(JSON.stringify(String(name)), " on ") + "".concat(this.name, " was declared as a reactive property ") + "but it's actually declared as a value on the prototype. " + "Usually this is due to using @property or @state on a method.");
            }
            issueWarning('reactive-property-without-getter', "Field ".concat(JSON.stringify(String(name)), " on ") + "".concat(this.name, " was declared as a reactive property ") + "but it does not have a getter. This will be an error in a " + "future version of Lit.");
        }
        return {
            get,
            set (value) {
                const oldValue = get === null || get === void 0 ? void 0 : get.call(this);
                set === null || set === void 0 ? void 0 : set.call(this, value);
                this.requestUpdate(name, oldValue, options);
            },
            configurable: true,
            enumerable: true
        };
    }
    /**
     * Returns the property options associated with the given property.
     * These options are defined with a `PropertyDeclaration` via the `properties`
     * object or the `@property` decorator and are registered in
     * `createProperty(...)`.
     *
     * Note, this method should be considered "final" and not overridden. To
     * customize the options for a given property, override
     * {@linkcode createProperty}.
     *
     * @nocollapse
     * @final
     * @category properties
     */ static getPropertyOptions(name) {
        var _this_elementProperties_get;
        return (_this_elementProperties_get = this.elementProperties.get(name)) !== null && _this_elementProperties_get !== void 0 ? _this_elementProperties_get : defaultPropertyDeclaration;
    }
    /**
     * Initializes static own properties of the class used in bookkeeping
     * for element properties, initializers, etc.
     *
     * Can be called multiple times by code that needs to ensure these
     * properties exist before using them.
     *
     * This method ensures the superclass is finalized so that inherited
     * property metadata can be copied down.
     * @nocollapse
     */ static __prepare() {
        if (this.hasOwnProperty(JSCompiler_renameProperty('elementProperties', this))) {
            // Already prepared
            return;
        }
        // Finalize any superclasses
        const superCtor = getPrototypeOf(this);
        superCtor.finalize();
        // Create own set of initializers for this class if any exist on the
        // superclass and copy them down. Note, for a small perf boost, avoid
        // creating initializers unless needed.
        if (superCtor._initializers !== undefined) {
            this._initializers = [
                ...superCtor._initializers
            ];
        }
        // Initialize elementProperties from the superclass
        this.elementProperties = new Map(superCtor.elementProperties);
    }
    /**
     * Finishes setting up the class so that it's ready to be registered
     * as a custom element and instantiated.
     *
     * This method is called by the ReactiveElement.observedAttributes getter.
     * If you override the observedAttributes getter, you must either call
     * super.observedAttributes to trigger finalization, or call finalize()
     * yourself.
     *
     * @nocollapse
     */ static finalize() {
        if (this.hasOwnProperty(JSCompiler_renameProperty('finalized', this))) {
            return;
        }
        this.finalized = true;
        this.__prepare();
        // Create properties from the static properties block:
        if (this.hasOwnProperty(JSCompiler_renameProperty('properties', this))) {
            const props = this.properties;
            const propKeys = [
                ...getOwnPropertyNames(props),
                ...getOwnPropertySymbols(props)
            ];
            for (const p of propKeys){
                this.createProperty(p, props[p]);
            }
        }
        // Create properties from standard decorator metadata:
        const metadata = this[Symbol.metadata];
        if (metadata !== null) {
            const properties = litPropertyMetadata.get(metadata);
            if (properties !== undefined) {
                for (const [p, options] of properties){
                    this.elementProperties.set(p, options);
                }
            }
        }
        // Create the attribute-to-property map
        this.__attributeToPropertyMap = new Map();
        for (const [p, options] of this.elementProperties){
            const attr = this.__attributeNameForProperty(p, options);
            if (attr !== undefined) {
                this.__attributeToPropertyMap.set(attr, p);
            }
        }
        this.elementStyles = this.finalizeStyles(this.styles);
        if ("TURBOPACK compile-time truthy", 1) {
            if (this.hasOwnProperty('createProperty')) {
                issueWarning('no-override-create-property', 'Overriding ReactiveElement.createProperty() is deprecated. ' + 'The override will not be called with standard decorators');
            }
            if (this.hasOwnProperty('getPropertyDescriptor')) {
                issueWarning('no-override-get-property-descriptor', 'Overriding ReactiveElement.getPropertyDescriptor() is deprecated. ' + 'The override will not be called with standard decorators');
            }
        }
    }
    /**
     * Takes the styles the user supplied via the `static styles` property and
     * returns the array of styles to apply to the element.
     * Override this method to integrate into a style management system.
     *
     * Styles are deduplicated preserving the _last_ instance in the list. This
     * is a performance optimization to avoid duplicated styles that can occur
     * especially when composing via subclassing. The last item is kept to try
     * to preserve the cascade order with the assumption that it's most important
     * that last added styles override previous styles.
     *
     * @nocollapse
     * @category styles
     */ static finalizeStyles(styles) {
        const elementStyles = [];
        if (Array.isArray(styles)) {
            // Dedupe the flattened array in reverse order to preserve the last items.
            // Casting to Array<unknown> works around TS error that
            // appears to come from trying to flatten a type CSSResultArray.
            const set = new Set(styles.flat(Infinity).reverse());
            // Then preserve original order by adding the set items in reverse order.
            for (const s of set){
                elementStyles.unshift((0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["getCompatibleStyle"])(s));
            }
        } else if (styles !== undefined) {
            elementStyles.push((0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["getCompatibleStyle"])(styles));
        }
        return elementStyles;
    }
    /**
     * Returns the property name for the given attribute `name`.
     * @nocollapse
     */ static __attributeNameForProperty(name, options) {
        const attribute = options.attribute;
        return attribute === false ? undefined : typeof attribute === 'string' ? attribute : typeof name === 'string' ? name.toLowerCase() : undefined;
    }
    /**
     * Internal only override point for customizing work done when elements
     * are constructed.
     */ __initialize() {
        var _this_constructor__initializers;
        this.__updatePromise = new Promise((res)=>this.enableUpdating = res);
        this._$changedProperties = new Map();
        // This enqueues a microtask that must run before the first update, so it
        // must be called before requestUpdate()
        this.__saveInstanceProperties();
        // ensures first update will be caught by an early access of
        // `updateComplete`
        this.requestUpdate();
        (_this_constructor__initializers = this.constructor._initializers) === null || _this_constructor__initializers === void 0 ? void 0 : _this_constructor__initializers.forEach((i)=>i(this));
    }
    /**
     * Registers a `ReactiveController` to participate in the element's reactive
     * update cycle. The element automatically calls into any registered
     * controllers during its lifecycle callbacks.
     *
     * If the element is connected when `addController()` is called, the
     * controller's `hostConnected()` callback will be immediately called.
     * @category controllers
     */ addController(controller) {
        var _this___controllers;
        ((_this___controllers = this.__controllers) !== null && _this___controllers !== void 0 ? _this___controllers : this.__controllers = new Set()).add(controller);
        // If a controller is added after the element has been connected,
        // call hostConnected. Note, re-using existence of `renderRoot` here
        // (which is set in connectedCallback) to avoid the need to track a
        // first connected state.
        if (this.renderRoot !== undefined && this.isConnected) {
            var _controller_hostConnected;
            (_controller_hostConnected = controller.hostConnected) === null || _controller_hostConnected === void 0 ? void 0 : _controller_hostConnected.call(controller);
        }
    }
    /**
     * Removes a `ReactiveController` from the element.
     * @category controllers
     */ removeController(controller) {
        var _this___controllers;
        (_this___controllers = this.__controllers) === null || _this___controllers === void 0 ? void 0 : _this___controllers.delete(controller);
    }
    /**
     * Fixes any properties set on the instance before upgrade time.
     * Otherwise these would shadow the accessor and break these properties.
     * The properties are stored in a Map which is played back after the
     * constructor runs.
     */ __saveInstanceProperties() {
        const instanceProperties = new Map();
        const elementProperties = this.constructor.elementProperties;
        for (const p of elementProperties.keys()){
            if (this.hasOwnProperty(p)) {
                instanceProperties.set(p, this[p]);
                delete this[p];
            }
        }
        if (instanceProperties.size > 0) {
            this.__instanceProperties = instanceProperties;
        }
    }
    /**
     * Returns the node into which the element should render and by default
     * creates and returns an open shadowRoot. Implement to customize where the
     * element's DOM is rendered. For example, to render into the element's
     * childNodes, return `this`.
     *
     * @return Returns a node into which to render.
     * @category rendering
     */ createRenderRoot() {
        var _this_shadowRoot;
        const renderRoot = (_this_shadowRoot = this.shadowRoot) !== null && _this_shadowRoot !== void 0 ? _this_shadowRoot : this.attachShadow(this.constructor.shadowRootOptions);
        (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["adoptStyles"])(renderRoot, this.constructor.elementStyles);
        return renderRoot;
    }
    /**
     * On first connection, creates the element's renderRoot, sets up
     * element styling, and enables updating.
     * @category lifecycle
     */ connectedCallback() {
        var _this___controllers;
        var _this_renderRoot;
        // Create renderRoot before controllers `hostConnected`
        (_this_renderRoot = this.renderRoot) !== null && _this_renderRoot !== void 0 ? _this_renderRoot : this.renderRoot = this.createRenderRoot();
        this.enableUpdating(true);
        (_this___controllers = this.__controllers) === null || _this___controllers === void 0 ? void 0 : _this___controllers.forEach((c)=>{
            var _c_hostConnected;
            return (_c_hostConnected = c.hostConnected) === null || _c_hostConnected === void 0 ? void 0 : _c_hostConnected.call(c);
        });
    }
    /**
     * Note, this method should be considered final and not overridden. It is
     * overridden on the element instance with a function that triggers the first
     * update.
     * @category updates
     */ enableUpdating(_requestedUpdate) {}
    /**
     * Allows for `super.disconnectedCallback()` in extensions while
     * reserving the possibility of making non-breaking feature additions
     * when disconnecting at some point in the future.
     * @category lifecycle
     */ disconnectedCallback() {
        var _this___controllers;
        (_this___controllers = this.__controllers) === null || _this___controllers === void 0 ? void 0 : _this___controllers.forEach((c)=>{
            var _c_hostDisconnected;
            return (_c_hostDisconnected = c.hostDisconnected) === null || _c_hostDisconnected === void 0 ? void 0 : _c_hostDisconnected.call(c);
        });
    }
    /**
     * Synchronizes property values when attributes change.
     *
     * Specifically, when an attribute is set, the corresponding property is set.
     * You should rarely need to implement this callback. If this method is
     * overridden, `super.attributeChangedCallback(name, _old, value)` must be
     * called.
     *
     * See [responding to attribute changes](https://developer.mozilla.org/en-US/docs/Web/API/Web_components/Using_custom_elements#responding_to_attribute_changes)
     * on MDN for more information about the `attributeChangedCallback`.
     * @category attributes
     */ attributeChangedCallback(name, _old, value) {
        this._$attributeToProperty(name, value);
    }
    __propertyToAttribute(name, value) {
        const elemProperties = this.constructor.elementProperties;
        const options = elemProperties.get(name);
        const attr = this.constructor.__attributeNameForProperty(name, options);
        if (attr !== undefined && options.reflect === true) {
            var _options_converter;
            const converter = ((_options_converter = options.converter) === null || _options_converter === void 0 ? void 0 : _options_converter.toAttribute) !== undefined ? options.converter : defaultConverter;
            const attrValue = converter.toAttribute(value, options.type);
            if (DEV_MODE && this.constructor.enabledWarnings.includes('migration') && attrValue === undefined) {
                issueWarning('undefined-attribute-value', "The attribute value for the ".concat(name, " property is ") + "undefined on element ".concat(this.localName, ". The attribute will be ") + "removed, but in the previous version of `ReactiveElement`, " + "the attribute would not have changed.");
            }
            // Track if the property is being reflected to avoid
            // setting the property again via `attributeChangedCallback`. Note:
            // 1. this takes advantage of the fact that the callback is synchronous.
            // 2. will behave incorrectly if multiple attributes are in the reaction
            // stack at time of calling. However, since we process attributes
            // in `update` this should not be possible (or an extreme corner case
            // that we'd like to discover).
            // mark state reflecting
            this.__reflectingProperty = name;
            if (attrValue == null) {
                this.removeAttribute(attr);
            } else {
                this.setAttribute(attr, attrValue);
            }
            // mark state not reflecting
            this.__reflectingProperty = null;
        }
    }
    /** @internal */ _$attributeToProperty(name, value) {
        const ctor = this.constructor;
        // Note, hint this as an `AttributeMap` so closure clearly understands
        // the type; it has issues with tracking types through statics
        const propName = ctor.__attributeToPropertyMap.get(name);
        // Use tracking info to avoid reflecting a property value to an attribute
        // if it was just set because the attribute changed.
        if (propName !== undefined && this.__reflectingProperty !== propName) {
            var _options_converter, _this___defaultValues;
            const options = ctor.getPropertyOptions(propName);
            const converter = typeof options.converter === 'function' ? {
                fromAttribute: options.converter
            } : ((_options_converter = options.converter) === null || _options_converter === void 0 ? void 0 : _options_converter.fromAttribute) !== undefined ? options.converter : defaultConverter;
            // mark state reflecting
            this.__reflectingProperty = propName;
            const convertedValue = converter.fromAttribute(value, options.type);
            var _ref;
            this[propName] = (_ref = convertedValue !== null && convertedValue !== void 0 ? convertedValue : (_this___defaultValues = this.__defaultValues) === null || _this___defaultValues === void 0 ? void 0 : _this___defaultValues.get(propName)) !== null && _ref !== void 0 ? _ref : // eslint-disable-next-line @typescript-eslint/no-explicit-any
            convertedValue;
            // mark state not reflecting
            this.__reflectingProperty = null;
        }
    }
    /**
     * Requests an update which is processed asynchronously. This should be called
     * when an element should update based on some state not triggered by setting
     * a reactive property. In this case, pass no arguments. It should also be
     * called when manually implementing a property setter. In this case, pass the
     * property `name` and `oldValue` to ensure that any configured property
     * options are honored.
     *
     * @param name name of requesting property
     * @param oldValue old value of requesting property
     * @param options property options to use instead of the previously
     *     configured options
     * @param useNewValue if true, the newValue argument is used instead of
     *     reading the property value. This is important to use if the reactive
     *     property is a standard private accessor, as opposed to a plain
     *     property, since private members can't be dynamically read by name.
     * @param newValue the new value of the property. This is only used if
     *     `useNewValue` is true.
     * @category updates
     */ requestUpdate(name, oldValue, options) {
        let useNewValue = arguments.length > 3 && arguments[3] !== void 0 ? arguments[3] : false, newValue = arguments.length > 4 ? arguments[4] : void 0;
        // If we have a property key, perform property update steps.
        if (name !== undefined) {
            var _this___defaultValues;
            if (DEV_MODE && name instanceof Event) {
                issueWarning("", "The requestUpdate() method was called with an Event as the property name. This is probably a mistake caused by binding this.requestUpdate as an event listener. Instead bind a function that will call it with no arguments: () => this.requestUpdate()");
            }
            const ctor = this.constructor;
            if (useNewValue === false) {
                newValue = this[name];
            }
            options !== null && options !== void 0 ? options : options = ctor.getPropertyOptions(name);
            var _options_hasChanged;
            const changed = ((_options_hasChanged = options.hasChanged) !== null && _options_hasChanged !== void 0 ? _options_hasChanged : notEqual)(newValue, oldValue) || options.useDefault && options.reflect && newValue === ((_this___defaultValues = this.__defaultValues) === null || _this___defaultValues === void 0 ? void 0 : _this___defaultValues.get(name)) && !this.hasAttribute(ctor.__attributeNameForProperty(name, options));
            if (changed) {
                this._$changeProperty(name, oldValue, options);
            } else {
                // Abort the request if the property should not be considered changed.
                return;
            }
        }
        if (this.isUpdatePending === false) {
            this.__updatePromise = this.__enqueueUpdate();
        }
    }
    /**
     * @internal
     */ _$changeProperty(name, oldValue, param, initializeValue) {
        let { useDefault, reflect, wrapped } = param;
        var _this___defaultValues;
        // Record default value when useDefault is used. This allows us to
        // restore this value when the attribute is removed.
        if (useDefault && !((_this___defaultValues = this.__defaultValues) !== null && _this___defaultValues !== void 0 ? _this___defaultValues : this.__defaultValues = new Map()).has(name)) {
            var _ref;
            this.__defaultValues.set(name, (_ref = initializeValue !== null && initializeValue !== void 0 ? initializeValue : oldValue) !== null && _ref !== void 0 ? _ref : this[name]);
            // if this is not wrapping an accessor, it must be an initial setting
            // and in this case we do not want to record the change or reflect.
            if (wrapped !== true || initializeValue !== undefined) {
                return;
            }
        }
        // TODO (justinfagnani): Create a benchmark of Map.has() + Map.set(
        // vs just Map.set()
        if (!this._$changedProperties.has(name)) {
            // On the initial change, the old value should be `undefined`, except
            // with `useDefault`
            if (!this.hasUpdated && !useDefault) {
                oldValue = undefined;
            }
            this._$changedProperties.set(name, oldValue);
        }
        // Add to reflecting properties set.
        // Note, it's important that every change has a chance to add the
        // property to `__reflectingProperties`. This ensures setting
        // attribute + property reflects correctly.
        if (reflect === true && this.__reflectingProperty !== name) {
            var _this___reflectingProperties;
            ((_this___reflectingProperties = this.__reflectingProperties) !== null && _this___reflectingProperties !== void 0 ? _this___reflectingProperties : this.__reflectingProperties = new Set()).add(name);
        }
    }
    /**
     * Sets up the element to asynchronously update.
     */ async __enqueueUpdate() {
        this.isUpdatePending = true;
        try {
            // Ensure any previous update has resolved before updating.
            // This `await` also ensures that property changes are batched.
            await this.__updatePromise;
        } catch (e) {
            // Refire any previous errors async so they do not disrupt the update
            // cycle. Errors are refired so developers have a chance to observe
            // them, and this can be done by implementing
            // `window.onunhandledrejection`.
            Promise.reject(e);
        }
        const result = this.scheduleUpdate();
        // If `scheduleUpdate` returns a Promise, we await it. This is done to
        // enable coordinating updates with a scheduler. Note, the result is
        // checked to avoid delaying an additional microtask unless we need to.
        if (result != null) {
            await result;
        }
        return !this.isUpdatePending;
    }
    /**
     * Schedules an element update. You can override this method to change the
     * timing of updates by returning a Promise. The update will await the
     * returned Promise, and you should resolve the Promise to allow the update
     * to proceed. If this method is overridden, `super.scheduleUpdate()`
     * must be called.
     *
     * For instance, to schedule updates to occur just before the next frame:
     *
     * ```ts
     * override protected async scheduleUpdate(): Promise<unknown> {
     *   await new Promise((resolve) => requestAnimationFrame(() => resolve()));
     *   super.scheduleUpdate();
     * }
     * ```
     * @category updates
     */ scheduleUpdate() {
        const result = this.performUpdate();
        if (DEV_MODE && this.constructor.enabledWarnings.includes('async-perform-update') && typeof (result === null || result === void 0 ? void 0 : result.then) === 'function') {
            issueWarning('async-perform-update', "Element ".concat(this.localName, " returned a Promise from performUpdate(). ") + "This behavior is deprecated and will be removed in a future " + "version of ReactiveElement.");
        }
        return result;
    }
    /**
     * Performs an element update. Note, if an exception is thrown during the
     * update, `firstUpdated` and `updated` will not be called.
     *
     * Call `performUpdate()` to immediately process a pending update. This should
     * generally not be needed, but it can be done in rare cases when you need to
     * update synchronously.
     *
     * @category updates
     */ performUpdate() {
        // Abort any update if one is not pending when this is called.
        // This can happen if `performUpdate` is called early to "flush"
        // the update.
        if (!this.isUpdatePending) {
            return;
        }
        debugLogEvent === null || debugLogEvent === void 0 ? void 0 : debugLogEvent({
            kind: 'update'
        });
        if (!this.hasUpdated) {
            var _this_renderRoot;
            // Create renderRoot before first update. This occurs in `connectedCallback`
            // but is done here to support out of tree calls to `enableUpdating`/`performUpdate`.
            (_this_renderRoot = this.renderRoot) !== null && _this_renderRoot !== void 0 ? _this_renderRoot : this.renderRoot = this.createRenderRoot();
            if ("TURBOPACK compile-time truthy", 1) {
                // Produce warning if any reactive properties on the prototype are
                // shadowed by class fields. Instance fields set before upgrade are
                // deleted by this point, so any own property is caused by class field
                // initialization in the constructor.
                const ctor = this.constructor;
                const shadowedProperties = [
                    ...ctor.elementProperties.keys()
                ].filter((p)=>this.hasOwnProperty(p) && p in getPrototypeOf(this));
                if (shadowedProperties.length) {
                    throw new Error("The following properties on element ".concat(this.localName, " will not ") + "trigger updates as expected because they are set using class " + "fields: ".concat(shadowedProperties.join(', '), ". ") + "Native class fields and some compiled output will overwrite " + "accessors used for detecting changes. See " + "https://lit.dev/msg/class-field-shadowing " + "for more information.");
                }
            }
            // Mixin instance properties once, if they exist.
            if (this.__instanceProperties) {
                // TODO (justinfagnani): should we use the stored value? Could a new value
                // have been set since we stored the own property value?
                for (const [p, value] of this.__instanceProperties){
                    this[p] = value;
                }
                this.__instanceProperties = undefined;
            }
            // Trigger initial value reflection and populate the initial
            // `changedProperties` map, but only for the case of properties created
            // via `createProperty` on accessors, which will not have already
            // populated the `changedProperties` map since they are not set.
            // We can't know if these accessors had initializers, so we just set
            // them anyway - a difference from experimental decorators on fields and
            // standard decorators on auto-accessors.
            // For context see:
            // https://github.com/lit/lit/pull/4183#issuecomment-1711959635
            const elementProperties = this.constructor.elementProperties;
            if (elementProperties.size > 0) {
                for (const [p, options] of elementProperties){
                    const { wrapped } = options;
                    const value = this[p];
                    if (wrapped === true && !this._$changedProperties.has(p) && value !== undefined) {
                        this._$changeProperty(p, undefined, options, value);
                    }
                }
            }
        }
        let shouldUpdate = false;
        const changedProperties = this._$changedProperties;
        try {
            shouldUpdate = this.shouldUpdate(changedProperties);
            if (shouldUpdate) {
                var _this___controllers;
                this.willUpdate(changedProperties);
                (_this___controllers = this.__controllers) === null || _this___controllers === void 0 ? void 0 : _this___controllers.forEach((c)=>{
                    var _c_hostUpdate;
                    return (_c_hostUpdate = c.hostUpdate) === null || _c_hostUpdate === void 0 ? void 0 : _c_hostUpdate.call(c);
                });
                this.update(changedProperties);
            } else {
                this.__markUpdated();
            }
        } catch (e) {
            // Prevent `firstUpdated` and `updated` from running when there's an
            // update exception.
            shouldUpdate = false;
            // Ensure element can accept additional updates after an exception.
            this.__markUpdated();
            throw e;
        }
        // The update is no longer considered pending and further updates are now allowed.
        if (shouldUpdate) {
            this._$didUpdate(changedProperties);
        }
    }
    /**
     * Invoked before `update()` to compute values needed during the update.
     *
     * Implement `willUpdate` to compute property values that depend on other
     * properties and are used in the rest of the update process.
     *
     * ```ts
     * willUpdate(changedProperties) {
     *   // only need to check changed properties for an expensive computation.
     *   if (changedProperties.has('firstName') || changedProperties.has('lastName')) {
     *     this.sha = computeSHA(`${this.firstName} ${this.lastName}`);
     *   }
     * }
     *
     * render() {
     *   return html`SHA: ${this.sha}`;
     * }
     * ```
     *
     * @category updates
     */ willUpdate(_changedProperties) {}
    // Note, this is an override point for polyfill-support.
    // @internal
    _$didUpdate(changedProperties) {
        var _this___controllers;
        (_this___controllers = this.__controllers) === null || _this___controllers === void 0 ? void 0 : _this___controllers.forEach((c)=>{
            var _c_hostUpdated;
            return (_c_hostUpdated = c.hostUpdated) === null || _c_hostUpdated === void 0 ? void 0 : _c_hostUpdated.call(c);
        });
        if (!this.hasUpdated) {
            this.hasUpdated = true;
            this.firstUpdated(changedProperties);
        }
        this.updated(changedProperties);
        if (DEV_MODE && this.isUpdatePending && this.constructor.enabledWarnings.includes('change-in-update')) {
            issueWarning('change-in-update', "Element ".concat(this.localName, " scheduled an update ") + "(generally because a property was set) " + "after an update completed, causing a new update to be scheduled. " + "This is inefficient and should be avoided unless the next update " + "can only be scheduled as a side effect of the previous update.");
        }
    }
    __markUpdated() {
        this._$changedProperties = new Map();
        this.isUpdatePending = false;
    }
    /**
     * Returns a Promise that resolves when the element has completed updating.
     * The Promise value is a boolean that is `true` if the element completed the
     * update without triggering another update. The Promise result is `false` if
     * a property was set inside `updated()`. If the Promise is rejected, an
     * exception was thrown during the update.
     *
     * To await additional asynchronous work, override the `getUpdateComplete`
     * method. For example, it is sometimes useful to await a rendered element
     * before fulfilling this Promise. To do this, first await
     * `super.getUpdateComplete()`, then any subsequent state.
     *
     * @return A promise of a boolean that resolves to true if the update completed
     *     without triggering another update.
     * @category updates
     */ get updateComplete() {
        return this.getUpdateComplete();
    }
    /**
     * Override point for the `updateComplete` promise.
     *
     * It is not safe to override the `updateComplete` getter directly due to a
     * limitation in TypeScript which means it is not possible to call a
     * superclass getter (e.g. `super.updateComplete.then(...)`) when the target
     * language is ES5 (https://github.com/microsoft/TypeScript/issues/338).
     * This method should be overridden instead. For example:
     *
     * ```ts
     * class MyElement extends LitElement {
     *   override async getUpdateComplete() {
     *     const result = await super.getUpdateComplete();
     *     await this._myChild.updateComplete;
     *     return result;
     *   }
     * }
     * ```
     *
     * @return A promise of a boolean that resolves to true if the update completed
     *     without triggering another update.
     * @category updates
     */ getUpdateComplete() {
        return this.__updatePromise;
    }
    /**
     * Controls whether or not `update()` should be called when the element requests
     * an update. By default, this method always returns `true`, but this can be
     * customized to control when to update.
     *
     * @param _changedProperties Map of changed properties with old values
     * @category updates
     */ shouldUpdate(_changedProperties) {
        return true;
    }
    /**
     * Updates the element. This method reflects property values to attributes.
     * It can be overridden to render and keep updated element DOM.
     * Setting properties inside this method will *not* trigger
     * another update.
     *
     * @param _changedProperties Map of changed properties with old values
     * @category updates
     */ update(_changedProperties) {
        // The forEach() expression will only run when __reflectingProperties is
        // defined, and it returns undefined, setting __reflectingProperties to
        // undefined
        this.__reflectingProperties && (this.__reflectingProperties = this.__reflectingProperties.forEach((p)=>this.__propertyToAttribute(p, this[p])));
        this.__markUpdated();
    }
    /**
     * Invoked whenever the element is updated. Implement to perform
     * post-updating tasks via DOM APIs, for example, focusing an element.
     *
     * Setting properties inside this method will trigger the element to update
     * again after this update cycle completes.
     *
     * @param _changedProperties Map of changed properties with old values
     * @category updates
     */ updated(_changedProperties) {}
    /**
     * Invoked when the element is first updated. Implement to perform one time
     * work on the element after update.
     *
     * ```ts
     * firstUpdated() {
     *   this.renderRoot.getElementById('my-text-area').focus();
     * }
     * ```
     *
     * Setting properties inside this method will trigger the element to update
     * again after this update cycle completes.
     *
     * @param _changedProperties Map of changed properties with old values
     * @category updates
     */ firstUpdated(_changedProperties) {}
    constructor(){
        super();
        this.__instanceProperties = undefined;
        /**
         * True if there is a pending update as a result of calling `requestUpdate()`.
         * Should only be read.
         * @category updates
         */ this.isUpdatePending = false;
        /**
         * Is set to `true` after the first update. The element code cannot assume
         * that `renderRoot` exists before the element `hasUpdated`.
         * @category updates
         */ this.hasUpdated = false;
        /**
         * Name of currently reflecting property
         */ this.__reflectingProperty = null;
        this.__initialize();
    }
}
/**
 * Memoized list of all element styles.
 * Created lazily on user subclasses when finalizing the class.
 * @nocollapse
 * @category styles
 */ ReactiveElement.elementStyles = [];
/**
 * Options used when calling `attachShadow`. Set this property to customize
 * the options for the shadowRoot; for example, to create a closed
 * shadowRoot: `{mode: 'closed'}`.
 *
 * Note, these options are used in `createRenderRoot`. If this method
 * is customized, options should be respected if possible.
 * @nocollapse
 * @category rendering
 */ ReactiveElement.shadowRootOptions = {
    mode: 'open'
};
// Assigned here to work around a jscompiler bug with static fields
// when compiling to ES5.
// https://github.com/google/closure-compiler/issues/3177
ReactiveElement[JSCompiler_renameProperty('elementProperties', ReactiveElement)] = new Map();
ReactiveElement[JSCompiler_renameProperty('finalized', ReactiveElement)] = new Map();
// Apply polyfills if available
polyfillSupport === null || polyfillSupport === void 0 ? void 0 : polyfillSupport({
    ReactiveElement
});
// Dev mode warnings...
if ("TURBOPACK compile-time truthy", 1) {
    // Default warning set.
    ReactiveElement.enabledWarnings = [
        'change-in-update',
        'async-perform-update'
    ];
    const ensureOwnWarnings = function(ctor) {
        if (!ctor.hasOwnProperty(JSCompiler_renameProperty('enabledWarnings', ctor))) {
            ctor.enabledWarnings = ctor.enabledWarnings.slice();
        }
    };
    ReactiveElement.enableWarning = function(warning) {
        ensureOwnWarnings(this);
        if (!this.enabledWarnings.includes(warning)) {
            this.enabledWarnings.push(warning);
        }
    };
    ReactiveElement.disableWarning = function(warning) {
        ensureOwnWarnings(this);
        const i = this.enabledWarnings.indexOf(warning);
        if (i >= 0) {
            this.enabledWarnings.splice(i, 1);
        }
    };
}
var _reactiveElementVersions;
// IMPORTANT: do not change the property name or the assignment expression.
// This line will be used in regexes to search for ReactiveElement usage.
((_reactiveElementVersions = (_global1 = global).reactiveElementVersions) !== null && _reactiveElementVersions !== void 0 ? _reactiveElementVersions : _global1.reactiveElementVersions = []).push('2.1.2');
if (DEV_MODE && global.reactiveElementVersions.length > 1) {
    queueMicrotask(()=>{
        issueWarning('multiple-versions', "Multiple versions of Lit loaded. Loading multiple versions " + "is not recommended.");
    });
} //# sourceMappingURL=reactive-element.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/development/lit-html.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "_$LH",
    ()=>_$LH,
    "html",
    ()=>html,
    "mathml",
    ()=>mathml,
    "noChange",
    ()=>noChange,
    "nothing",
    ()=>nothing,
    "render",
    ()=>render,
    "svg",
    ()=>svg
]);
var _global_ShadyDOM, _global_ShadyDOM1;
var _global;
const DEV_MODE = true;
const ENABLE_EXTRA_SECURITY_HOOKS = true;
const ENABLE_SHADYDOM_NOPATCH = true;
const NODE_MODE = false;
// Allows minifiers to rename references to globalThis
const global = globalThis;
/**
 * Useful for visualizing and logging insights into what the Lit template system is doing.
 *
 * Compiled out of prod mode builds.
 */ const debugLogEvent = ("TURBOPACK compile-time truthy", 1) ? (event)=>{
    const shouldEmit = global.emitLitDebugLogEvents;
    if (!shouldEmit) {
        return;
    }
    global.dispatchEvent(new CustomEvent('lit-debug', {
        detail: event
    }));
} : "TURBOPACK unreachable";
// Used for connecting beginRender and endRender events when there are nested
// renders when errors are thrown preventing an endRender event from being
// called.
let debugLogRenderId = 0;
let issueWarning;
if ("TURBOPACK compile-time truthy", 1) {
    var _global1;
    var _litIssuedWarnings;
    (_litIssuedWarnings = (_global1 = global).litIssuedWarnings) !== null && _litIssuedWarnings !== void 0 ? _litIssuedWarnings : _global1.litIssuedWarnings = new Set();
    /**
     * Issue a warning if we haven't already, based either on `code` or `warning`.
     * Warnings are disabled automatically only by `warning`; disabling via `code`
     * can be done by users.
     */ issueWarning = (code, warning)=>{
        warning += code ? " See https://lit.dev/msg/".concat(code, " for more information.") : '';
        if (!global.litIssuedWarnings.has(warning) && !global.litIssuedWarnings.has(code)) {
            console.warn(warning);
            global.litIssuedWarnings.add(warning);
        }
    };
    queueMicrotask(()=>{
        issueWarning('dev-mode', "Lit is in dev mode. Not recommended for production!");
    });
}
const wrap = ENABLE_SHADYDOM_NOPATCH && ((_global_ShadyDOM = global.ShadyDOM) === null || _global_ShadyDOM === void 0 ? void 0 : _global_ShadyDOM.inUse) && ((_global_ShadyDOM1 = global.ShadyDOM) === null || _global_ShadyDOM1 === void 0 ? void 0 : _global_ShadyDOM1.noPatch) === true ? global.ShadyDOM.wrap : (node)=>node;
const trustedTypes = global.trustedTypes;
/**
 * Our TrustedTypePolicy for HTML which is declared using the html template
 * tag function.
 *
 * That HTML is a developer-authored constant, and is parsed with innerHTML
 * before any untrusted expressions have been mixed in. Therefor it is
 * considered safe by construction.
 */ const policy = trustedTypes ? trustedTypes.createPolicy('lit-html', {
    createHTML: (s)=>s
}) : undefined;
const identityFunction = (value)=>value;
const noopSanitizer = (_node, _name, _type)=>identityFunction;
/** Sets the global sanitizer factory. */ const setSanitizer = (newSanitizer)=>{
    if ("TURBOPACK compile-time falsy", 0) //TURBOPACK unreachable
    ;
    if (sanitizerFactoryInternal !== noopSanitizer) {
        throw new Error("Attempted to overwrite existing lit-html security policy." + " setSanitizeDOMValueFactory should be called at most once.");
    }
    sanitizerFactoryInternal = newSanitizer;
};
/**
 * Only used in internal tests, not a part of the public API.
 */ const _testOnlyClearSanitizerFactoryDoNotCallOrElse = ()=>{
    sanitizerFactoryInternal = noopSanitizer;
};
const createSanitizer = (node, name, type)=>{
    return sanitizerFactoryInternal(node, name, type);
};
// Added to an attribute name to mark the attribute as bound so we can find
// it easily.
const boundAttributeSuffix = '$lit$';
// This marker is used in many syntactic positions in HTML, so it must be
// a valid element name and attribute name. We don't support dynamic names (yet)
// but this at least ensures that the parse tree is closer to the template
// intention.
const marker = "lit$".concat(Math.random().toFixed(9).slice(2), "$");
// String used to tell if a comment is a marker comment
const markerMatch = '?' + marker;
// Text used to insert a comment marker node. We use processing instruction
// syntax because it's slightly smaller, but parses as a comment node.
const nodeMarker = "<".concat(markerMatch, ">");
const d = ("TURBOPACK compile-time falsy", 0) ? "TURBOPACK unreachable" : document;
// Creates a dynamic marker. We never have to search for these in the DOM.
const createMarker = ()=>d.createComment('');
const isPrimitive = (value)=>value === null || typeof value != 'object' && typeof value != 'function';
const isArray = Array.isArray;
const isIterable = (value)=>isArray(value) || // eslint-disable-next-line @typescript-eslint/no-explicit-any
    typeof (value === null || value === void 0 ? void 0 : value[Symbol.iterator]) === 'function';
const SPACE_CHAR = "[ 	\n\f\r]";
const ATTR_VALUE_CHAR = "[^ 	\n\f\r\"'`<>=]";
const NAME_CHAR = "[^\\s\"'>=/]";
// These regexes represent the five parsing states that we care about in the
// Template's HTML scanner. They match the *end* of the state they're named
// after.
// Depending on the match, we transition to a new state. If there's no match,
// we stay in the same state.
// Note that the regexes are stateful. We utilize lastIndex and sync it
// across the multiple regexes used. In addition to the five regexes below
// we also dynamically create a regex to find the matching end tags for raw
// text elements.
/**
 * End of text is: `<` followed by:
 *   (comment start) or (tag) or (dynamic tag binding)
 */ const textEndRegex = /<(?:(!--|\/[^a-zA-Z])|(\/?[a-zA-Z][^>\s]*)|(\/?$))/g;
const COMMENT_START = 1;
const TAG_NAME = 2;
const DYNAMIC_TAG_NAME = 3;
const commentEndRegex = /-->/g;
/**
 * Comments not started with <!--, like </{, can be ended by a single `>`
 */ const comment2EndRegex = />/g;
/**
 * The tagEnd regex matches the end of the "inside an opening" tag syntax
 * position. It either matches a `>`, an attribute-like sequence, or the end
 * of the string after a space (attribute-name position ending).
 *
 * See attributes in the HTML spec:
 * https://www.w3.org/TR/html5/syntax.html#elements-attributes
 *
 * " \t\n\f\r" are HTML space characters:
 * https://infra.spec.whatwg.org/#ascii-whitespace
 *
 * So an attribute is:
 *  * The name: any character except a whitespace character, ("), ('), ">",
 *    "=", or "/". Note: this is different from the HTML spec which also excludes control characters.
 *  * Followed by zero or more space characters
 *  * Followed by "="
 *  * Followed by zero or more space characters
 *  * Followed by:
 *    * Any character except space, ('), ("), "<", ">", "=", (`), or
 *    * (") then any non-("), or
 *    * (') then any non-(')
 */ const tagEndRegex = new RegExp(">|".concat(SPACE_CHAR, "(?:(").concat(NAME_CHAR, "+)(").concat(SPACE_CHAR, "*=").concat(SPACE_CHAR, "*(?:").concat(ATTR_VALUE_CHAR, "|(\"|')|))|$)"), 'g');
const ENTIRE_MATCH = 0;
const ATTRIBUTE_NAME = 1;
const SPACES_AND_EQUALS = 2;
const QUOTE_CHAR = 3;
const singleQuoteAttrEndRegex = /'/g;
const doubleQuoteAttrEndRegex = /"/g;
/**
 * Matches the raw text elements.
 *
 * Comments are not parsed within raw text elements, so we need to search their
 * text content for marker strings.
 */ const rawTextElement = /^(?:script|style|textarea|title)$/i;
/** TemplateResult types */ const HTML_RESULT = 1;
const SVG_RESULT = 2;
const MATHML_RESULT = 3;
// TemplatePart types
// IMPORTANT: these must match the values in PartType
const ATTRIBUTE_PART = 1;
const CHILD_PART = 2;
const PROPERTY_PART = 3;
const BOOLEAN_ATTRIBUTE_PART = 4;
const EVENT_PART = 5;
const ELEMENT_PART = 6;
const COMMENT_PART = 7;
/**
 * Generates a template literal tag function that returns a TemplateResult with
 * the given result type.
 */ const tag = (type)=>function(strings) {
        for(var _len = arguments.length, values = new Array(_len > 1 ? _len - 1 : 0), _key = 1; _key < _len; _key++){
            values[_key - 1] = arguments[_key];
        }
        // Warn against templates octal escape sequences
        // We do this here rather than in render so that the warning is closer to the
        // template definition.
        if (DEV_MODE && strings.some((s)=>s === undefined)) {
            console.warn('Some template strings are undefined.\n' + 'This is probably caused by illegal octal escape sequences.');
        }
        if ("TURBOPACK compile-time truthy", 1) {
            // Import static-html.js results in a circular dependency which g3 doesn't
            // handle. Instead we know that static values must have the field
            // `_$litStatic$`.
            if (values.some((val)=>val === null || val === void 0 ? void 0 : val['_$litStatic$'])) {
                issueWarning('', "Static values 'literal' or 'unsafeStatic' cannot be used as values to non-static templates.\n" + "Please use the static 'html' tag function. See https://lit.dev/docs/templates/expressions/#static-expressions");
            }
        }
        return {
            // This property needs to remain unminified.
            ['_$litType$']: type,
            strings,
            values
        };
    };
const html = tag(HTML_RESULT);
const svg = tag(SVG_RESULT);
const mathml = tag(MATHML_RESULT);
const noChange = Symbol.for('lit-noChange');
const nothing = Symbol.for('lit-nothing');
/**
 * The cache of prepared templates, keyed by the tagged TemplateStringsArray
 * and _not_ accounting for the specific template tag used. This means that
 * template tags cannot be dynamic - they must statically be one of html, svg,
 * or attr. This restriction simplifies the cache lookup, which is on the hot
 * path for rendering.
 */ const templateCache = new WeakMap();
const walker = d.createTreeWalker(d, 129 /* NodeFilter.SHOW_{ELEMENT|COMMENT} */ );
let sanitizerFactoryInternal = noopSanitizer;
function trustFromTemplateString(tsa, stringFromTSA) {
    // A security check to prevent spoofing of Lit template results.
    // In the future, we may be able to replace this with Array.isTemplateObject,
    // though we might need to make that check inside of the html and svg
    // functions, because precompiled templates don't come in as
    // TemplateStringArray objects.
    if (!isArray(tsa) || !tsa.hasOwnProperty('raw')) {
        let message = 'invalid template strings array';
        if ("TURBOPACK compile-time truthy", 1) {
            message = "\n          Internal Error: expected template strings to be an array\n          with a 'raw' field. Faking a template strings array by\n          calling html or svg like an ordinary function is effectively\n          the same as calling unsafeHtml and can lead to major security\n          issues, e.g. opening your code up to XSS attacks.\n          If you're using the html or svg tagged template functions normally\n          and still seeing this error, please file a bug at\n          https://github.com/lit/lit/issues/new?template=bug_report.md\n          and include information about your build tooling, if any.\n        ".trim().replace(/\n */g, '\n');
        }
        throw new Error(message);
    }
    return policy !== undefined ? policy.createHTML(stringFromTSA) : stringFromTSA;
}
/**
 * Returns an HTML string for the given TemplateStringsArray and result type
 * (HTML or SVG), along with the case-sensitive bound attribute names in
 * template order. The HTML contains comment markers denoting the `ChildPart`s
 * and suffixes on bound attributes denoting the `AttributeParts`.
 *
 * @param strings template strings array
 * @param type HTML or SVG
 * @return Array containing `[html, attrNames]` (array returned for terseness,
 *     to avoid object fields since this code is shared with non-minified SSR
 *     code)
 */ const getTemplateHtml = (strings, type)=>{
    // Insert makers into the template HTML to represent the position of
    // bindings. The following code scans the template strings to determine the
    // syntactic position of the bindings. They can be in text position, where
    // we insert an HTML comment, attribute value position, where we insert a
    // sentinel string and re-write the attribute name, or inside a tag where
    // we insert the sentinel string.
    const l = strings.length - 1;
    // Stores the case-sensitive bound attribute names in the order of their
    // parts. ElementParts are also reflected in this array as undefined
    // rather than a string, to disambiguate from attribute bindings.
    const attrNames = [];
    let html = type === SVG_RESULT ? '<svg>' : type === MATHML_RESULT ? '<math>' : '';
    // When we're inside a raw text tag (not it's text content), the regex
    // will still be tagRegex so we can find attributes, but will switch to
    // this regex when the tag ends.
    let rawTextEndRegex;
    // The current parsing state, represented as a reference to one of the
    // regexes
    let regex = textEndRegex;
    for(let i = 0; i < l; i++){
        const s = strings[i];
        // The index of the end of the last attribute name. When this is
        // positive at end of a string, it means we're in an attribute value
        // position and need to rewrite the attribute name.
        // We also use a special value of -2 to indicate that we encountered
        // the end of a string in attribute name position.
        let attrNameEndIndex = -1;
        let attrName;
        let lastIndex = 0;
        let match;
        // The conditions in this loop handle the current parse state, and the
        // assignments to the `regex` variable are the state transitions.
        while(lastIndex < s.length){
            // Make sure we start searching from where we previously left off
            regex.lastIndex = lastIndex;
            match = regex.exec(s);
            if (match === null) {
                break;
            }
            lastIndex = regex.lastIndex;
            if (regex === textEndRegex) {
                if (match[COMMENT_START] === '!--') {
                    regex = commentEndRegex;
                } else if (match[COMMENT_START] !== undefined) {
                    // We started a weird comment, like </{
                    regex = comment2EndRegex;
                } else if (match[TAG_NAME] !== undefined) {
                    if (rawTextElement.test(match[TAG_NAME])) {
                        // Record if we encounter a raw-text element. We'll switch to
                        // this regex at the end of the tag.
                        rawTextEndRegex = new RegExp("</".concat(match[TAG_NAME]), 'g');
                    }
                    regex = tagEndRegex;
                } else if (match[DYNAMIC_TAG_NAME] !== undefined) {
                    if ("TURBOPACK compile-time truthy", 1) {
                        throw new Error('Bindings in tag names are not supported. Please use static templates instead. ' + 'See https://lit.dev/docs/templates/expressions/#static-expressions');
                    }
                    regex = tagEndRegex;
                }
            } else if (regex === tagEndRegex) {
                if (match[ENTIRE_MATCH] === '>') {
                    // End of a tag. If we had started a raw-text element, use that
                    // regex
                    regex = rawTextEndRegex !== null && rawTextEndRegex !== void 0 ? rawTextEndRegex : textEndRegex;
                    // We may be ending an unquoted attribute value, so make sure we
                    // clear any pending attrNameEndIndex
                    attrNameEndIndex = -1;
                } else if (match[ATTRIBUTE_NAME] === undefined) {
                    // Attribute name position
                    attrNameEndIndex = -2;
                } else {
                    attrNameEndIndex = regex.lastIndex - match[SPACES_AND_EQUALS].length;
                    attrName = match[ATTRIBUTE_NAME];
                    regex = match[QUOTE_CHAR] === undefined ? tagEndRegex : match[QUOTE_CHAR] === '"' ? doubleQuoteAttrEndRegex : singleQuoteAttrEndRegex;
                }
            } else if (regex === doubleQuoteAttrEndRegex || regex === singleQuoteAttrEndRegex) {
                regex = tagEndRegex;
            } else if (regex === commentEndRegex || regex === comment2EndRegex) {
                regex = textEndRegex;
            } else {
                // Not one of the five state regexes, so it must be the dynamically
                // created raw text regex and we're at the close of that element.
                regex = tagEndRegex;
                rawTextEndRegex = undefined;
            }
        }
        if ("TURBOPACK compile-time truthy", 1) {
            // If we have a attrNameEndIndex, which indicates that we should
            // rewrite the attribute name, assert that we're in a valid attribute
            // position - either in a tag, or a quoted attribute value.
            console.assert(attrNameEndIndex === -1 || regex === tagEndRegex || regex === singleQuoteAttrEndRegex || regex === doubleQuoteAttrEndRegex, 'unexpected parse state B');
        }
        // We have four cases:
        //  1. We're in text position, and not in a raw text element
        //     (regex === textEndRegex): insert a comment marker.
        //  2. We have a non-negative attrNameEndIndex which means we need to
        //     rewrite the attribute name to add a bound attribute suffix.
        //  3. We're at the non-first binding in a multi-binding attribute, use a
        //     plain marker.
        //  4. We're somewhere else inside the tag. If we're in attribute name
        //     position (attrNameEndIndex === -2), add a sequential suffix to
        //     generate a unique attribute name.
        // Detect a binding next to self-closing tag end and insert a space to
        // separate the marker from the tag end:
        const end = regex === tagEndRegex && strings[i + 1].startsWith('/>') ? ' ' : '';
        html += regex === textEndRegex ? s + nodeMarker : attrNameEndIndex >= 0 ? (attrNames.push(attrName), s.slice(0, attrNameEndIndex) + boundAttributeSuffix + s.slice(attrNameEndIndex)) + marker + end : s + marker + (attrNameEndIndex === -2 ? i : end);
    }
    const htmlResult = html + (strings[l] || '<?>') + (type === SVG_RESULT ? '</svg>' : type === MATHML_RESULT ? '</math>' : '');
    // Returned as an array for terseness
    return [
        trustFromTemplateString(strings, htmlResult),
        attrNames
    ];
};
class Template {
    // Overridden via `litHtmlPolyfillSupport` to provide platform support.
    /** @nocollapse */ static createElement(html, _options) {
        const el = d.createElement('template');
        el.innerHTML = html;
        return el;
    }
    constructor(// This property needs to remain unminified.
    { strings, ['_$litType$']: type }, options){
        this.parts = [];
        let node;
        let nodeIndex = 0;
        let attrNameIndex = 0;
        const partCount = strings.length - 1;
        const parts = this.parts;
        // Create template element
        const [html, attrNames] = getTemplateHtml(strings, type);
        this.el = Template.createElement(html, options);
        walker.currentNode = this.el.content;
        // Re-parent SVG or MathML nodes into template root
        if (type === SVG_RESULT || type === MATHML_RESULT) {
            const wrapper = this.el.content.firstChild;
            wrapper.replaceWith(...wrapper.childNodes);
        }
        // Walk the template to find binding markers and create TemplateParts
        while((node = walker.nextNode()) !== null && parts.length < partCount){
            if (node.nodeType === 1) {
                if ("TURBOPACK compile-time truthy", 1) {
                    const tag = node.localName;
                    // Warn if `textarea` includes an expression and throw if `template`
                    // does since these are not supported. We do this by checking
                    // innerHTML for anything that looks like a marker. This catches
                    // cases like bindings in textarea there markers turn into text nodes.
                    if (/^(?:textarea|template)$/i.test(tag) && node.innerHTML.includes(marker)) {
                        const m = "Expressions are not supported inside `".concat(tag, "` ") + "elements. See https://lit.dev/msg/expression-in-".concat(tag, " for more ") + "information.";
                        if (tag === 'template') {
                            throw new Error(m);
                        } else issueWarning('', m);
                    }
                }
                // TODO (justinfagnani): for attempted dynamic tag names, we don't
                // increment the bindingIndex, and it'll be off by 1 in the element
                // and off by two after it.
                if (node.hasAttributes()) {
                    for (const name of node.getAttributeNames()){
                        if (name.endsWith(boundAttributeSuffix)) {
                            const realName = attrNames[attrNameIndex++];
                            const value = node.getAttribute(name);
                            const statics = value.split(marker);
                            const m = /([.?@])?(.*)/.exec(realName);
                            parts.push({
                                type: ATTRIBUTE_PART,
                                index: nodeIndex,
                                name: m[2],
                                strings: statics,
                                ctor: m[1] === '.' ? PropertyPart : m[1] === '?' ? BooleanAttributePart : m[1] === '@' ? EventPart : AttributePart
                            });
                            node.removeAttribute(name);
                        } else if (name.startsWith(marker)) {
                            parts.push({
                                type: ELEMENT_PART,
                                index: nodeIndex
                            });
                            node.removeAttribute(name);
                        }
                    }
                }
                // TODO (justinfagnani): benchmark the regex against testing for each
                // of the 3 raw text element names.
                if (rawTextElement.test(node.tagName)) {
                    // For raw text elements we need to split the text content on
                    // markers, create a Text node for each segment, and create
                    // a TemplatePart for each marker.
                    const strings = node.textContent.split(marker);
                    const lastIndex = strings.length - 1;
                    if (lastIndex > 0) {
                        node.textContent = trustedTypes ? trustedTypes.emptyScript : '';
                        // Generate a new text node for each literal section
                        // These nodes are also used as the markers for child parts
                        for(let i = 0; i < lastIndex; i++){
                            node.append(strings[i], createMarker());
                            // Walk past the marker node we just added
                            walker.nextNode();
                            parts.push({
                                type: CHILD_PART,
                                index: ++nodeIndex
                            });
                        }
                        // Note because this marker is added after the walker's current
                        // node, it will be walked to in the outer loop (and ignored), so
                        // we don't need to adjust nodeIndex here
                        node.append(strings[lastIndex], createMarker());
                    }
                }
            } else if (node.nodeType === 8) {
                const data = node.data;
                if (data === markerMatch) {
                    parts.push({
                        type: CHILD_PART,
                        index: nodeIndex
                    });
                } else {
                    let i = -1;
                    while((i = node.data.indexOf(marker, i + 1)) !== -1){
                        // Comment node has a binding marker inside, make an inactive part
                        // The binding won't work, but subsequent bindings will
                        parts.push({
                            type: COMMENT_PART,
                            index: nodeIndex
                        });
                        // Move to the end of the match
                        i += marker.length - 1;
                    }
                }
            }
            nodeIndex++;
        }
        if ("TURBOPACK compile-time truthy", 1) {
            // If there was a duplicate attribute on a tag, then when the tag is
            // parsed into an element the attribute gets de-duplicated. We can detect
            // this mismatch if we haven't precisely consumed every attribute name
            // when preparing the template. This works because `attrNames` is built
            // from the template string and `attrNameIndex` comes from processing the
            // resulting DOM.
            if (attrNames.length !== attrNameIndex) {
                throw new Error("Detected duplicate attribute bindings. This occurs if your template " + "has duplicate attributes on an element tag. For example " + '"<input ?disabled=${true} ?disabled=${false}>" contains a ' + 'duplicate "disabled" attribute. The error was detected in ' + "the following template: \n" + '`' + strings.join('${...}') + '`');
            }
        }
        // We could set walker.currentNode to another node here to prevent a memory
        // leak, but every time we prepare a template, we immediately render it
        // and re-use the walker in new TemplateInstance._clone().
        debugLogEvent && debugLogEvent({
            kind: 'template prep',
            template: this,
            clonableTemplate: this.el,
            parts: this.parts,
            strings
        });
    }
}
function resolveDirective(part, value) {
    let parent = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : part, attributeIndex = arguments.length > 3 ? arguments[3] : void 0;
    var _parent___directives;
    // Bail early if the value is explicitly noChange. Note, this means any
    // nested directive is still attached and is not run.
    if (value === noChange) {
        return value;
    }
    let currentDirective = attributeIndex !== undefined ? (_parent___directives = parent.__directives) === null || _parent___directives === void 0 ? void 0 : _parent___directives[attributeIndex] : parent.__directive;
    const nextDirectiveConstructor = isPrimitive(value) ? undefined : value['_$litDirective$'];
    if ((currentDirective === null || currentDirective === void 0 ? void 0 : currentDirective.constructor) !== nextDirectiveConstructor) {
        var // This property needs to remain unminified.
        _currentDirective__$notifyDirectiveConnectionChanged;
        currentDirective === null || currentDirective === void 0 ? void 0 : (_currentDirective__$notifyDirectiveConnectionChanged = currentDirective['_$notifyDirectiveConnectionChanged']) === null || _currentDirective__$notifyDirectiveConnectionChanged === void 0 ? void 0 : _currentDirective__$notifyDirectiveConnectionChanged.call(currentDirective, false);
        if (nextDirectiveConstructor === undefined) {
            currentDirective = undefined;
        } else {
            currentDirective = new nextDirectiveConstructor(part);
            currentDirective._$initialize(part, parent, attributeIndex);
        }
        if (attributeIndex !== undefined) {
            var _parent;
            var ___directives;
            ((___directives = (_parent = parent).__directives) !== null && ___directives !== void 0 ? ___directives : _parent.__directives = [])[attributeIndex] = currentDirective;
        } else {
            parent.__directive = currentDirective;
        }
    }
    if (currentDirective !== undefined) {
        value = resolveDirective(part, currentDirective._$resolve(part, value.values), currentDirective, attributeIndex);
    }
    return value;
}
/**
 * An updateable instance of a Template. Holds references to the Parts used to
 * update the template instance.
 */ class TemplateInstance {
    // Called by ChildPart parentNode getter
    get parentNode() {
        return this._$parent.parentNode;
    }
    // See comment in Disconnectable interface for why this is a getter
    get _$isConnected() {
        return this._$parent._$isConnected;
    }
    // This method is separate from the constructor because we need to return a
    // DocumentFragment and we don't want to hold onto it with an instance field.
    _clone(options) {
        const { el: { content }, parts: parts } = this._$template;
        var _options_creationScope;
        const fragment = ((_options_creationScope = options === null || options === void 0 ? void 0 : options.creationScope) !== null && _options_creationScope !== void 0 ? _options_creationScope : d).importNode(content, true);
        walker.currentNode = fragment;
        let node = walker.nextNode();
        let nodeIndex = 0;
        let partIndex = 0;
        let templatePart = parts[0];
        while(templatePart !== undefined){
            if (nodeIndex === templatePart.index) {
                let part;
                if (templatePart.type === CHILD_PART) {
                    part = new ChildPart(node, node.nextSibling, this, options);
                } else if (templatePart.type === ATTRIBUTE_PART) {
                    part = new templatePart.ctor(node, templatePart.name, templatePart.strings, this, options);
                } else if (templatePart.type === ELEMENT_PART) {
                    part = new ElementPart(node, this, options);
                }
                this._$parts.push(part);
                templatePart = parts[++partIndex];
            }
            if (nodeIndex !== (templatePart === null || templatePart === void 0 ? void 0 : templatePart.index)) {
                node = walker.nextNode();
                nodeIndex++;
            }
        }
        // We need to set the currentNode away from the cloned tree so that we
        // don't hold onto the tree even if the tree is detached and should be
        // freed.
        walker.currentNode = d;
        return fragment;
    }
    _update(values) {
        let i = 0;
        for (const part of this._$parts){
            if (part !== undefined) {
                debugLogEvent && debugLogEvent({
                    kind: 'set part',
                    part,
                    value: values[i],
                    valueIndex: i,
                    values,
                    templateInstance: this
                });
                if (part.strings !== undefined) {
                    part._$setValue(values, part, i);
                    // The number of values the part consumes is part.strings.length - 1
                    // since values are in between template spans. We increment i by 1
                    // later in the loop, so increment it by part.strings.length - 2 here
                    i += part.strings.length - 2;
                } else {
                    part._$setValue(values[i]);
                }
            }
            i++;
        }
    }
    constructor(template, parent){
        this._$parts = [];
        /** @internal */ this._$disconnectableChildren = undefined;
        this._$template = template;
        this._$parent = parent;
    }
}
class ChildPart {
    // See comment in Disconnectable interface for why this is a getter
    get _$isConnected() {
        var _this__$parent;
        var _this__$parent__$isConnected;
        // ChildParts that are not at the root should always be created with a
        // parent; only RootChildNode's won't, so they return the local isConnected
        // state
        return (_this__$parent__$isConnected = (_this__$parent = this._$parent) === null || _this__$parent === void 0 ? void 0 : _this__$parent._$isConnected) !== null && _this__$parent__$isConnected !== void 0 ? _this__$parent__$isConnected : this.__isConnected;
    }
    /**
     * The parent node into which the part renders its content.
     *
     * A ChildPart's content consists of a range of adjacent child nodes of
     * `.parentNode`, possibly bordered by 'marker nodes' (`.startNode` and
     * `.endNode`).
     *
     * - If both `.startNode` and `.endNode` are non-null, then the part's content
     * consists of all siblings between `.startNode` and `.endNode`, exclusively.
     *
     * - If `.startNode` is non-null but `.endNode` is null, then the part's
     * content consists of all siblings following `.startNode`, up to and
     * including the last child of `.parentNode`. If `.endNode` is non-null, then
     * `.startNode` will always be non-null.
     *
     * - If both `.endNode` and `.startNode` are null, then the part's content
     * consists of all child nodes of `.parentNode`.
     */ get parentNode() {
        let parentNode = wrap(this._$startNode).parentNode;
        const parent = this._$parent;
        if (parent !== undefined && (parentNode === null || parentNode === void 0 ? void 0 : parentNode.nodeType) === 11 /* Node.DOCUMENT_FRAGMENT */ ) {
            // If the parentNode is a DocumentFragment, it may be because the DOM is
            // still in the cloned fragment during initial render; if so, get the real
            // parentNode the part will be committed into by asking the parent.
            parentNode = parent.parentNode;
        }
        return parentNode;
    }
    /**
     * The part's leading marker node, if any. See `.parentNode` for more
     * information.
     */ get startNode() {
        return this._$startNode;
    }
    /**
     * The part's trailing marker node, if any. See `.parentNode` for more
     * information.
     */ get endNode() {
        return this._$endNode;
    }
    _$setValue(value) {
        let directiveParent = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : this;
        if (DEV_MODE && this.parentNode === null) {
            throw new Error("This `ChildPart` has no `parentNode` and therefore cannot accept a value. This likely means the element containing the part was manipulated in an unsupported way outside of Lit's control such that the part's marker nodes were ejected from DOM. For example, setting the element's `innerHTML` or `textContent` can do this.");
        }
        value = resolveDirective(this, value, directiveParent);
        if (isPrimitive(value)) {
            // Non-rendering child values. It's important that these do not render
            // empty text nodes to avoid issues with preventing default <slot>
            // fallback content.
            if (value === nothing || value == null || value === '') {
                if (this._$committedValue !== nothing) {
                    debugLogEvent && debugLogEvent({
                        kind: 'commit nothing to child',
                        start: this._$startNode,
                        end: this._$endNode,
                        parent: this._$parent,
                        options: this.options
                    });
                    this._$clear();
                }
                this._$committedValue = nothing;
            } else if (value !== this._$committedValue && value !== noChange) {
                this._commitText(value);
            }
        // This property needs to remain unminified.
        } else if (value['_$litType$'] !== undefined) {
            this._commitTemplateResult(value);
        } else if (value.nodeType !== undefined) {
            var _this_options;
            if (DEV_MODE && ((_this_options = this.options) === null || _this_options === void 0 ? void 0 : _this_options.host) === value) {
                this._commitText("[probable mistake: rendered a template's host in itself " + "(commonly caused by writing ${this} in a template]");
                console.warn("Attempted to render the template host", value, "inside itself. This is almost always a mistake, and in dev mode ", "we render some warning text. In production however, we'll ", "render it, which will usually result in an error, and sometimes ", "in the element disappearing from the DOM.");
                return;
            }
            this._commitNode(value);
        } else if (isIterable(value)) {
            this._commitIterable(value);
        } else {
            // Fallback, will render the string representation
            this._commitText(value);
        }
    }
    _insert(node) {
        return wrap(wrap(this._$startNode).parentNode).insertBefore(node, this._$endNode);
    }
    _commitNode(value) {
        if (this._$committedValue !== value) {
            this._$clear();
            if (ENABLE_EXTRA_SECURITY_HOOKS && sanitizerFactoryInternal !== noopSanitizer) {
                var _this__$startNode_parentNode;
                const parentNodeName = (_this__$startNode_parentNode = this._$startNode.parentNode) === null || _this__$startNode_parentNode === void 0 ? void 0 : _this__$startNode_parentNode.nodeName;
                if (parentNodeName === 'STYLE' || parentNodeName === 'SCRIPT') {
                    let message = 'Forbidden';
                    if (DEV_MODE) {
                        if (parentNodeName === 'STYLE') {
                            message = "Lit does not support binding inside style nodes. " + "This is a security risk, as style injection attacks can " + "exfiltrate data and spoof UIs. " + "Consider instead using css`...` literals " + "to compose styles, and do dynamic styling with " + "css custom properties, ::parts, <slot>s, " + "and by mutating the DOM rather than stylesheets.";
                        } else {
                            message = "Lit does not support binding inside script nodes. " + "This is a security risk, as it could allow arbitrary " + "code execution.";
                        }
                    }
                    throw new Error(message);
                }
            }
            debugLogEvent && debugLogEvent({
                kind: 'commit node',
                start: this._$startNode,
                parent: this._$parent,
                value: value,
                options: this.options
            });
            this._$committedValue = this._insert(value);
        }
    }
    _commitText(value) {
        // If the committed value is a primitive it means we called _commitText on
        // the previous render, and we know that this._$startNode.nextSibling is a
        // Text node. We can now just replace the text content (.data) of the node.
        if (this._$committedValue !== nothing && isPrimitive(this._$committedValue)) {
            const node = wrap(this._$startNode).nextSibling;
            if ("TURBOPACK compile-time truthy", 1) {
                if (this._textSanitizer === undefined) {
                    this._textSanitizer = createSanitizer(node, 'data', 'property');
                }
                value = this._textSanitizer(value);
            }
            debugLogEvent && debugLogEvent({
                kind: 'commit text',
                node,
                value,
                options: this.options
            });
            node.data = value;
        } else {
            if ("TURBOPACK compile-time truthy", 1) {
                const textNode = d.createTextNode('');
                this._commitNode(textNode);
                // When setting text content, for security purposes it matters a lot
                // what the parent is. For example, <style> and <script> need to be
                // handled with care, while <span> does not. So first we need to put a
                // text node into the document, then we can sanitize its content.
                if (this._textSanitizer === undefined) {
                    this._textSanitizer = createSanitizer(textNode, 'data', 'property');
                }
                value = this._textSanitizer(value);
                debugLogEvent && debugLogEvent({
                    kind: 'commit text',
                    node: textNode,
                    value,
                    options: this.options
                });
                textNode.data = value;
            } else //TURBOPACK unreachable
            ;
        }
        this._$committedValue = value;
    }
    _commitTemplateResult(result) {
        var _this__$committedValue;
        // This property needs to remain unminified.
        const { values, ['_$litType$']: type } = result;
        // If $litType$ is a number, result is a plain TemplateResult and we get
        // the template from the template cache. If not, result is a
        // CompiledTemplateResult and _$litType$ is a CompiledTemplate and we need
        // to create the <template> element the first time we see it.
        const template = typeof type === 'number' ? this._$getTemplate(result) : (type.el === undefined && (type.el = Template.createElement(trustFromTemplateString(type.h, type.h[0]), this.options)), type);
        if (((_this__$committedValue = this._$committedValue) === null || _this__$committedValue === void 0 ? void 0 : _this__$committedValue._$template) === template) {
            debugLogEvent && debugLogEvent({
                kind: 'template updating',
                template,
                instance: this._$committedValue,
                parts: this._$committedValue._$parts,
                options: this.options,
                values
            });
            this._$committedValue._update(values);
        } else {
            const instance = new TemplateInstance(template, this);
            const fragment = instance._clone(this.options);
            debugLogEvent && debugLogEvent({
                kind: 'template instantiated',
                template,
                instance,
                parts: instance._$parts,
                options: this.options,
                fragment,
                values
            });
            instance._update(values);
            debugLogEvent && debugLogEvent({
                kind: 'template instantiated and updated',
                template,
                instance,
                parts: instance._$parts,
                options: this.options,
                fragment,
                values
            });
            this._commitNode(fragment);
            this._$committedValue = instance;
        }
    }
    // Overridden via `litHtmlPolyfillSupport` to provide platform support.
    /** @internal */ _$getTemplate(result) {
        let template = templateCache.get(result.strings);
        if (template === undefined) {
            templateCache.set(result.strings, template = new Template(result));
        }
        return template;
    }
    _commitIterable(value) {
        // For an Iterable, we create a new InstancePart per item, then set its
        // value to the item. This is a little bit of overhead for every item in
        // an Iterable, but it lets us recurse easily and efficiently update Arrays
        // of TemplateResults that will be commonly returned from expressions like:
        // array.map((i) => html`${i}`), by reusing existing TemplateInstances.
        // If value is an array, then the previous render was of an
        // iterable and value will contain the ChildParts from the previous
        // render. If value is not an array, clear this part and make a new
        // array for ChildParts.
        if (!isArray(this._$committedValue)) {
            this._$committedValue = [];
            this._$clear();
        }
        // Lets us keep track of how many items we stamped so we can clear leftover
        // items from a previous render
        const itemParts = this._$committedValue;
        let partIndex = 0;
        let itemPart;
        for (const item of value){
            if (partIndex === itemParts.length) {
                // If no existing part, create a new one
                // TODO (justinfagnani): test perf impact of always creating two parts
                // instead of sharing parts between nodes
                // https://github.com/lit/lit/issues/1266
                itemParts.push(itemPart = new ChildPart(this._insert(createMarker()), this._insert(createMarker()), this, this.options));
            } else {
                // Reuse an existing part
                itemPart = itemParts[partIndex];
            }
            itemPart._$setValue(item);
            partIndex++;
        }
        if (partIndex < itemParts.length) {
            // itemParts always have end nodes
            this._$clear(itemPart && wrap(itemPart._$endNode).nextSibling, partIndex);
            // Truncate the parts array so _value reflects the current state
            itemParts.length = partIndex;
        }
    }
    /**
     * Removes the nodes contained within this Part from the DOM.
     *
     * @param start Start node to clear from, for clearing a subset of the part's
     *     DOM (used when truncating iterables)
     * @param from  When `start` is specified, the index within the iterable from
     *     which ChildParts are being removed, used for disconnecting directives
     *     in those Parts.
     *
     * @internal
     */ _$clear() {
        let start = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : wrap(this._$startNode).nextSibling, from = arguments.length > 1 ? arguments[1] : void 0;
        var _this__$notifyConnectionChanged, _this;
        (_this__$notifyConnectionChanged = (_this = this)._$notifyConnectionChanged) === null || _this__$notifyConnectionChanged === void 0 ? void 0 : _this__$notifyConnectionChanged.call(_this, false, true, from);
        while(start !== this._$endNode){
            // The non-null assertion is safe because if _$startNode.nextSibling is
            // null, then _$endNode is also null, and we would not have entered this
            // loop.
            const n = wrap(start).nextSibling;
            wrap(start).remove();
            start = n;
        }
    }
    /**
     * Implementation of RootPart's `isConnected`. Note that this method
     * should only be called on `RootPart`s (the `ChildPart` returned from a
     * top-level `render()` call). It has no effect on non-root ChildParts.
     * @param isConnected Whether to set
     * @internal
     */ setConnected(isConnected) {
        if (this._$parent === undefined) {
            var _this__$notifyConnectionChanged, _this;
            this.__isConnected = isConnected;
            (_this__$notifyConnectionChanged = (_this = this)._$notifyConnectionChanged) === null || _this__$notifyConnectionChanged === void 0 ? void 0 : _this__$notifyConnectionChanged.call(_this, isConnected);
        } else if ("TURBOPACK compile-time truthy", 1) {
            throw new Error('part.setConnected() may only be called on a ' + 'RootPart returned from render().');
        }
    }
    constructor(startNode, endNode, parent, options){
        this.type = CHILD_PART;
        this._$committedValue = nothing;
        // The following fields will be patched onto ChildParts when required by
        // AsyncDirective
        /** @internal */ this._$disconnectableChildren = undefined;
        this._$startNode = startNode;
        this._$endNode = endNode;
        this._$parent = parent;
        this.options = options;
        var _options_isConnected;
        // Note __isConnected is only ever accessed on RootParts (i.e. when there is
        // no _$parent); the value on a non-root-part is "don't care", but checking
        // for parent would be more code
        this.__isConnected = (_options_isConnected = options === null || options === void 0 ? void 0 : options.isConnected) !== null && _options_isConnected !== void 0 ? _options_isConnected : true;
        if ("TURBOPACK compile-time truthy", 1) {
            // Explicitly initialize for consistent class shape.
            this._textSanitizer = undefined;
        }
    }
}
class AttributePart {
    get tagName() {
        return this.element.tagName;
    }
    // See comment in Disconnectable interface for why this is a getter
    get _$isConnected() {
        return this._$parent._$isConnected;
    }
    /**
     * Sets the value of this part by resolving the value from possibly multiple
     * values and static strings and committing it to the DOM.
     * If this part is single-valued, `this._strings` will be undefined, and the
     * method will be called with a single value argument. If this part is
     * multi-value, `this._strings` will be defined, and the method is called
     * with the value array of the part's owning TemplateInstance, and an offset
     * into the value array from which the values should be read.
     * This method is overloaded this way to eliminate short-lived array slices
     * of the template instance values, and allow a fast-path for single-valued
     * parts.
     *
     * @param value The part value, or an array of values for multi-valued parts
     * @param valueIndex the index to start reading values from. `undefined` for
     *   single-valued parts
     * @param noCommit causes the part to not commit its value to the DOM. Used
     *   in hydration to prime attribute parts with their first-rendered value,
     *   but not set the attribute, and in SSR to no-op the DOM operation and
     *   capture the value for serialization.
     *
     * @internal
     */ _$setValue(value) {
        let directiveParent = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : this, valueIndex = arguments.length > 2 ? arguments[2] : void 0, noCommit = arguments.length > 3 ? arguments[3] : void 0;
        const strings = this.strings;
        // Whether any of the values has changed, for dirty-checking
        let change = false;
        if (strings === undefined) {
            // Single-value binding case
            value = resolveDirective(this, value, directiveParent, 0);
            change = !isPrimitive(value) || value !== this._$committedValue && value !== noChange;
            if (change) {
                this._$committedValue = value;
            }
        } else {
            // Interpolation case
            const values = value;
            value = strings[0];
            let i, v;
            for(i = 0; i < strings.length - 1; i++){
                v = resolveDirective(this, values[valueIndex + i], directiveParent, i);
                if (v === noChange) {
                    // If the user-provided value is `noChange`, use the previous value
                    v = this._$committedValue[i];
                }
                change || (change = !isPrimitive(v) || v !== this._$committedValue[i]);
                if (v === nothing) {
                    value = nothing;
                } else if (value !== nothing) {
                    value += (v !== null && v !== void 0 ? v : '') + strings[i + 1];
                }
                // We always record each value, even if one is `nothing`, for future
                // change detection.
                this._$committedValue[i] = v;
            }
        }
        if (change && !noCommit) {
            this._commitValue(value);
        }
    }
    /** @internal */ _commitValue(value) {
        if (value === nothing) {
            wrap(this.element).removeAttribute(this.name);
        } else {
            if ("TURBOPACK compile-time truthy", 1) {
                if (this._sanitizer === undefined) {
                    this._sanitizer = sanitizerFactoryInternal(this.element, this.name, 'attribute');
                }
                value = this._sanitizer(value !== null && value !== void 0 ? value : '');
            }
            debugLogEvent && debugLogEvent({
                kind: 'commit attribute',
                element: this.element,
                name: this.name,
                value,
                options: this.options
            });
            wrap(this.element).setAttribute(this.name, value !== null && value !== void 0 ? value : '');
        }
    }
    constructor(element, name, strings, parent, options){
        this.type = ATTRIBUTE_PART;
        /** @internal */ this._$committedValue = nothing;
        /** @internal */ this._$disconnectableChildren = undefined;
        this.element = element;
        this.name = name;
        this._$parent = parent;
        this.options = options;
        if (strings.length > 2 || strings[0] !== '' || strings[1] !== '') {
            this._$committedValue = new Array(strings.length - 1).fill(new String());
            this.strings = strings;
        } else {
            this._$committedValue = nothing;
        }
        if ("TURBOPACK compile-time truthy", 1) {
            this._sanitizer = undefined;
        }
    }
}
class PropertyPart extends AttributePart {
    /** @internal */ _commitValue(value) {
        if ("TURBOPACK compile-time truthy", 1) {
            if (this._sanitizer === undefined) {
                this._sanitizer = sanitizerFactoryInternal(this.element, this.name, 'property');
            }
            value = this._sanitizer(value);
        }
        debugLogEvent && debugLogEvent({
            kind: 'commit property',
            element: this.element,
            name: this.name,
            value,
            options: this.options
        });
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        this.element[this.name] = value === nothing ? undefined : value;
    }
    constructor(){
        super(...arguments);
        this.type = PROPERTY_PART;
    }
}
class BooleanAttributePart extends AttributePart {
    /** @internal */ _commitValue(value) {
        debugLogEvent && debugLogEvent({
            kind: 'commit boolean attribute',
            element: this.element,
            name: this.name,
            value: !!(value && value !== nothing),
            options: this.options
        });
        wrap(this.element).toggleAttribute(this.name, !!value && value !== nothing);
    }
    constructor(){
        super(...arguments);
        this.type = BOOLEAN_ATTRIBUTE_PART;
    }
}
class EventPart extends AttributePart {
    // EventPart does not use the base _$setValue/_resolveValue implementation
    // since the dirty checking is more complex
    /** @internal */ _$setValue(newListener) {
        let directiveParent = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : this;
        var _resolveDirective;
        newListener = (_resolveDirective = resolveDirective(this, newListener, directiveParent, 0)) !== null && _resolveDirective !== void 0 ? _resolveDirective : nothing;
        if (newListener === noChange) {
            return;
        }
        const oldListener = this._$committedValue;
        // If the new value is nothing or any options change we have to remove the
        // part as a listener.
        const shouldRemoveListener = newListener === nothing && oldListener !== nothing || newListener.capture !== oldListener.capture || newListener.once !== oldListener.once || newListener.passive !== oldListener.passive;
        // If the new value is not nothing and we removed the listener, we have
        // to add the part as a listener.
        const shouldAddListener = newListener !== nothing && (oldListener === nothing || shouldRemoveListener);
        debugLogEvent && debugLogEvent({
            kind: 'commit event listener',
            element: this.element,
            name: this.name,
            value: newListener,
            options: this.options,
            removeListener: shouldRemoveListener,
            addListener: shouldAddListener,
            oldListener
        });
        if (shouldRemoveListener) {
            this.element.removeEventListener(this.name, this, oldListener);
        }
        if (shouldAddListener) {
            this.element.addEventListener(this.name, this, newListener);
        }
        this._$committedValue = newListener;
    }
    handleEvent(event) {
        if (typeof this._$committedValue === 'function') {
            var _this_options;
            var _this_options_host;
            this._$committedValue.call((_this_options_host = (_this_options = this.options) === null || _this_options === void 0 ? void 0 : _this_options.host) !== null && _this_options_host !== void 0 ? _this_options_host : this.element, event);
        } else {
            this._$committedValue.handleEvent(event);
        }
    }
    constructor(element, name, strings, parent, options){
        super(element, name, strings, parent, options);
        this.type = EVENT_PART;
        if (DEV_MODE && this.strings !== undefined) {
            throw new Error("A `<".concat(element.localName, ">` has a `@").concat(name, "=...` listener with ") + 'invalid content. Event listeners in templates must have exactly ' + 'one expression and no surrounding text.');
        }
    }
}
class ElementPart {
    // See comment in Disconnectable interface for why this is a getter
    get _$isConnected() {
        return this._$parent._$isConnected;
    }
    _$setValue(value) {
        debugLogEvent && debugLogEvent({
            kind: 'commit to element binding',
            element: this.element,
            value,
            options: this.options
        });
        resolveDirective(this, value);
    }
    constructor(element, parent, options){
        this.element = element;
        this.type = ELEMENT_PART;
        /** @internal */ this._$disconnectableChildren = undefined;
        this._$parent = parent;
        this.options = options;
    }
}
const _$LH = {
    // Used in lit-ssr
    _boundAttributeSuffix: boundAttributeSuffix,
    _marker: marker,
    _markerMatch: markerMatch,
    _HTML_RESULT: HTML_RESULT,
    _getTemplateHtml: getTemplateHtml,
    // Used in tests and private-ssr-support
    _TemplateInstance: TemplateInstance,
    _isIterable: isIterable,
    _resolveDirective: resolveDirective,
    _ChildPart: ChildPart,
    _AttributePart: AttributePart,
    _BooleanAttributePart: BooleanAttributePart,
    _EventPart: EventPart,
    _PropertyPart: PropertyPart,
    _ElementPart: ElementPart
};
// Apply polyfills if available
const polyfillSupport = ("TURBOPACK compile-time truthy", 1) ? global.litHtmlPolyfillSupportDevMode : "TURBOPACK unreachable";
polyfillSupport === null || polyfillSupport === void 0 ? void 0 : polyfillSupport(Template, ChildPart);
var _litHtmlVersions;
// IMPORTANT: do not change the property name or the assignment expression.
// This line will be used in regexes to search for lit-html usage.
((_litHtmlVersions = (_global = global).litHtmlVersions) !== null && _litHtmlVersions !== void 0 ? _litHtmlVersions : _global.litHtmlVersions = []).push('3.3.2');
if (DEV_MODE && global.litHtmlVersions.length > 1) {
    queueMicrotask(()=>{
        issueWarning('multiple-versions', "Multiple versions of Lit loaded. " + "Loading multiple versions is not recommended.");
    });
}
const render = (value, container, options)=>{
    if (DEV_MODE && container == null) {
        // Give a clearer error message than
        //     Uncaught TypeError: Cannot read properties of null (reading
        //     '_$litPart$')
        // which reads like an internal Lit error.
        throw new TypeError("The container to render into may not be ".concat(container));
    }
    const renderId = ("TURBOPACK compile-time truthy", 1) ? debugLogRenderId++ : "TURBOPACK unreachable";
    var _options_renderBefore;
    const partOwnerNode = (_options_renderBefore = options === null || options === void 0 ? void 0 : options.renderBefore) !== null && _options_renderBefore !== void 0 ? _options_renderBefore : container;
    // This property needs to remain unminified.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let part = partOwnerNode['_$litPart$'];
    debugLogEvent && debugLogEvent({
        kind: 'begin render',
        id: renderId,
        value,
        container,
        options,
        part
    });
    if (part === undefined) {
        var _options_renderBefore1;
        const endNode = (_options_renderBefore1 = options === null || options === void 0 ? void 0 : options.renderBefore) !== null && _options_renderBefore1 !== void 0 ? _options_renderBefore1 : null;
        // This property needs to remain unminified.
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        partOwnerNode['_$litPart$'] = part = new ChildPart(container.insertBefore(createMarker(), endNode), endNode, undefined, options !== null && options !== void 0 ? options : {});
    }
    part._$setValue(value);
    debugLogEvent && debugLogEvent({
        kind: 'end render',
        id: renderId,
        value,
        container,
        options,
        part
    });
    return part;
};
if ("TURBOPACK compile-time truthy", 1) {
    render.setSanitizer = setSanitizer;
    render.createSanitizer = createSanitizer;
    if ("TURBOPACK compile-time truthy", 1) {
        render._testOnlyClearSanitizerFactoryDoNotCallOrElse = _testOnlyClearSanitizerFactoryDoNotCallOrElse;
    }
} //# sourceMappingURL=lit-html.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-element/development/lit-element.js [app-client] (ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /**
 * The main LitElement module, which defines the {@linkcode LitElement} base
 * class and related APIs.
 *
 * LitElement components can define a template and a set of observed
 * properties. Changing an observed property triggers a re-render of the
 * element.
 *
 * Import {@linkcode LitElement} and {@linkcode html} from this module to
 * create a component:
 *
 *  ```js
 * import {LitElement, html} from 'lit-element';
 *
 * class MyElement extends LitElement {
 *
 *   // Declare observed properties
 *   static get properties() {
 *     return {
 *       adjective: {}
 *     }
 *   }
 *
 *   constructor() {
 *     this.adjective = 'awesome';
 *   }
 *
 *   // Define the element's template
 *   render() {
 *     return html`<p>your ${adjective} template here</p>`;
 *   }
 * }
 *
 * customElements.define('my-element', MyElement);
 * ```
 *
 * `LitElement` extends {@linkcode ReactiveElement} and adds lit-html
 * templating. The `ReactiveElement` class is provided for users that want to
 * build their own custom element base classes that don't use lit-html.
 *
 * @packageDocumentation
 */ __turbopack_context__.s([
    "LitElement",
    ()=>LitElement,
    "_$LE",
    ()=>_$LE
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/development/lit-html.js [app-client] (ecmascript)");
var // Install hydration if available
_global_litElementHydrateSupport;
var _global;
;
;
;
;
/*
 * When using Closure Compiler, JSCompiler_renameProperty(property, object) is
 * replaced at compile time by the munged name for object[property]. We cannot
 * alias this function, so we have to use a small shim that has the same
 * behavior when not compiling.
 */ /*@__INLINE__*/ const JSCompiler_renameProperty = (prop, _obj)=>prop;
const DEV_MODE = true;
// Allows minifiers to rename references to globalThis
const global = globalThis;
let issueWarning;
if ("TURBOPACK compile-time truthy", 1) {
    var // Ensure warnings are issued only 1x, even if multiple versions of Lit
    // are loaded.
    _global1;
    var _litIssuedWarnings;
    (_litIssuedWarnings = (_global1 = global).litIssuedWarnings) !== null && _litIssuedWarnings !== void 0 ? _litIssuedWarnings : _global1.litIssuedWarnings = new Set();
    /**
     * Issue a warning if we haven't already, based either on `code` or `warning`.
     * Warnings are disabled automatically only by `warning`; disabling via `code`
     * can be done by users.
     */ issueWarning = (code, warning)=>{
        warning += " See https://lit.dev/msg/".concat(code, " for more information.");
        if (!global.litIssuedWarnings.has(warning) && !global.litIssuedWarnings.has(code)) {
            console.warn(warning);
            global.litIssuedWarnings.add(warning);
        }
    };
}
class LitElement extends __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["ReactiveElement"] {
    /**
     * @category rendering
     */ createRenderRoot() {
        var // When adoptedStyleSheets are shimmed, they are inserted into the
        // shadowRoot by createRenderRoot. Adjust the renderBefore node so that
        // any styles in Lit content render before adoptedStyleSheets. This is
        // important so that adoptedStyleSheets have precedence over styles in
        // the shadowRoot.
        _this_renderOptions;
        const renderRoot = super.createRenderRoot();
        var _renderBefore;
        (_renderBefore = (_this_renderOptions = this.renderOptions).renderBefore) !== null && _renderBefore !== void 0 ? _renderBefore : _this_renderOptions.renderBefore = renderRoot.firstChild;
        return renderRoot;
    }
    /**
     * Updates the element. This method reflects property values to attributes
     * and calls `render` to render DOM via lit-html. Setting properties inside
     * this method will *not* trigger another update.
     * @param changedProperties Map of changed properties with old values
     * @category updates
     */ update(changedProperties) {
        // Setting properties in `render` should not trigger an update. Since
        // updates are allowed after super.update, it's important to call `render`
        // before that.
        const value = this.render();
        if (!this.hasUpdated) {
            this.renderOptions.isConnected = this.isConnected;
        }
        super.update(changedProperties);
        this.__childPart = (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["render"])(value, this.renderRoot, this.renderOptions);
    }
    /**
     * Invoked when the component is added to the document's DOM.
     *
     * In `connectedCallback()` you should setup tasks that should only occur when
     * the element is connected to the document. The most common of these is
     * adding event listeners to nodes external to the element, like a keydown
     * event handler added to the window.
     *
     * ```ts
     * connectedCallback() {
     *   super.connectedCallback();
     *   addEventListener('keydown', this._handleKeydown);
     * }
     * ```
     *
     * Typically, anything done in `connectedCallback()` should be undone when the
     * element is disconnected, in `disconnectedCallback()`.
     *
     * @category lifecycle
     */ connectedCallback() {
        var _this___childPart;
        super.connectedCallback();
        (_this___childPart = this.__childPart) === null || _this___childPart === void 0 ? void 0 : _this___childPart.setConnected(true);
    }
    /**
     * Invoked when the component is removed from the document's DOM.
     *
     * This callback is the main signal to the element that it may no longer be
     * used. `disconnectedCallback()` should ensure that nothing is holding a
     * reference to the element (such as event listeners added to nodes external
     * to the element), so that it is free to be garbage collected.
     *
     * ```ts
     * disconnectedCallback() {
     *   super.disconnectedCallback();
     *   window.removeEventListener('keydown', this._handleKeydown);
     * }
     * ```
     *
     * An element may be re-connected after being disconnected.
     *
     * @category lifecycle
     */ disconnectedCallback() {
        var _this___childPart;
        super.disconnectedCallback();
        (_this___childPart = this.__childPart) === null || _this___childPart === void 0 ? void 0 : _this___childPart.setConnected(false);
    }
    /**
     * Invoked on each update to perform rendering tasks. This method may return
     * any value renderable by lit-html's `ChildPart` - typically a
     * `TemplateResult`. Setting properties inside this method will *not* trigger
     * the element to update.
     * @category rendering
     */ render() {
        return __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["noChange"];
    }
    constructor(){
        super(...arguments);
        /**
         * @category rendering
         */ this.renderOptions = {
            host: this
        };
        this.__childPart = undefined;
    }
}
// This property needs to remain unminified.
LitElement['_$litElement$'] = true;
/**
 * Ensure this class is marked as `finalized` as an optimization ensuring
 * it will not needlessly try to `finalize`.
 *
 * Note this property name is a string to prevent breaking Closure JS Compiler
 * optimizations. See @lit/reactive-element for more information.
 */ LitElement[JSCompiler_renameProperty('finalized', LitElement)] = true;
(_global_litElementHydrateSupport = global.litElementHydrateSupport) === null || _global_litElementHydrateSupport === void 0 ? void 0 : _global_litElementHydrateSupport.call(global, {
    LitElement
});
// Apply polyfills if available
const polyfillSupport = ("TURBOPACK compile-time truthy", 1) ? global.litElementPolyfillSupportDevMode : "TURBOPACK unreachable";
polyfillSupport === null || polyfillSupport === void 0 ? void 0 : polyfillSupport({
    LitElement
});
const _$LE = {
    _$attributeToProperty: (el, name, value)=>{
        // eslint-disable-next-line
        el._$attributeToProperty(name, value);
    },
    // eslint-disable-next-line
    _$changedProperties: (el)=>el._$changedProperties
};
var _litElementVersions;
// IMPORTANT: do not change the property name or the assignment expression.
// This line will be used in regexes to search for LitElement usage.
((_litElementVersions = (_global = global).litElementVersions) !== null && _litElementVersions !== void 0 ? _litElementVersions : _global.litElementVersions = []).push('4.2.2');
if (DEV_MODE && global.litElementVersions.length > 1) {
    queueMicrotask(()=>{
        issueWarning('multiple-versions', "Multiple versions of Lit loaded. Loading multiple versions " + "is not recommended.");
    });
} //# sourceMappingURL=lit-element.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/development/is-server.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2022 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /**
 * @fileoverview
 *
 * This file exports a boolean const whose value will depend on what environment
 * the module is being imported from.
 */ __turbopack_context__.s([
    "isServer",
    ()=>isServer
]);
const NODE_MODE = false;
const isServer = NODE_MODE; //# sourceMappingURL=is-server.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit/index.js [app-client] (ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/development/lit-html.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-element/development/lit-element.js [app-client] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$is$2d$server$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/development/is-server.js [app-client] (ecmascript)"); //# sourceMappingURL=index.js.map
;
;
;
;
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "CSSResult",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["CSSResult"],
    "ReactiveElement",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["ReactiveElement"],
    "adoptStyles",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["adoptStyles"],
    "css",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["css"],
    "defaultConverter",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["defaultConverter"],
    "getCompatibleStyle",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["getCompatibleStyle"],
    "notEqual",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["notEqual"],
    "supportsAdoptingStyleSheets",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["supportsAdoptingStyleSheets"],
    "unsafeCSS",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["unsafeCSS"]
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$css$2d$tag$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/css-tag.js [app-client] (ecmascript)");
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-element/development/lit-element.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([
    "CSSResult",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["CSSResult"],
    "LitElement",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["LitElement"],
    "ReactiveElement",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["ReactiveElement"],
    "_$LE",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["_$LE"],
    "_$LH",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["_$LH"],
    "adoptStyles",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["adoptStyles"],
    "css",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["css"],
    "defaultConverter",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["defaultConverter"],
    "getCompatibleStyle",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["getCompatibleStyle"],
    "html",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["html"],
    "mathml",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["mathml"],
    "noChange",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["noChange"],
    "notEqual",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["notEqual"],
    "nothing",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["nothing"],
    "render",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["render"],
    "supportsAdoptingStyleSheets",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["supportsAdoptingStyleSheets"],
    "svg",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["svg"],
    "unsafeCSS",
    ()=>__TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["unsafeCSS"]
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$element$2f$development$2f$lit$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-element/development/lit-element.js [app-client] (ecmascript) <locals>");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f$lit$2d$html$2f$development$2f$lit$2d$html$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit-html/development/lit-html.js [app-client] (ecmascript)");
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/custom-element.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /**
 * Class decorator factory that defines the decorated class as a custom element.
 *
 * ```js
 * @customElement('my-element')
 * class MyElement extends LitElement {
 *   render() {
 *     return html``;
 *   }
 * }
 * ```
 * @category Decorator
 * @param tagName The tag name of the custom element to define.
 */ __turbopack_context__.s([
    "customElement",
    ()=>customElement
]);
const customElement = (tagName)=>(classOrTarget, context)=>{
        if (context !== undefined) {
            context.addInitializer(()=>{
                customElements.define(tagName, classOrTarget);
            });
        } else {
            customElements.define(tagName, classOrTarget);
        }
    }; //# sourceMappingURL=custom-element.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/property.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /*
 * IMPORTANT: For compatibility with tsickle and the Closure JS compiler, all
 * property decorators (but not class decorators) in this file that have
 * an @ExportDecoratedItems annotation must be defined as a regular function,
 * not an arrow function.
 */ __turbopack_context__.s([
    "property",
    ()=>property,
    "standardProperty",
    ()=>standardProperty
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/reactive-element.js [app-client] (ecmascript) <locals>");
;
const DEV_MODE = true;
let issueWarning;
if ("TURBOPACK compile-time truthy", 1) {
    var // Ensure warnings are issued only 1x, even if multiple versions of Lit
    // are loaded.
    _globalThis;
    var _litIssuedWarnings;
    (_litIssuedWarnings = (_globalThis = globalThis).litIssuedWarnings) !== null && _litIssuedWarnings !== void 0 ? _litIssuedWarnings : _globalThis.litIssuedWarnings = new Set();
    /**
     * Issue a warning if we haven't already, based either on `code` or `warning`.
     * Warnings are disabled automatically only by `warning`; disabling via `code`
     * can be done by users.
     */ issueWarning = (code, warning)=>{
        warning += " See https://lit.dev/msg/".concat(code, " for more information.");
        if (!globalThis.litIssuedWarnings.has(warning) && !globalThis.litIssuedWarnings.has(code)) {
            console.warn(warning);
            globalThis.litIssuedWarnings.add(warning);
        }
    };
}
const legacyProperty = (options, proto, name)=>{
    const hasOwnProperty = proto.hasOwnProperty(name);
    proto.constructor.createProperty(name, options);
    // For accessors (which have a descriptor on the prototype) we need to
    // return a descriptor, otherwise TypeScript overwrites the descriptor we
    // define in createProperty() with the original descriptor. We don't do this
    // for fields, which don't have a descriptor, because this could overwrite
    // descriptor defined by other decorators.
    return hasOwnProperty ? Object.getOwnPropertyDescriptor(proto, name) : undefined;
};
// This is duplicated from a similar variable in reactive-element.ts, but
// actually makes sense to have this default defined with the decorator, so
// that different decorators could have different defaults.
const defaultPropertyDeclaration = {
    attribute: true,
    type: String,
    converter: __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["defaultConverter"],
    reflect: false,
    hasChanged: __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$reactive$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__$3c$locals$3e$__["notEqual"]
};
const standardProperty = function() {
    let options = arguments.length > 0 && arguments[0] !== void 0 ? arguments[0] : defaultPropertyDeclaration, target = arguments.length > 1 ? arguments[1] : void 0, context = arguments.length > 2 ? arguments[2] : void 0;
    const { kind, metadata } = context;
    if (DEV_MODE && metadata == null) {
        issueWarning('missing-class-metadata', "The class ".concat(target, " is missing decorator metadata. This ") + "could mean that you're using a compiler that supports decorators " + "but doesn't support decorator metadata, such as TypeScript 5.1. " + "Please update your compiler.");
    }
    // Store the property options
    let properties = globalThis.litPropertyMetadata.get(metadata);
    if (properties === undefined) {
        globalThis.litPropertyMetadata.set(metadata, properties = new Map());
    }
    if (kind === 'setter') {
        options = Object.create(options);
        options.wrapped = true;
    }
    properties.set(context.name, options);
    if (kind === 'accessor') {
        // Standard decorators cannot dynamically modify the class, so we can't
        // replace a field with accessors. The user must use the new `accessor`
        // keyword instead.
        const { name } = context;
        return {
            set (v) {
                const oldValue = target.get.call(this);
                target.set.call(this, v);
                this.requestUpdate(name, oldValue, options, true, v);
            },
            init (v) {
                if (v !== undefined) {
                    this._$changeProperty(name, undefined, options, v);
                }
                return v;
            }
        };
    } else if (kind === 'setter') {
        const { name } = context;
        return function(value) {
            const oldValue = this[name];
            target.call(this, value);
            this.requestUpdate(name, oldValue, options, true, value);
        };
    }
    throw new Error("Unsupported decorator location: ".concat(kind));
};
function property(options) {
    return (protoOrTarget, nameOrContext)=>{
        return typeof nameOrContext === 'object' ? standardProperty(options, protoOrTarget, nameOrContext) : legacyProperty(options, protoOrTarget, nameOrContext);
    };
} //# sourceMappingURL=property.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/state.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /*
 * IMPORTANT: For compatibility with tsickle and the Closure JS compiler, all
 * property decorators (but not class decorators) in this file that have
 * an @ExportDecoratedItems annotation must be defined as a regular function,
 * not an arrow function.
 */ __turbopack_context__.s([
    "state",
    ()=>state
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/property.js [app-client] (ecmascript)");
;
function state(options) {
    return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["property"])({
        ...options,
        // Add both `state` and `attribute` because we found a third party
        // controller that is keying off of PropertyOptions.state to determine
        // whether a field is a private internal property or not.
        state: true,
        attribute: false
    });
} //# sourceMappingURL=state.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/event-options.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /**
 * Adds event listener options to a method used as an event listener in a
 * lit-html template.
 *
 * @param options An object that specifies event listener options as accepted by
 * `EventTarget#addEventListener` and `EventTarget#removeEventListener`.
 *
 * Current browsers support the `capture`, `passive`, and `once` options. See:
 * https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener#Parameters
 *
 * ```ts
 * class MyElement {
 *   clicked = false;
 *
 *   render() {
 *     return html`
 *       <div @click=${this._onClick}>
 *         <button></button>
 *       </div>
 *     `;
 *   }
 *
 *   @eventOptions({capture: true})
 *   _onClick(e) {
 *     this.clicked = true;
 *   }
 * }
 * ```
 * @category Decorator
 */ __turbopack_context__.s([
    "eventOptions",
    ()=>eventOptions
]);
function eventOptions(options) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return (protoOrValue, nameOrContext)=>{
        const method = typeof protoOrValue === 'function' ? protoOrValue : protoOrValue[nameOrContext];
        Object.assign(method, options);
    };
} //# sourceMappingURL=event-options.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/base.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ /**
 * Wraps up a few best practices when returning a property descriptor from a
 * decorator.
 *
 * Marks the defined property as configurable, and enumerable, and handles
 * the case where we have a busted Reflect.decorate zombiefill (e.g. in Angular
 * apps).
 *
 * @internal
 */ __turbopack_context__.s([
    "desc",
    ()=>desc
]);
const desc = (obj, name, descriptor)=>{
    // For backwards compatibility, we keep them configurable and enumerable.
    descriptor.configurable = true;
    descriptor.enumerable = true;
    if (// We check for Reflect.decorate each time, in case the zombiefill
    // is applied via lazy loading some Angular code.
    Reflect.decorate && typeof name !== 'object') {
        // If we're called as a legacy decorator, and Reflect.decorate is present
        // then we have no guarantees that the returned descriptor will be
        // defined on the class, so we must apply it directly ourselves.
        Object.defineProperty(obj, name, descriptor);
    }
    return descriptor;
}; //# sourceMappingURL=base.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "query",
    ()=>query
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/base.js [app-client] (ecmascript)");
;
const DEV_MODE = true;
let issueWarning;
if ("TURBOPACK compile-time truthy", 1) {
    var // Ensure warnings are issued only 1x, even if multiple versions of Lit
    // are loaded.
    _globalThis;
    var _litIssuedWarnings;
    (_litIssuedWarnings = (_globalThis = globalThis).litIssuedWarnings) !== null && _litIssuedWarnings !== void 0 ? _litIssuedWarnings : _globalThis.litIssuedWarnings = new Set();
    /**
     * Issue a warning if we haven't already, based either on `code` or `warning`.
     * Warnings are disabled automatically only by `warning`; disabling via `code`
     * can be done by users.
     */ issueWarning = (code, warning)=>{
        warning += code ? " See https://lit.dev/msg/".concat(code, " for more information.") : '';
        if (!globalThis.litIssuedWarnings.has(warning) && !globalThis.litIssuedWarnings.has(code)) {
            console.warn(warning);
            globalThis.litIssuedWarnings.add(warning);
        }
    };
}
function query(selector, cache) {
    return (protoOrTarget, nameOrContext, descriptor)=>{
        const doQuery = (el)=>{
            var _el_renderRoot;
            var _el_renderRoot_querySelector;
            const result = (_el_renderRoot_querySelector = (_el_renderRoot = el.renderRoot) === null || _el_renderRoot === void 0 ? void 0 : _el_renderRoot.querySelector(selector)) !== null && _el_renderRoot_querySelector !== void 0 ? _el_renderRoot_querySelector : null;
            if (DEV_MODE && result === null && cache && !el.hasUpdated) {
                const name = typeof nameOrContext === 'object' ? nameOrContext.name : nameOrContext;
                issueWarning('', "@query'd field ".concat(JSON.stringify(String(name)), " with the 'cache' ") + "flag set for selector '".concat(selector, "' has been accessed before ") + "the first update and returned null. This is expected if the " + "renderRoot tree has not been provided beforehand (e.g. via " + "Declarative Shadow DOM). Therefore the value hasn't been cached.");
            }
            // TODO: if we want to allow users to assert that the query will never
            // return null, we need a new option and to throw here if the result
            // is null.
            return result;
        };
        if (cache) {
            // Accessors to wrap from either:
            //   1. The decorator target, in the case of standard decorators
            //   2. The property descriptor, in the case of experimental decorators
            //      on auto-accessors.
            //   3. Functions that access our own cache-key property on the instance,
            //      in the case of experimental decorators on fields.
            const { get, set } = typeof nameOrContext === 'object' ? protoOrTarget : descriptor !== null && descriptor !== void 0 ? descriptor : (()=>{
                const key = ("TURBOPACK compile-time truthy", 1) ? Symbol("".concat(String(nameOrContext), " (@query() cache)")) : "TURBOPACK unreachable";
                return {
                    get () {
                        return this[key];
                    },
                    set (v) {
                        this[key] = v;
                    }
                };
            })();
            return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["desc"])(protoOrTarget, nameOrContext, {
                get () {
                    let result = get.call(this);
                    if (result === undefined) {
                        result = doQuery(this);
                        if (result !== null || this.hasUpdated) {
                            set.call(this, result);
                        }
                    }
                    return result;
                }
            });
        } else {
            // This object works as the return type for both standard and
            // experimental decorators.
            return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["desc"])(protoOrTarget, nameOrContext, {
                get () {
                    return doQuery(this);
                }
            });
        }
    };
} //# sourceMappingURL=query.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-all.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "queryAll",
    ()=>queryAll
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/base.js [app-client] (ecmascript)");
;
// Shared fragment used to generate empty NodeLists when a render root is
// undefined
let fragment;
function queryAll(selector) {
    return (obj, name)=>{
        return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["desc"])(obj, name, {
            get () {
                var _this_renderRoot;
                const container = (_this_renderRoot = this.renderRoot) !== null && _this_renderRoot !== void 0 ? _this_renderRoot : fragment !== null && fragment !== void 0 ? fragment : fragment = document.createDocumentFragment();
                return container.querySelectorAll(selector);
            }
        });
    };
} //# sourceMappingURL=query-all.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-async.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "queryAsync",
    ()=>queryAsync
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/base.js [app-client] (ecmascript)");
;
function queryAsync(selector) {
    return (obj, name)=>{
        return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["desc"])(obj, name, {
            async get () {
                var _this_renderRoot;
                await this.updateComplete;
                var _this_renderRoot_querySelector;
                return (_this_renderRoot_querySelector = (_this_renderRoot = this.renderRoot) === null || _this_renderRoot === void 0 ? void 0 : _this_renderRoot.querySelector(selector)) !== null && _this_renderRoot_querySelector !== void 0 ? _this_renderRoot_querySelector : null;
            }
        });
    };
} //# sourceMappingURL=query-async.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-assigned-elements.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2021 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "queryAssignedElements",
    ()=>queryAssignedElements
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/base.js [app-client] (ecmascript)");
;
function queryAssignedElements(options) {
    return (obj, name)=>{
        const { slot, selector } = options !== null && options !== void 0 ? options : {};
        const slotSelector = "slot".concat(slot ? "[name=".concat(slot, "]") : ':not([name])');
        return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["desc"])(obj, name, {
            get () {
                var _this_renderRoot;
                const slotEl = (_this_renderRoot = this.renderRoot) === null || _this_renderRoot === void 0 ? void 0 : _this_renderRoot.querySelector(slotSelector);
                var _slotEl_assignedElements;
                const elements = (_slotEl_assignedElements = slotEl === null || slotEl === void 0 ? void 0 : slotEl.assignedElements(options)) !== null && _slotEl_assignedElements !== void 0 ? _slotEl_assignedElements : [];
                return selector === undefined ? elements : elements.filter((node)=>node.matches(selector));
            }
        });
    };
} //# sourceMappingURL=query-assigned-elements.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-assigned-nodes.js [app-client] (ecmascript)", ((__turbopack_context__) => {
"use strict";

/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */ __turbopack_context__.s([
    "queryAssignedNodes",
    ()=>queryAssignedNodes
]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/base.js [app-client] (ecmascript)");
;
function queryAssignedNodes(options) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return (obj, name)=>{
        const { slot } = options !== null && options !== void 0 ? options : {};
        const slotSelector = "slot".concat(slot ? "[name=".concat(slot, "]") : ':not([name])');
        return (0, __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$base$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__["desc"])(obj, name, {
            get () {
                var _this_renderRoot;
                const slotEl = (_this_renderRoot = this.renderRoot) === null || _this_renderRoot === void 0 ? void 0 : _this_renderRoot.querySelector(slotSelector);
                var _slotEl_assignedNodes;
                return (_slotEl_assignedNodes = slotEl === null || slotEl === void 0 ? void 0 : slotEl.assignedNodes(options)) !== null && _slotEl_assignedNodes !== void 0 ? _slotEl_assignedNodes : [];
            }
        });
    };
} //# sourceMappingURL=query-assigned-nodes.js.map
}),
"[project]/projects/adi-family/cli/packages/ui-components/node_modules/lit/decorators.js [app-client] (ecmascript) <locals>", ((__turbopack_context__) => {
"use strict";

__turbopack_context__.s([]);
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$custom$2d$element$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/custom-element.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$property$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/property.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$state$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/state.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$event$2d$options$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/event-options.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$query$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$query$2d$all$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-all.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$query$2d$async$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-async.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$query$2d$assigned$2d$elements$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-assigned-elements.js [app-client] (ecmascript)");
var __TURBOPACK__imported__module__$5b$project$5d2f$projects$2f$adi$2d$family$2f$cli$2f$packages$2f$ui$2d$components$2f$node_modules$2f40$lit$2f$reactive$2d$element$2f$development$2f$decorators$2f$query$2d$assigned$2d$nodes$2e$js__$5b$app$2d$client$5d$__$28$ecmascript$29$__ = __turbopack_context__.i("[project]/projects/adi-family/cli/packages/ui-components/node_modules/@lit/reactive-element/development/decorators/query-assigned-nodes.js [app-client] (ecmascript)"); //# sourceMappingURL=decorators.js.map
;
;
;
;
;
;
;
;
;
}),
]);

//# sourceMappingURL=projects_adi-family_cli_4f4c86fd._.js.map