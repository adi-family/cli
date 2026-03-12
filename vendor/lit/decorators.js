/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
const customElement = (g) => (R, z) => {
	z === void 0 ? customElements.define(g, R) : z.addInitializer(() => {
		customElements.define(g, R);
	});
};
/**
* @license
* Copyright 2019 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var NODE_MODE = !1, global$1 = globalThis;
const supportsAdoptingStyleSheets = global$1.ShadowRoot && (global$1.ShadyCSS === void 0 || global$1.ShadyCSS.nativeShadow) && "adoptedStyleSheets" in Document.prototype && "replace" in CSSStyleSheet.prototype;
var constructionToken = Symbol(), cssTagCache = /* @__PURE__ */ new WeakMap(), CSSResult = class {
	constructor(g, R, z) {
		if (this._$cssResult$ = !0, z !== constructionToken) throw Error("CSSResult is not constructable. Use `unsafeCSS` or `css` instead.");
		this.cssText = g, this._strings = R;
	}
	get styleSheet() {
		let g = this._styleSheet, R = this._strings;
		if (supportsAdoptingStyleSheets && g === void 0) {
			let z = R !== void 0 && R.length === 1;
			z && (g = cssTagCache.get(R)), g === void 0 && ((this._styleSheet = g = new CSSStyleSheet()).replaceSync(this.cssText), z && cssTagCache.set(R, g));
		}
		return g;
	}
	toString() {
		return this.cssText;
	}
};
const unsafeCSS = (g) => new CSSResult(typeof g == "string" ? g : String(g), void 0, constructionToken), adoptStyles = (g, R) => {
	if (supportsAdoptingStyleSheets) g.adoptedStyleSheets = R.map((g) => g instanceof CSSStyleSheet ? g : g.styleSheet);
	else for (let B of R) {
		let R = document.createElement("style"), V = global$1.litNonce;
		V !== void 0 && R.setAttribute("nonce", V), R.textContent = B.cssText, g.appendChild(R);
	}
};
var cssResultFromStyleSheet = (g) => {
	let R = "";
	for (let z of g.cssRules) R += z.cssText;
	return unsafeCSS(R);
};
const getCompatibleStyle = supportsAdoptingStyleSheets || NODE_MODE ? (g) => g : (g) => g instanceof CSSStyleSheet ? cssResultFromStyleSheet(g) : g;
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var { is, defineProperty, getOwnPropertyDescriptor, getOwnPropertyNames, getOwnPropertySymbols, getPrototypeOf } = Object, global = globalThis, issueWarning$2, trustedTypes = global.trustedTypes, emptyStringForBooleanAttribute = trustedTypes ? trustedTypes.emptyScript : "", polyfillSupport = global.reactiveElementPolyfillSupportDevMode;
global.litIssuedWarnings ??= /* @__PURE__ */ new Set(), issueWarning$2 = (g, R) => {
	R += ` See https://lit.dev/msg/${g} for more information.`, !global.litIssuedWarnings.has(R) && !global.litIssuedWarnings.has(g) && (console.warn(R), global.litIssuedWarnings.add(R));
}, queueMicrotask(() => {
	issueWarning$2("dev-mode", "Lit is in dev mode. Not recommended for production!"), global.ShadyDOM?.inUse && polyfillSupport === void 0 && issueWarning$2("polyfill-support-missing", "Shadow DOM is being polyfilled via `ShadyDOM` but the `polyfill-support` module has not been loaded.");
});
var debugLogEvent = (g) => {
	global.emitLitDebugLogEvents && global.dispatchEvent(new CustomEvent("lit-debug", { detail: g }));
}, JSCompiler_renameProperty = (g, R) => g;
const defaultConverter = {
	toAttribute(g, R) {
		switch (R) {
			case Boolean:
				g = g ? emptyStringForBooleanAttribute : null;
				break;
			case Object:
			case Array:
				g = g == null ? g : JSON.stringify(g);
				break;
		}
		return g;
	},
	fromAttribute(g, R) {
		let z = g;
		switch (R) {
			case Boolean:
				z = g !== null;
				break;
			case Number:
				z = g === null ? null : Number(g);
				break;
			case Object:
			case Array:
				try {
					z = JSON.parse(g);
				} catch {
					z = null;
				}
				break;
		}
		return z;
	}
}, notEqual = (g, R) => !is(g, R);
var defaultPropertyDeclaration$1 = {
	attribute: !0,
	type: String,
	converter: defaultConverter,
	reflect: !1,
	useDefault: !1,
	hasChanged: notEqual
};
Symbol.metadata ??= Symbol("metadata"), global.litPropertyMetadata ??= /* @__PURE__ */ new WeakMap();
var ReactiveElement = class extends HTMLElement {
	static addInitializer(g) {
		this.__prepare(), (this._initializers ??= []).push(g);
	}
	static get observedAttributes() {
		return this.finalize(), this.__attributeToPropertyMap && [...this.__attributeToPropertyMap.keys()];
	}
	static createProperty(g, R = defaultPropertyDeclaration$1) {
		if (R.state && (R.attribute = !1), this.__prepare(), this.prototype.hasOwnProperty(g) && (R = Object.create(R), R.wrapped = !0), this.elementProperties.set(g, R), !R.noAccessor) {
			let z = Symbol.for(`${String(g)} (@property() cache)`), B = this.getPropertyDescriptor(g, z, R);
			B !== void 0 && defineProperty(this.prototype, g, B);
		}
	}
	static getPropertyDescriptor(g, R, z) {
		let { get: B, set: V } = getOwnPropertyDescriptor(this.prototype, g) ?? {
			get() {
				return this[R];
			},
			set(g) {
				this[R] = g;
			}
		};
		if (B == null) {
			if ("value" in (getOwnPropertyDescriptor(this.prototype, g) ?? {})) throw Error(`Field ${JSON.stringify(String(g))} on ${this.name} was declared as a reactive property but it's actually declared as a value on the prototype. Usually this is due to using @property or @state on a method.`);
			issueWarning$2("reactive-property-without-getter", `Field ${JSON.stringify(String(g))} on ${this.name} was declared as a reactive property but it does not have a getter. This will be an error in a future version of Lit.`);
		}
		return {
			get: B,
			set(R) {
				let H = B?.call(this);
				V?.call(this, R), this.requestUpdate(g, H, z);
			},
			configurable: !0,
			enumerable: !0
		};
	}
	static getPropertyOptions(g) {
		return this.elementProperties.get(g) ?? defaultPropertyDeclaration$1;
	}
	static __prepare() {
		if (this.hasOwnProperty(JSCompiler_renameProperty("elementProperties", this))) return;
		let g = getPrototypeOf(this);
		g.finalize(), g._initializers !== void 0 && (this._initializers = [...g._initializers]), this.elementProperties = new Map(g.elementProperties);
	}
	static finalize() {
		if (this.hasOwnProperty(JSCompiler_renameProperty("finalized", this))) return;
		if (this.finalized = !0, this.__prepare(), this.hasOwnProperty(JSCompiler_renameProperty("properties", this))) {
			let g = this.properties, R = [...getOwnPropertyNames(g), ...getOwnPropertySymbols(g)];
			for (let z of R) this.createProperty(z, g[z]);
		}
		let g = this[Symbol.metadata];
		if (g !== null) {
			let R = litPropertyMetadata.get(g);
			if (R !== void 0) for (let [g, z] of R) this.elementProperties.set(g, z);
		}
		this.__attributeToPropertyMap = /* @__PURE__ */ new Map();
		for (let [g, R] of this.elementProperties) {
			let z = this.__attributeNameForProperty(g, R);
			z !== void 0 && this.__attributeToPropertyMap.set(z, g);
		}
		this.elementStyles = this.finalizeStyles(this.styles), this.hasOwnProperty("createProperty") && issueWarning$2("no-override-create-property", "Overriding ReactiveElement.createProperty() is deprecated. The override will not be called with standard decorators"), this.hasOwnProperty("getPropertyDescriptor") && issueWarning$2("no-override-get-property-descriptor", "Overriding ReactiveElement.getPropertyDescriptor() is deprecated. The override will not be called with standard decorators");
	}
	static finalizeStyles(g) {
		let R = [];
		if (Array.isArray(g)) {
			let z = new Set(g.flat(Infinity).reverse());
			for (let g of z) R.unshift(getCompatibleStyle(g));
		} else g !== void 0 && R.push(getCompatibleStyle(g));
		return R;
	}
	static __attributeNameForProperty(g, R) {
		let z = R.attribute;
		return z === !1 ? void 0 : typeof z == "string" ? z : typeof g == "string" ? g.toLowerCase() : void 0;
	}
	constructor() {
		super(), this.__instanceProperties = void 0, this.isUpdatePending = !1, this.hasUpdated = !1, this.__reflectingProperty = null, this.__initialize();
	}
	__initialize() {
		this.__updatePromise = new Promise((g) => this.enableUpdating = g), this._$changedProperties = /* @__PURE__ */ new Map(), this.__saveInstanceProperties(), this.requestUpdate(), this.constructor._initializers?.forEach((g) => g(this));
	}
	addController(g) {
		(this.__controllers ??= /* @__PURE__ */ new Set()).add(g), this.renderRoot !== void 0 && this.isConnected && g.hostConnected?.();
	}
	removeController(g) {
		this.__controllers?.delete(g);
	}
	__saveInstanceProperties() {
		let g = /* @__PURE__ */ new Map(), R = this.constructor.elementProperties;
		for (let z of R.keys()) this.hasOwnProperty(z) && (g.set(z, this[z]), delete this[z]);
		g.size > 0 && (this.__instanceProperties = g);
	}
	createRenderRoot() {
		let g = this.shadowRoot ?? this.attachShadow(this.constructor.shadowRootOptions);
		return adoptStyles(g, this.constructor.elementStyles), g;
	}
	connectedCallback() {
		this.renderRoot ??= this.createRenderRoot(), this.enableUpdating(!0), this.__controllers?.forEach((g) => g.hostConnected?.());
	}
	enableUpdating(g) {}
	disconnectedCallback() {
		this.__controllers?.forEach((g) => g.hostDisconnected?.());
	}
	attributeChangedCallback(g, R, z) {
		this._$attributeToProperty(g, z);
	}
	__propertyToAttribute(g, R) {
		let z = this.constructor.elementProperties.get(g), B = this.constructor.__attributeNameForProperty(g, z);
		if (B !== void 0 && z.reflect === !0) {
			let V = (z.converter?.toAttribute === void 0 ? defaultConverter : z.converter).toAttribute(R, z.type);
			this.constructor.enabledWarnings.includes("migration") && V === void 0 && issueWarning$2("undefined-attribute-value", `The attribute value for the ${g} property is undefined on element ${this.localName}. The attribute will be removed, but in the previous version of \`ReactiveElement\`, the attribute would not have changed.`), this.__reflectingProperty = g, V == null ? this.removeAttribute(B) : this.setAttribute(B, V), this.__reflectingProperty = null;
		}
	}
	_$attributeToProperty(g, R) {
		let z = this.constructor, B = z.__attributeToPropertyMap.get(g);
		if (B !== void 0 && this.__reflectingProperty !== B) {
			let g = z.getPropertyOptions(B), V = typeof g.converter == "function" ? { fromAttribute: g.converter } : g.converter?.fromAttribute === void 0 ? defaultConverter : g.converter;
			this.__reflectingProperty = B;
			let H = V.fromAttribute(R, g.type);
			this[B] = H ?? this.__defaultValues?.get(B) ?? H, this.__reflectingProperty = null;
		}
	}
	requestUpdate(g, R, z, B = !1, V) {
		if (g !== void 0) {
			g instanceof Event && issueWarning$2("", "The requestUpdate() method was called with an Event as the property name. This is probably a mistake caused by binding this.requestUpdate as an event listener. Instead bind a function that will call it with no arguments: () => this.requestUpdate()");
			let H = this.constructor;
			if (B === !1 && (V = this[g]), z ??= H.getPropertyOptions(g), (z.hasChanged ?? notEqual)(V, R) || z.useDefault && z.reflect && V === this.__defaultValues?.get(g) && !this.hasAttribute(H.__attributeNameForProperty(g, z))) this._$changeProperty(g, R, z);
			else return;
		}
		this.isUpdatePending === !1 && (this.__updatePromise = this.__enqueueUpdate());
	}
	_$changeProperty(g, R, { useDefault: z, reflect: B, wrapped: V }, H) {
		z && !(this.__defaultValues ??= /* @__PURE__ */ new Map()).has(g) && (this.__defaultValues.set(g, H ?? R ?? this[g]), V !== !0 || H !== void 0) || (this._$changedProperties.has(g) || (!this.hasUpdated && !z && (R = void 0), this._$changedProperties.set(g, R)), B === !0 && this.__reflectingProperty !== g && (this.__reflectingProperties ??= /* @__PURE__ */ new Set()).add(g));
	}
	async __enqueueUpdate() {
		this.isUpdatePending = !0;
		try {
			await this.__updatePromise;
		} catch (g) {
			Promise.reject(g);
		}
		let g = this.scheduleUpdate();
		return g != null && await g, !this.isUpdatePending;
	}
	scheduleUpdate() {
		let g = this.performUpdate();
		return this.constructor.enabledWarnings.includes("async-perform-update") && typeof g?.then == "function" && issueWarning$2("async-perform-update", `Element ${this.localName} returned a Promise from performUpdate(). This behavior is deprecated and will be removed in a future version of ReactiveElement.`), g;
	}
	performUpdate() {
		if (!this.isUpdatePending) return;
		if (debugLogEvent?.({ kind: "update" }), !this.hasUpdated) {
			this.renderRoot ??= this.createRenderRoot();
			{
				let g = [...this.constructor.elementProperties.keys()].filter((g) => this.hasOwnProperty(g) && g in getPrototypeOf(this));
				if (g.length) throw Error(`The following properties on element ${this.localName} will not trigger updates as expected because they are set using class fields: ${g.join(", ")}. Native class fields and some compiled output will overwrite accessors used for detecting changes. See https://lit.dev/msg/class-field-shadowing for more information.`);
			}
			if (this.__instanceProperties) {
				for (let [g, R] of this.__instanceProperties) this[g] = R;
				this.__instanceProperties = void 0;
			}
			let g = this.constructor.elementProperties;
			if (g.size > 0) for (let [R, z] of g) {
				let { wrapped: g } = z, B = this[R];
				g === !0 && !this._$changedProperties.has(R) && B !== void 0 && this._$changeProperty(R, void 0, z, B);
			}
		}
		let g = !1, R = this._$changedProperties;
		try {
			g = this.shouldUpdate(R), g ? (this.willUpdate(R), this.__controllers?.forEach((g) => g.hostUpdate?.()), this.update(R)) : this.__markUpdated();
		} catch (R) {
			throw g = !1, this.__markUpdated(), R;
		}
		g && this._$didUpdate(R);
	}
	willUpdate(g) {}
	_$didUpdate(g) {
		this.__controllers?.forEach((g) => g.hostUpdated?.()), this.hasUpdated || (this.hasUpdated = !0, this.firstUpdated(g)), this.updated(g), this.isUpdatePending && this.constructor.enabledWarnings.includes("change-in-update") && issueWarning$2("change-in-update", `Element ${this.localName} scheduled an update (generally because a property was set) after an update completed, causing a new update to be scheduled. This is inefficient and should be avoided unless the next update can only be scheduled as a side effect of the previous update.`);
	}
	__markUpdated() {
		this._$changedProperties = /* @__PURE__ */ new Map(), this.isUpdatePending = !1;
	}
	get updateComplete() {
		return this.getUpdateComplete();
	}
	getUpdateComplete() {
		return this.__updatePromise;
	}
	shouldUpdate(g) {
		return !0;
	}
	update(g) {
		this.__reflectingProperties &&= this.__reflectingProperties.forEach((g) => this.__propertyToAttribute(g, this[g])), this.__markUpdated();
	}
	updated(g) {}
	firstUpdated(g) {}
};
ReactiveElement.elementStyles = [], ReactiveElement.shadowRootOptions = { mode: "open" }, ReactiveElement[JSCompiler_renameProperty("elementProperties", ReactiveElement)] = /* @__PURE__ */ new Map(), ReactiveElement[JSCompiler_renameProperty("finalized", ReactiveElement)] = /* @__PURE__ */ new Map(), polyfillSupport?.({ ReactiveElement });
{
	ReactiveElement.enabledWarnings = ["change-in-update", "async-perform-update"];
	let g = function(g) {
		g.hasOwnProperty(JSCompiler_renameProperty("enabledWarnings", g)) || (g.enabledWarnings = g.enabledWarnings.slice());
	};
	ReactiveElement.enableWarning = function(R) {
		g(this), this.enabledWarnings.includes(R) || this.enabledWarnings.push(R);
	}, ReactiveElement.disableWarning = function(R) {
		g(this);
		let z = this.enabledWarnings.indexOf(R);
		z >= 0 && this.enabledWarnings.splice(z, 1);
	};
}
(global.reactiveElementVersions ??= []).push("2.1.2"), global.reactiveElementVersions.length > 1 && queueMicrotask(() => {
	issueWarning$2("multiple-versions", "Multiple versions of Lit loaded. Loading multiple versions is not recommended.");
});
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var issueWarning$1;
globalThis.litIssuedWarnings ??= /* @__PURE__ */ new Set(), issueWarning$1 = (g, R) => {
	R += ` See https://lit.dev/msg/${g} for more information.`, !globalThis.litIssuedWarnings.has(R) && !globalThis.litIssuedWarnings.has(g) && (console.warn(R), globalThis.litIssuedWarnings.add(R));
};
var legacyProperty = (g, R, z) => {
	let B = R.hasOwnProperty(z);
	return R.constructor.createProperty(z, g), B ? Object.getOwnPropertyDescriptor(R, z) : void 0;
}, defaultPropertyDeclaration = {
	attribute: !0,
	type: String,
	converter: defaultConverter,
	reflect: !1,
	hasChanged: notEqual
};
const standardProperty = (g = defaultPropertyDeclaration, R, z) => {
	let { kind: B, metadata: V } = z;
	V ?? issueWarning$1("missing-class-metadata", `The class ${R} is missing decorator metadata. This could mean that you're using a compiler that supports decorators but doesn't support decorator metadata, such as TypeScript 5.1. Please update your compiler.`);
	let H = globalThis.litPropertyMetadata.get(V);
	if (H === void 0 && globalThis.litPropertyMetadata.set(V, H = /* @__PURE__ */ new Map()), B === "setter" && (g = Object.create(g), g.wrapped = !0), H.set(z.name, g), B === "accessor") {
		let { name: B } = z;
		return {
			set(z) {
				let V = R.get.call(this);
				R.set.call(this, z), this.requestUpdate(B, V, g, !0, z);
			},
			init(R) {
				return R !== void 0 && this._$changeProperty(B, void 0, g, R), R;
			}
		};
	} else if (B === "setter") {
		let { name: B } = z;
		return function(z) {
			let V = this[B];
			R.call(this, z), this.requestUpdate(B, V, g, !0, z);
		};
	}
	throw Error(`Unsupported decorator location: ${B}`);
};
function property(g) {
	return (R, z) => typeof z == "object" ? standardProperty(g, R, z) : legacyProperty(g, R, z);
}
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
function state(g) {
	return property({
		...g,
		state: !0,
		attribute: !1
	});
}
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
function eventOptions(g) {
	return ((R, z) => {
		let B = typeof R == "function" ? R : R[z];
		Object.assign(B, g);
	});
}
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
const desc = (g, R, z) => (z.configurable = !0, z.enumerable = !0, Reflect.decorate && typeof R != "object" && Object.defineProperty(g, R, z), z);
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var issueWarning;
globalThis.litIssuedWarnings ??= /* @__PURE__ */ new Set(), issueWarning = (g, R) => {
	R += g ? ` See https://lit.dev/msg/${g} for more information.` : "", !globalThis.litIssuedWarnings.has(R) && !globalThis.litIssuedWarnings.has(g) && (console.warn(R), globalThis.litIssuedWarnings.add(R));
};
function query(g, R) {
	return ((z, B, V) => {
		let H = (z) => {
			let V = z.renderRoot?.querySelector(g) ?? null;
			if (V === null && R && !z.hasUpdated) {
				let R = typeof B == "object" ? B.name : B;
				issueWarning("", `@query'd field ${JSON.stringify(String(R))} with the 'cache' flag set for selector '${g}' has been accessed before the first update and returned null. This is expected if the renderRoot tree has not been provided beforehand (e.g. via Declarative Shadow DOM). Therefore the value hasn't been cached.`);
			}
			return V;
		};
		if (R) {
			let { get: g, set: R } = typeof B == "object" ? z : V ?? (() => {
				let g = Symbol(`${String(B)} (@query() cache)`);
				return {
					get() {
						return this[g];
					},
					set(R) {
						this[g] = R;
					}
				};
			})();
			return desc(z, B, { get() {
				let z = g.call(this);
				return z === void 0 && (z = H(this), (z !== null || this.hasUpdated) && R.call(this, z)), z;
			} });
		} else return desc(z, B, { get() {
			return H(this);
		} });
	});
}
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var fragment;
function queryAll(g) {
	return ((R, z) => desc(R, z, { get() {
		return (this.renderRoot ?? (fragment ??= document.createDocumentFragment())).querySelectorAll(g);
	} }));
}
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
function queryAsync(g) {
	return ((R, z) => desc(R, z, { async get() {
		return await this.updateComplete, this.renderRoot?.querySelector(g) ?? null;
	} }));
}
/**
* @license
* Copyright 2021 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
function queryAssignedElements(g) {
	return ((R, z) => {
		let { slot: B, selector: V } = g ?? {}, H = `slot${B ? `[name=${B}]` : ":not([name])"}`;
		return desc(R, z, { get() {
			let R = (this.renderRoot?.querySelector(H))?.assignedElements(g) ?? [];
			return V === void 0 ? R : R.filter((g) => g.matches(V));
		} });
	});
}
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
function queryAssignedNodes(g) {
	return ((R, z) => {
		let { slot: B } = g ?? {}, V = `slot${B ? `[name=${B}]` : ":not([name])"}`;
		return desc(R, z, { get() {
			return (this.renderRoot?.querySelector(V))?.assignedNodes(g) ?? [];
		} });
	});
}
export { customElement, eventOptions, property, query, queryAll, queryAssignedElements, queryAssignedNodes, queryAsync, standardProperty, state };
