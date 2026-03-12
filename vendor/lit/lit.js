/**
* @license
* Copyright 2019 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var NODE_MODE$1 = !1, global$3 = globalThis;
const supportsAdoptingStyleSheets = global$3.ShadowRoot && (global$3.ShadyCSS === void 0 || global$3.ShadyCSS.nativeShadow) && "adoptedStyleSheets" in Document.prototype && "replace" in CSSStyleSheet.prototype;
var constructionToken = Symbol(), cssTagCache = /* @__PURE__ */ new WeakMap(), CSSResult = class {
	constructor(h, U, W) {
		if (this._$cssResult$ = !0, W !== constructionToken) throw Error("CSSResult is not constructable. Use `unsafeCSS` or `css` instead.");
		this.cssText = h, this._strings = U;
	}
	get styleSheet() {
		let h = this._styleSheet, U = this._strings;
		if (supportsAdoptingStyleSheets && h === void 0) {
			let W = U !== void 0 && U.length === 1;
			W && (h = cssTagCache.get(U)), h === void 0 && ((this._styleSheet = h = new CSSStyleSheet()).replaceSync(this.cssText), W && cssTagCache.set(U, h));
		}
		return h;
	}
	toString() {
		return this.cssText;
	}
}, textFromCSSResult = (h) => {
	if (h._$cssResult$ === !0) return h.cssText;
	if (typeof h == "number") return h;
	throw Error(`Value passed to 'css' function must be a 'css' function result: ${h}. Use 'unsafeCSS' to pass non-literal values, but take care to ensure page security.`);
};
const unsafeCSS = (h) => new CSSResult(typeof h == "string" ? h : String(h), void 0, constructionToken), css = (h, ...U) => new CSSResult(h.length === 1 ? h[0] : U.reduce((U, W, G) => U + textFromCSSResult(W) + h[G + 1], h[0]), h, constructionToken), adoptStyles = (h, G) => {
	if (supportsAdoptingStyleSheets) h.adoptedStyleSheets = G.map((h) => h instanceof CSSStyleSheet ? h : h.styleSheet);
	else for (let W of G) {
		let G = document.createElement("style"), K = global$3.litNonce;
		K !== void 0 && G.setAttribute("nonce", K), G.textContent = W.cssText, h.appendChild(G);
	}
};
var cssResultFromStyleSheet = (h) => {
	let U = "";
	for (let W of h.cssRules) U += W.cssText;
	return unsafeCSS(U);
};
const getCompatibleStyle = supportsAdoptingStyleSheets || NODE_MODE$1 ? (h) => h : (h) => h instanceof CSSStyleSheet ? cssResultFromStyleSheet(h) : h;
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var { is, defineProperty, getOwnPropertyDescriptor, getOwnPropertyNames, getOwnPropertySymbols, getPrototypeOf } = Object, global$2 = globalThis, issueWarning$2, trustedTypes$1 = global$2.trustedTypes, emptyStringForBooleanAttribute = trustedTypes$1 ? trustedTypes$1.emptyScript : "", polyfillSupport$2 = global$2.reactiveElementPolyfillSupportDevMode;
global$2.litIssuedWarnings ??= /* @__PURE__ */ new Set(), issueWarning$2 = (h, U) => {
	U += ` See https://lit.dev/msg/${h} for more information.`, !global$2.litIssuedWarnings.has(U) && !global$2.litIssuedWarnings.has(h) && (console.warn(U), global$2.litIssuedWarnings.add(U));
}, queueMicrotask(() => {
	issueWarning$2("dev-mode", "Lit is in dev mode. Not recommended for production!"), global$2.ShadyDOM?.inUse && polyfillSupport$2 === void 0 && issueWarning$2("polyfill-support-missing", "Shadow DOM is being polyfilled via `ShadyDOM` but the `polyfill-support` module has not been loaded.");
});
var debugLogEvent$1 = (h) => {
	global$2.emitLitDebugLogEvents && global$2.dispatchEvent(new CustomEvent("lit-debug", { detail: h }));
}, JSCompiler_renameProperty$1 = (h, U) => h;
const defaultConverter = {
	toAttribute(h, U) {
		switch (U) {
			case Boolean:
				h = h ? emptyStringForBooleanAttribute : null;
				break;
			case Object:
			case Array:
				h = h == null ? h : JSON.stringify(h);
				break;
		}
		return h;
	},
	fromAttribute(h, U) {
		let W = h;
		switch (U) {
			case Boolean:
				W = h !== null;
				break;
			case Number:
				W = h === null ? null : Number(h);
				break;
			case Object:
			case Array:
				try {
					W = JSON.parse(h);
				} catch {
					W = null;
				}
				break;
		}
		return W;
	}
}, notEqual = (h, U) => !is(h, U);
var defaultPropertyDeclaration = {
	attribute: !0,
	type: String,
	converter: defaultConverter,
	reflect: !1,
	useDefault: !1,
	hasChanged: notEqual
};
Symbol.metadata ??= Symbol("metadata"), global$2.litPropertyMetadata ??= /* @__PURE__ */ new WeakMap();
var ReactiveElement = class extends HTMLElement {
	static addInitializer(h) {
		this.__prepare(), (this._initializers ??= []).push(h);
	}
	static get observedAttributes() {
		return this.finalize(), this.__attributeToPropertyMap && [...this.__attributeToPropertyMap.keys()];
	}
	static createProperty(h, U = defaultPropertyDeclaration) {
		if (U.state && (U.attribute = !1), this.__prepare(), this.prototype.hasOwnProperty(h) && (U = Object.create(U), U.wrapped = !0), this.elementProperties.set(h, U), !U.noAccessor) {
			let W = Symbol.for(`${String(h)} (@property() cache)`), G = this.getPropertyDescriptor(h, W, U);
			G !== void 0 && defineProperty(this.prototype, h, G);
		}
	}
	static getPropertyDescriptor(h, U, W) {
		let { get: G, set: K } = getOwnPropertyDescriptor(this.prototype, h) ?? {
			get() {
				return this[U];
			},
			set(h) {
				this[U] = h;
			}
		};
		if (G == null) {
			if ("value" in (getOwnPropertyDescriptor(this.prototype, h) ?? {})) throw Error(`Field ${JSON.stringify(String(h))} on ${this.name} was declared as a reactive property but it's actually declared as a value on the prototype. Usually this is due to using @property or @state on a method.`);
			issueWarning$2("reactive-property-without-getter", `Field ${JSON.stringify(String(h))} on ${this.name} was declared as a reactive property but it does not have a getter. This will be an error in a future version of Lit.`);
		}
		return {
			get: G,
			set(U) {
				let q = G?.call(this);
				K?.call(this, U), this.requestUpdate(h, q, W);
			},
			configurable: !0,
			enumerable: !0
		};
	}
	static getPropertyOptions(h) {
		return this.elementProperties.get(h) ?? defaultPropertyDeclaration;
	}
	static __prepare() {
		if (this.hasOwnProperty(JSCompiler_renameProperty$1("elementProperties", this))) return;
		let h = getPrototypeOf(this);
		h.finalize(), h._initializers !== void 0 && (this._initializers = [...h._initializers]), this.elementProperties = new Map(h.elementProperties);
	}
	static finalize() {
		if (this.hasOwnProperty(JSCompiler_renameProperty$1("finalized", this))) return;
		if (this.finalized = !0, this.__prepare(), this.hasOwnProperty(JSCompiler_renameProperty$1("properties", this))) {
			let h = this.properties, U = [...getOwnPropertyNames(h), ...getOwnPropertySymbols(h)];
			for (let W of U) this.createProperty(W, h[W]);
		}
		let h = this[Symbol.metadata];
		if (h !== null) {
			let U = litPropertyMetadata.get(h);
			if (U !== void 0) for (let [h, W] of U) this.elementProperties.set(h, W);
		}
		this.__attributeToPropertyMap = /* @__PURE__ */ new Map();
		for (let [h, U] of this.elementProperties) {
			let W = this.__attributeNameForProperty(h, U);
			W !== void 0 && this.__attributeToPropertyMap.set(W, h);
		}
		this.elementStyles = this.finalizeStyles(this.styles), this.hasOwnProperty("createProperty") && issueWarning$2("no-override-create-property", "Overriding ReactiveElement.createProperty() is deprecated. The override will not be called with standard decorators"), this.hasOwnProperty("getPropertyDescriptor") && issueWarning$2("no-override-get-property-descriptor", "Overriding ReactiveElement.getPropertyDescriptor() is deprecated. The override will not be called with standard decorators");
	}
	static finalizeStyles(h) {
		let U = [];
		if (Array.isArray(h)) {
			let W = new Set(h.flat(Infinity).reverse());
			for (let h of W) U.unshift(getCompatibleStyle(h));
		} else h !== void 0 && U.push(getCompatibleStyle(h));
		return U;
	}
	static __attributeNameForProperty(h, U) {
		let W = U.attribute;
		return W === !1 ? void 0 : typeof W == "string" ? W : typeof h == "string" ? h.toLowerCase() : void 0;
	}
	constructor() {
		super(), this.__instanceProperties = void 0, this.isUpdatePending = !1, this.hasUpdated = !1, this.__reflectingProperty = null, this.__initialize();
	}
	__initialize() {
		this.__updatePromise = new Promise((h) => this.enableUpdating = h), this._$changedProperties = /* @__PURE__ */ new Map(), this.__saveInstanceProperties(), this.requestUpdate(), this.constructor._initializers?.forEach((h) => h(this));
	}
	addController(h) {
		(this.__controllers ??= /* @__PURE__ */ new Set()).add(h), this.renderRoot !== void 0 && this.isConnected && h.hostConnected?.();
	}
	removeController(h) {
		this.__controllers?.delete(h);
	}
	__saveInstanceProperties() {
		let h = /* @__PURE__ */ new Map(), U = this.constructor.elementProperties;
		for (let W of U.keys()) this.hasOwnProperty(W) && (h.set(W, this[W]), delete this[W]);
		h.size > 0 && (this.__instanceProperties = h);
	}
	createRenderRoot() {
		let h = this.shadowRoot ?? this.attachShadow(this.constructor.shadowRootOptions);
		return adoptStyles(h, this.constructor.elementStyles), h;
	}
	connectedCallback() {
		this.renderRoot ??= this.createRenderRoot(), this.enableUpdating(!0), this.__controllers?.forEach((h) => h.hostConnected?.());
	}
	enableUpdating(h) {}
	disconnectedCallback() {
		this.__controllers?.forEach((h) => h.hostDisconnected?.());
	}
	attributeChangedCallback(h, U, W) {
		this._$attributeToProperty(h, W);
	}
	__propertyToAttribute(h, U) {
		let W = this.constructor.elementProperties.get(h), G = this.constructor.__attributeNameForProperty(h, W);
		if (G !== void 0 && W.reflect === !0) {
			let K = (W.converter?.toAttribute === void 0 ? defaultConverter : W.converter).toAttribute(U, W.type);
			this.constructor.enabledWarnings.includes("migration") && K === void 0 && issueWarning$2("undefined-attribute-value", `The attribute value for the ${h} property is undefined on element ${this.localName}. The attribute will be removed, but in the previous version of \`ReactiveElement\`, the attribute would not have changed.`), this.__reflectingProperty = h, K == null ? this.removeAttribute(G) : this.setAttribute(G, K), this.__reflectingProperty = null;
		}
	}
	_$attributeToProperty(h, U) {
		let W = this.constructor, G = W.__attributeToPropertyMap.get(h);
		if (G !== void 0 && this.__reflectingProperty !== G) {
			let h = W.getPropertyOptions(G), K = typeof h.converter == "function" ? { fromAttribute: h.converter } : h.converter?.fromAttribute === void 0 ? defaultConverter : h.converter;
			this.__reflectingProperty = G;
			let q = K.fromAttribute(U, h.type);
			this[G] = q ?? this.__defaultValues?.get(G) ?? q, this.__reflectingProperty = null;
		}
	}
	requestUpdate(h, U, W, G = !1, K) {
		if (h !== void 0) {
			h instanceof Event && issueWarning$2("", "The requestUpdate() method was called with an Event as the property name. This is probably a mistake caused by binding this.requestUpdate as an event listener. Instead bind a function that will call it with no arguments: () => this.requestUpdate()");
			let q = this.constructor;
			if (G === !1 && (K = this[h]), W ??= q.getPropertyOptions(h), (W.hasChanged ?? notEqual)(K, U) || W.useDefault && W.reflect && K === this.__defaultValues?.get(h) && !this.hasAttribute(q.__attributeNameForProperty(h, W))) this._$changeProperty(h, U, W);
			else return;
		}
		this.isUpdatePending === !1 && (this.__updatePromise = this.__enqueueUpdate());
	}
	_$changeProperty(h, U, { useDefault: W, reflect: G, wrapped: K }, q) {
		W && !(this.__defaultValues ??= /* @__PURE__ */ new Map()).has(h) && (this.__defaultValues.set(h, q ?? U ?? this[h]), K !== !0 || q !== void 0) || (this._$changedProperties.has(h) || (!this.hasUpdated && !W && (U = void 0), this._$changedProperties.set(h, U)), G === !0 && this.__reflectingProperty !== h && (this.__reflectingProperties ??= /* @__PURE__ */ new Set()).add(h));
	}
	async __enqueueUpdate() {
		this.isUpdatePending = !0;
		try {
			await this.__updatePromise;
		} catch (h) {
			Promise.reject(h);
		}
		let h = this.scheduleUpdate();
		return h != null && await h, !this.isUpdatePending;
	}
	scheduleUpdate() {
		let h = this.performUpdate();
		return this.constructor.enabledWarnings.includes("async-perform-update") && typeof h?.then == "function" && issueWarning$2("async-perform-update", `Element ${this.localName} returned a Promise from performUpdate(). This behavior is deprecated and will be removed in a future version of ReactiveElement.`), h;
	}
	performUpdate() {
		if (!this.isUpdatePending) return;
		if (debugLogEvent$1?.({ kind: "update" }), !this.hasUpdated) {
			this.renderRoot ??= this.createRenderRoot();
			{
				let h = [...this.constructor.elementProperties.keys()].filter((h) => this.hasOwnProperty(h) && h in getPrototypeOf(this));
				if (h.length) throw Error(`The following properties on element ${this.localName} will not trigger updates as expected because they are set using class fields: ${h.join(", ")}. Native class fields and some compiled output will overwrite accessors used for detecting changes. See https://lit.dev/msg/class-field-shadowing for more information.`);
			}
			if (this.__instanceProperties) {
				for (let [h, U] of this.__instanceProperties) this[h] = U;
				this.__instanceProperties = void 0;
			}
			let h = this.constructor.elementProperties;
			if (h.size > 0) for (let [U, W] of h) {
				let { wrapped: h } = W, G = this[U];
				h === !0 && !this._$changedProperties.has(U) && G !== void 0 && this._$changeProperty(U, void 0, W, G);
			}
		}
		let h = !1, U = this._$changedProperties;
		try {
			h = this.shouldUpdate(U), h ? (this.willUpdate(U), this.__controllers?.forEach((h) => h.hostUpdate?.()), this.update(U)) : this.__markUpdated();
		} catch (U) {
			throw h = !1, this.__markUpdated(), U;
		}
		h && this._$didUpdate(U);
	}
	willUpdate(h) {}
	_$didUpdate(h) {
		this.__controllers?.forEach((h) => h.hostUpdated?.()), this.hasUpdated || (this.hasUpdated = !0, this.firstUpdated(h)), this.updated(h), this.isUpdatePending && this.constructor.enabledWarnings.includes("change-in-update") && issueWarning$2("change-in-update", `Element ${this.localName} scheduled an update (generally because a property was set) after an update completed, causing a new update to be scheduled. This is inefficient and should be avoided unless the next update can only be scheduled as a side effect of the previous update.`);
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
	shouldUpdate(h) {
		return !0;
	}
	update(h) {
		this.__reflectingProperties &&= this.__reflectingProperties.forEach((h) => this.__propertyToAttribute(h, this[h])), this.__markUpdated();
	}
	updated(h) {}
	firstUpdated(h) {}
};
ReactiveElement.elementStyles = [], ReactiveElement.shadowRootOptions = { mode: "open" }, ReactiveElement[JSCompiler_renameProperty$1("elementProperties", ReactiveElement)] = /* @__PURE__ */ new Map(), ReactiveElement[JSCompiler_renameProperty$1("finalized", ReactiveElement)] = /* @__PURE__ */ new Map(), polyfillSupport$2?.({ ReactiveElement });
{
	ReactiveElement.enabledWarnings = ["change-in-update", "async-perform-update"];
	let h = function(h) {
		h.hasOwnProperty(JSCompiler_renameProperty$1("enabledWarnings", h)) || (h.enabledWarnings = h.enabledWarnings.slice());
	};
	ReactiveElement.enableWarning = function(U) {
		h(this), this.enabledWarnings.includes(U) || this.enabledWarnings.push(U);
	}, ReactiveElement.disableWarning = function(U) {
		h(this);
		let W = this.enabledWarnings.indexOf(U);
		W >= 0 && this.enabledWarnings.splice(W, 1);
	};
}
(global$2.reactiveElementVersions ??= []).push("2.1.2"), global$2.reactiveElementVersions.length > 1 && queueMicrotask(() => {
	issueWarning$2("multiple-versions", "Multiple versions of Lit loaded. Loading multiple versions is not recommended.");
});
var global$1 = globalThis, debugLogEvent = (h) => {
	global$1.emitLitDebugLogEvents && global$1.dispatchEvent(new CustomEvent("lit-debug", { detail: h }));
}, debugLogRenderId = 0, issueWarning$1;
global$1.litIssuedWarnings ??= /* @__PURE__ */ new Set(), issueWarning$1 = (h, U) => {
	U += h ? ` See https://lit.dev/msg/${h} for more information.` : "", !global$1.litIssuedWarnings.has(U) && !global$1.litIssuedWarnings.has(h) && (console.warn(U), global$1.litIssuedWarnings.add(U));
}, queueMicrotask(() => {
	issueWarning$1("dev-mode", "Lit is in dev mode. Not recommended for production!");
});
var wrap = global$1.ShadyDOM?.inUse && global$1.ShadyDOM?.noPatch === !0 ? global$1.ShadyDOM.wrap : (h) => h, trustedTypes = global$1.trustedTypes, policy = trustedTypes ? trustedTypes.createPolicy("lit-html", { createHTML: (h) => h }) : void 0, identityFunction = (h) => h, noopSanitizer = (h, U, W) => identityFunction, setSanitizer = (h) => {
	if (sanitizerFactoryInternal !== noopSanitizer) throw Error("Attempted to overwrite existing lit-html security policy. setSanitizeDOMValueFactory should be called at most once.");
	sanitizerFactoryInternal = h;
}, _testOnlyClearSanitizerFactoryDoNotCallOrElse = () => {
	sanitizerFactoryInternal = noopSanitizer;
}, createSanitizer = (h, U, W) => sanitizerFactoryInternal(h, U, W), boundAttributeSuffix = "$lit$", marker = `lit$${Math.random().toFixed(9).slice(2)}$`, markerMatch = "?" + marker, nodeMarker = `<${markerMatch}>`, d = document, createMarker = () => d.createComment(""), isPrimitive = (h) => h === null || typeof h != "object" && typeof h != "function", isArray = Array.isArray, isIterable = (h) => isArray(h) || typeof h?.[Symbol.iterator] == "function", SPACE_CHAR = "[ 	\n\f\r]", ATTR_VALUE_CHAR = "[^ 	\n\f\r\"'`<>=]", NAME_CHAR = "[^\\s\"'>=/]", textEndRegex = /<(?:(!--|\/[^a-zA-Z])|(\/?[a-zA-Z][^>\s]*)|(\/?$))/g, COMMENT_START = 1, TAG_NAME = 2, DYNAMIC_TAG_NAME = 3, commentEndRegex = /-->/g, comment2EndRegex = />/g, tagEndRegex = RegExp(`>|${SPACE_CHAR}(?:(${NAME_CHAR}+)(${SPACE_CHAR}*=${SPACE_CHAR}*(?:${ATTR_VALUE_CHAR}|("|')|))|$)`, "g"), ENTIRE_MATCH = 0, ATTRIBUTE_NAME = 1, SPACES_AND_EQUALS = 2, QUOTE_CHAR = 3, singleQuoteAttrEndRegex = /'/g, doubleQuoteAttrEndRegex = /"/g, rawTextElement = /^(?:script|style|textarea|title)$/i, HTML_RESULT = 1, SVG_RESULT = 2, MATHML_RESULT = 3, ATTRIBUTE_PART = 1, CHILD_PART = 2, PROPERTY_PART = 3, BOOLEAN_ATTRIBUTE_PART = 4, EVENT_PART = 5, ELEMENT_PART = 6, COMMENT_PART = 7, tag = (h) => (U, ...W) => (U.some((h) => h === void 0) && console.warn("Some template strings are undefined.\nThis is probably caused by illegal octal escape sequences."), W.some((h) => h?._$litStatic$) && issueWarning$1("", "Static values 'literal' or 'unsafeStatic' cannot be used as values to non-static templates.\nPlease use the static 'html' tag function. See https://lit.dev/docs/templates/expressions/#static-expressions"), {
	_$litType$: h,
	strings: U,
	values: W
});
const html = tag(HTML_RESULT), svg = tag(SVG_RESULT), mathml = tag(MATHML_RESULT), noChange = Symbol.for("lit-noChange"), nothing = Symbol.for("lit-nothing");
var templateCache = /* @__PURE__ */ new WeakMap(), walker = d.createTreeWalker(d, 129), sanitizerFactoryInternal = noopSanitizer;
function trustFromTemplateString(h, U) {
	if (!isArray(h) || !h.hasOwnProperty("raw")) {
		let h = "invalid template strings array";
		throw h = "Internal Error: expected template strings to be an array\n          with a 'raw' field. Faking a template strings array by\n          calling html or svg like an ordinary function is effectively\n          the same as calling unsafeHtml and can lead to major security\n          issues, e.g. opening your code up to XSS attacks.\n          If you're using the html or svg tagged template functions normally\n          and still seeing this error, please file a bug at\n          https://github.com/lit/lit/issues/new?template=bug_report.md\n          and include information about your build tooling, if any.".replace(/\n */g, "\n"), Error(h);
	}
	return policy === void 0 ? U : policy.createHTML(U);
}
var getTemplateHtml = (h, U) => {
	let W = h.length - 1, G = [], K = U === SVG_RESULT ? "<svg>" : U === MATHML_RESULT ? "<math>" : "", q, J = textEndRegex;
	for (let U = 0; U < W; U++) {
		let W = h[U], Y = -1, X, Z = 0, Q;
		for (; Z < W.length && (J.lastIndex = Z, Q = J.exec(W), Q !== null);) if (Z = J.lastIndex, J === textEndRegex) {
			if (Q[COMMENT_START] === "!--") J = commentEndRegex;
			else if (Q[COMMENT_START] !== void 0) J = comment2EndRegex;
			else if (Q[TAG_NAME] !== void 0) rawTextElement.test(Q[TAG_NAME]) && (q = RegExp(`</${Q[TAG_NAME]}`, "g")), J = tagEndRegex;
			else if (Q[DYNAMIC_TAG_NAME] !== void 0) throw Error("Bindings in tag names are not supported. Please use static templates instead. See https://lit.dev/docs/templates/expressions/#static-expressions");
		} else J === tagEndRegex ? Q[ENTIRE_MATCH] === ">" ? (J = q ?? textEndRegex, Y = -1) : Q[ATTRIBUTE_NAME] === void 0 ? Y = -2 : (Y = J.lastIndex - Q[SPACES_AND_EQUALS].length, X = Q[ATTRIBUTE_NAME], J = Q[QUOTE_CHAR] === void 0 ? tagEndRegex : Q[QUOTE_CHAR] === "\"" ? doubleQuoteAttrEndRegex : singleQuoteAttrEndRegex) : J === doubleQuoteAttrEndRegex || J === singleQuoteAttrEndRegex ? J = tagEndRegex : J === commentEndRegex || J === comment2EndRegex ? J = textEndRegex : (J = tagEndRegex, q = void 0);
		console.assert(Y === -1 || J === tagEndRegex || J === singleQuoteAttrEndRegex || J === doubleQuoteAttrEndRegex, "unexpected parse state B");
		let $ = J === tagEndRegex && h[U + 1].startsWith("/>") ? " " : "";
		K += J === textEndRegex ? W + nodeMarker : Y >= 0 ? (G.push(X), W.slice(0, Y) + boundAttributeSuffix + W.slice(Y)) + marker + $ : W + marker + (Y === -2 ? U : $);
	}
	return [trustFromTemplateString(h, K + (h[W] || "<?>") + (U === SVG_RESULT ? "</svg>" : U === MATHML_RESULT ? "</math>" : "")), G];
}, Template = class h {
	constructor({ strings: U, _$litType$: W }, G) {
		this.parts = [];
		let K, q = 0, J = 0, Y = U.length - 1, X = this.parts, [Z, Q] = getTemplateHtml(U, W);
		if (this.el = h.createElement(Z, G), walker.currentNode = this.el.content, W === SVG_RESULT || W === MATHML_RESULT) {
			let h = this.el.content.firstChild;
			h.replaceWith(...h.childNodes);
		}
		for (; (K = walker.nextNode()) !== null && X.length < Y;) {
			if (K.nodeType === 1) {
				{
					let h = K.localName;
					if (/^(?:textarea|template)$/i.test(h) && K.innerHTML.includes(marker)) {
						let U = `Expressions are not supported inside \`${h}\` elements. See https://lit.dev/msg/expression-in-${h} for more information.`;
						if (h === "template") throw Error(U);
						issueWarning$1("", U);
					}
				}
				if (K.hasAttributes()) for (let h of K.getAttributeNames()) if (h.endsWith(boundAttributeSuffix)) {
					let U = Q[J++], W = K.getAttribute(h).split(marker), G = /([.?@])?(.*)/.exec(U);
					X.push({
						type: ATTRIBUTE_PART,
						index: q,
						name: G[2],
						strings: W,
						ctor: G[1] === "." ? PropertyPart : G[1] === "?" ? BooleanAttributePart : G[1] === "@" ? EventPart : AttributePart
					}), K.removeAttribute(h);
				} else h.startsWith(marker) && (X.push({
					type: ELEMENT_PART,
					index: q
				}), K.removeAttribute(h));
				if (rawTextElement.test(K.tagName)) {
					let h = K.textContent.split(marker), U = h.length - 1;
					if (U > 0) {
						K.textContent = trustedTypes ? trustedTypes.emptyScript : "";
						for (let W = 0; W < U; W++) K.append(h[W], createMarker()), walker.nextNode(), X.push({
							type: CHILD_PART,
							index: ++q
						});
						K.append(h[U], createMarker());
					}
				}
			} else if (K.nodeType === 8) if (K.data === markerMatch) X.push({
				type: CHILD_PART,
				index: q
			});
			else {
				let h = -1;
				for (; (h = K.data.indexOf(marker, h + 1)) !== -1;) X.push({
					type: COMMENT_PART,
					index: q
				}), h += marker.length - 1;
			}
			q++;
		}
		if (Q.length !== J) throw Error("Detected duplicate attribute bindings. This occurs if your template has duplicate attributes on an element tag. For example \"<input ?disabled=${true} ?disabled=${false}>\" contains a duplicate \"disabled\" attribute. The error was detected in the following template: \n`" + U.join("${...}") + "`");
		debugLogEvent && debugLogEvent({
			kind: "template prep",
			template: this,
			clonableTemplate: this.el,
			parts: this.parts,
			strings: U
		});
	}
	static createElement(h, U) {
		let W = d.createElement("template");
		return W.innerHTML = h, W;
	}
};
function resolveDirective(h, U, W = h, G) {
	if (U === noChange) return U;
	let K = G === void 0 ? W.__directive : W.__directives?.[G], q = isPrimitive(U) ? void 0 : U._$litDirective$;
	return K?.constructor !== q && (K?._$notifyDirectiveConnectionChanged?.(!1), q === void 0 ? K = void 0 : (K = new q(h), K._$initialize(h, W, G)), G === void 0 ? W.__directive = K : (W.__directives ??= [])[G] = K), K !== void 0 && (U = resolveDirective(h, K._$resolve(h, U.values), K, G)), U;
}
var TemplateInstance = class {
	constructor(h, U) {
		this._$parts = [], this._$disconnectableChildren = void 0, this._$template = h, this._$parent = U;
	}
	get parentNode() {
		return this._$parent.parentNode;
	}
	get _$isConnected() {
		return this._$parent._$isConnected;
	}
	_clone(h) {
		let { el: { content: U }, parts: W } = this._$template, G = (h?.creationScope ?? d).importNode(U, !0);
		walker.currentNode = G;
		let K = walker.nextNode(), q = 0, J = 0, Y = W[0];
		for (; Y !== void 0;) {
			if (q === Y.index) {
				let U;
				Y.type === CHILD_PART ? U = new ChildPart(K, K.nextSibling, this, h) : Y.type === ATTRIBUTE_PART ? U = new Y.ctor(K, Y.name, Y.strings, this, h) : Y.type === ELEMENT_PART && (U = new ElementPart(K, this, h)), this._$parts.push(U), Y = W[++J];
			}
			q !== Y?.index && (K = walker.nextNode(), q++);
		}
		return walker.currentNode = d, G;
	}
	_update(h) {
		let U = 0;
		for (let W of this._$parts) W !== void 0 && (debugLogEvent && debugLogEvent({
			kind: "set part",
			part: W,
			value: h[U],
			valueIndex: U,
			values: h,
			templateInstance: this
		}), W.strings === void 0 ? W._$setValue(h[U]) : (W._$setValue(h, W, U), U += W.strings.length - 2)), U++;
	}
}, ChildPart = class h {
	get _$isConnected() {
		return this._$parent?._$isConnected ?? this.__isConnected;
	}
	constructor(h, U, W, G) {
		this.type = CHILD_PART, this._$committedValue = nothing, this._$disconnectableChildren = void 0, this._$startNode = h, this._$endNode = U, this._$parent = W, this.options = G, this.__isConnected = G?.isConnected ?? !0, this._textSanitizer = void 0;
	}
	get parentNode() {
		let h = wrap(this._$startNode).parentNode, U = this._$parent;
		return U !== void 0 && h?.nodeType === 11 && (h = U.parentNode), h;
	}
	get startNode() {
		return this._$startNode;
	}
	get endNode() {
		return this._$endNode;
	}
	_$setValue(h, U = this) {
		if (this.parentNode === null) throw Error("This `ChildPart` has no `parentNode` and therefore cannot accept a value. This likely means the element containing the part was manipulated in an unsupported way outside of Lit's control such that the part's marker nodes were ejected from DOM. For example, setting the element's `innerHTML` or `textContent` can do this.");
		if (h = resolveDirective(this, h, U), isPrimitive(h)) h === nothing || h == null || h === "" ? (this._$committedValue !== nothing && (debugLogEvent && debugLogEvent({
			kind: "commit nothing to child",
			start: this._$startNode,
			end: this._$endNode,
			parent: this._$parent,
			options: this.options
		}), this._$clear()), this._$committedValue = nothing) : h !== this._$committedValue && h !== noChange && this._commitText(h);
		else if (h._$litType$ !== void 0) this._commitTemplateResult(h);
		else if (h.nodeType !== void 0) {
			if (this.options?.host === h) {
				this._commitText("[probable mistake: rendered a template's host in itself (commonly caused by writing ${this} in a template]"), console.warn("Attempted to render the template host", h, "inside itself. This is almost always a mistake, and in dev mode ", "we render some warning text. In production however, we'll ", "render it, which will usually result in an error, and sometimes ", "in the element disappearing from the DOM.");
				return;
			}
			this._commitNode(h);
		} else isIterable(h) ? this._commitIterable(h) : this._commitText(h);
	}
	_insert(h) {
		return wrap(wrap(this._$startNode).parentNode).insertBefore(h, this._$endNode);
	}
	_commitNode(h) {
		if (this._$committedValue !== h) {
			if (this._$clear(), sanitizerFactoryInternal !== noopSanitizer) {
				let h = this._$startNode.parentNode?.nodeName;
				if (h === "STYLE" || h === "SCRIPT") {
					let U = "Forbidden";
					throw U = h === "STYLE" ? "Lit does not support binding inside style nodes. This is a security risk, as style injection attacks can exfiltrate data and spoof UIs. Consider instead using css`...` literals to compose styles, and do dynamic styling with css custom properties, ::parts, <slot>s, and by mutating the DOM rather than stylesheets." : "Lit does not support binding inside script nodes. This is a security risk, as it could allow arbitrary code execution.", Error(U);
				}
			}
			debugLogEvent && debugLogEvent({
				kind: "commit node",
				start: this._$startNode,
				parent: this._$parent,
				value: h,
				options: this.options
			}), this._$committedValue = this._insert(h);
		}
	}
	_commitText(h) {
		if (this._$committedValue !== nothing && isPrimitive(this._$committedValue)) {
			let U = wrap(this._$startNode).nextSibling;
			this._textSanitizer === void 0 && (this._textSanitizer = createSanitizer(U, "data", "property")), h = this._textSanitizer(h), debugLogEvent && debugLogEvent({
				kind: "commit text",
				node: U,
				value: h,
				options: this.options
			}), U.data = h;
		} else {
			let U = d.createTextNode("");
			this._commitNode(U), this._textSanitizer === void 0 && (this._textSanitizer = createSanitizer(U, "data", "property")), h = this._textSanitizer(h), debugLogEvent && debugLogEvent({
				kind: "commit text",
				node: U,
				value: h,
				options: this.options
			}), U.data = h;
		}
		this._$committedValue = h;
	}
	_commitTemplateResult(h) {
		let { values: U, _$litType$: W } = h, G = typeof W == "number" ? this._$getTemplate(h) : (W.el === void 0 && (W.el = Template.createElement(trustFromTemplateString(W.h, W.h[0]), this.options)), W);
		if (this._$committedValue?._$template === G) debugLogEvent && debugLogEvent({
			kind: "template updating",
			template: G,
			instance: this._$committedValue,
			parts: this._$committedValue._$parts,
			options: this.options,
			values: U
		}), this._$committedValue._update(U);
		else {
			let h = new TemplateInstance(G, this), W = h._clone(this.options);
			debugLogEvent && debugLogEvent({
				kind: "template instantiated",
				template: G,
				instance: h,
				parts: h._$parts,
				options: this.options,
				fragment: W,
				values: U
			}), h._update(U), debugLogEvent && debugLogEvent({
				kind: "template instantiated and updated",
				template: G,
				instance: h,
				parts: h._$parts,
				options: this.options,
				fragment: W,
				values: U
			}), this._commitNode(W), this._$committedValue = h;
		}
	}
	_$getTemplate(h) {
		let U = templateCache.get(h.strings);
		return U === void 0 && templateCache.set(h.strings, U = new Template(h)), U;
	}
	_commitIterable(U) {
		isArray(this._$committedValue) || (this._$committedValue = [], this._$clear());
		let W = this._$committedValue, G = 0, K;
		for (let q of U) G === W.length ? W.push(K = new h(this._insert(createMarker()), this._insert(createMarker()), this, this.options)) : K = W[G], K._$setValue(q), G++;
		G < W.length && (this._$clear(K && wrap(K._$endNode).nextSibling, G), W.length = G);
	}
	_$clear(h = wrap(this._$startNode).nextSibling, U) {
		for (this._$notifyConnectionChanged?.(!1, !0, U); h !== this._$endNode;) {
			let U = wrap(h).nextSibling;
			wrap(h).remove(), h = U;
		}
	}
	setConnected(h) {
		if (this._$parent === void 0) this.__isConnected = h, this._$notifyConnectionChanged?.(h);
		else throw Error("part.setConnected() may only be called on a RootPart returned from render().");
	}
}, AttributePart = class {
	get tagName() {
		return this.element.tagName;
	}
	get _$isConnected() {
		return this._$parent._$isConnected;
	}
	constructor(h, U, W, G, K) {
		this.type = ATTRIBUTE_PART, this._$committedValue = nothing, this._$disconnectableChildren = void 0, this.element = h, this.name = U, this._$parent = G, this.options = K, W.length > 2 || W[0] !== "" || W[1] !== "" ? (this._$committedValue = Array(W.length - 1).fill(/* @__PURE__ */ new String()), this.strings = W) : this._$committedValue = nothing, this._sanitizer = void 0;
	}
	_$setValue(h, U = this, W, G) {
		let K = this.strings, q = !1;
		if (K === void 0) h = resolveDirective(this, h, U, 0), q = !isPrimitive(h) || h !== this._$committedValue && h !== noChange, q && (this._$committedValue = h);
		else {
			let G = h;
			h = K[0];
			let J, Y;
			for (J = 0; J < K.length - 1; J++) Y = resolveDirective(this, G[W + J], U, J), Y === noChange && (Y = this._$committedValue[J]), q ||= !isPrimitive(Y) || Y !== this._$committedValue[J], Y === nothing ? h = nothing : h !== nothing && (h += (Y ?? "") + K[J + 1]), this._$committedValue[J] = Y;
		}
		q && !G && this._commitValue(h);
	}
	_commitValue(h) {
		h === nothing ? wrap(this.element).removeAttribute(this.name) : (this._sanitizer === void 0 && (this._sanitizer = sanitizerFactoryInternal(this.element, this.name, "attribute")), h = this._sanitizer(h ?? ""), debugLogEvent && debugLogEvent({
			kind: "commit attribute",
			element: this.element,
			name: this.name,
			value: h,
			options: this.options
		}), wrap(this.element).setAttribute(this.name, h ?? ""));
	}
}, PropertyPart = class extends AttributePart {
	constructor() {
		super(...arguments), this.type = PROPERTY_PART;
	}
	_commitValue(h) {
		this._sanitizer === void 0 && (this._sanitizer = sanitizerFactoryInternal(this.element, this.name, "property")), h = this._sanitizer(h), debugLogEvent && debugLogEvent({
			kind: "commit property",
			element: this.element,
			name: this.name,
			value: h,
			options: this.options
		}), this.element[this.name] = h === nothing ? void 0 : h;
	}
}, BooleanAttributePart = class extends AttributePart {
	constructor() {
		super(...arguments), this.type = BOOLEAN_ATTRIBUTE_PART;
	}
	_commitValue(h) {
		debugLogEvent && debugLogEvent({
			kind: "commit boolean attribute",
			element: this.element,
			name: this.name,
			value: !!(h && h !== nothing),
			options: this.options
		}), wrap(this.element).toggleAttribute(this.name, !!h && h !== nothing);
	}
}, EventPart = class extends AttributePart {
	constructor(h, U, W, G, K) {
		if (super(h, U, W, G, K), this.type = EVENT_PART, this.strings !== void 0) throw Error(`A \`<${h.localName}>\` has a \`@${U}=...\` listener with invalid content. Event listeners in templates must have exactly one expression and no surrounding text.`);
	}
	_$setValue(h, U = this) {
		if (h = resolveDirective(this, h, U, 0) ?? nothing, h === noChange) return;
		let W = this._$committedValue, G = h === nothing && W !== nothing || h.capture !== W.capture || h.once !== W.once || h.passive !== W.passive, K = h !== nothing && (W === nothing || G);
		debugLogEvent && debugLogEvent({
			kind: "commit event listener",
			element: this.element,
			name: this.name,
			value: h,
			options: this.options,
			removeListener: G,
			addListener: K,
			oldListener: W
		}), G && this.element.removeEventListener(this.name, this, W), K && this.element.addEventListener(this.name, this, h), this._$committedValue = h;
	}
	handleEvent(h) {
		typeof this._$committedValue == "function" ? this._$committedValue.call(this.options?.host ?? this.element, h) : this._$committedValue.handleEvent(h);
	}
}, ElementPart = class {
	constructor(h, U, W) {
		this.element = h, this.type = ELEMENT_PART, this._$disconnectableChildren = void 0, this._$parent = U, this.options = W;
	}
	get _$isConnected() {
		return this._$parent._$isConnected;
	}
	_$setValue(h) {
		debugLogEvent && debugLogEvent({
			kind: "commit to element binding",
			element: this.element,
			value: h,
			options: this.options
		}), resolveDirective(this, h);
	}
};
const _$LH = {
	_boundAttributeSuffix: boundAttributeSuffix,
	_marker: marker,
	_markerMatch: markerMatch,
	_HTML_RESULT: HTML_RESULT,
	_getTemplateHtml: getTemplateHtml,
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
var polyfillSupport$1 = global$1.litHtmlPolyfillSupportDevMode;
polyfillSupport$1?.(Template, ChildPart), (global$1.litHtmlVersions ??= []).push("3.3.2"), global$1.litHtmlVersions.length > 1 && queueMicrotask(() => {
	issueWarning$1("multiple-versions", "Multiple versions of Lit loaded. Loading multiple versions is not recommended.");
});
const render = (h, U, W) => {
	if (U == null) throw TypeError(`The container to render into may not be ${U}`);
	let G = debugLogRenderId++, K = W?.renderBefore ?? U, q = K._$litPart$;
	if (debugLogEvent && debugLogEvent({
		kind: "begin render",
		id: G,
		value: h,
		container: U,
		options: W,
		part: q
	}), q === void 0) {
		let h = W?.renderBefore ?? null;
		K._$litPart$ = q = new ChildPart(U.insertBefore(createMarker(), h), h, void 0, W ?? {});
	}
	return q._$setValue(h), debugLogEvent && debugLogEvent({
		kind: "end render",
		id: G,
		value: h,
		container: U,
		options: W,
		part: q
	}), q;
};
render.setSanitizer = setSanitizer, render.createSanitizer = createSanitizer, render._testOnlyClearSanitizerFactoryDoNotCallOrElse = _testOnlyClearSanitizerFactoryDoNotCallOrElse;
/**
* @license
* Copyright 2017 Google LLC
* SPDX-License-Identifier: BSD-3-Clause
*/
var JSCompiler_renameProperty = (h, U) => h, global = globalThis, issueWarning;
global.litIssuedWarnings ??= /* @__PURE__ */ new Set(), issueWarning = (h, U) => {
	U += ` See https://lit.dev/msg/${h} for more information.`, !global.litIssuedWarnings.has(U) && !global.litIssuedWarnings.has(h) && (console.warn(U), global.litIssuedWarnings.add(U));
};
var LitElement = class extends ReactiveElement {
	constructor() {
		super(...arguments), this.renderOptions = { host: this }, this.__childPart = void 0;
	}
	createRenderRoot() {
		let h = super.createRenderRoot();
		return this.renderOptions.renderBefore ??= h.firstChild, h;
	}
	update(h) {
		let U = this.render();
		this.hasUpdated || (this.renderOptions.isConnected = this.isConnected), super.update(h), this.__childPart = render(U, this.renderRoot, this.renderOptions);
	}
	connectedCallback() {
		super.connectedCallback(), this.__childPart?.setConnected(!0);
	}
	disconnectedCallback() {
		super.disconnectedCallback(), this.__childPart?.setConnected(!1);
	}
	render() {
		return noChange;
	}
};
LitElement._$litElement$ = !0, LitElement[JSCompiler_renameProperty("finalized", LitElement)] = !0, global.litElementHydrateSupport?.({ LitElement });
var polyfillSupport = global.litElementPolyfillSupportDevMode;
polyfillSupport?.({ LitElement });
const _$LE = {
	_$attributeToProperty: (h, U, W) => {
		h._$attributeToProperty(U, W);
	},
	_$changedProperties: (h) => h._$changedProperties
};
(global.litElementVersions ??= []).push("4.2.2"), global.litElementVersions.length > 1 && queueMicrotask(() => {
	issueWarning("multiple-versions", "Multiple versions of Lit loaded. Loading multiple versions is not recommended.");
});
const isServer = !1;
export { CSSResult, LitElement, ReactiveElement, _$LE, _$LH, adoptStyles, css, defaultConverter, getCompatibleStyle, html, isServer, mathml, noChange, notEqual, nothing, render, supportsAdoptingStyleSheets, svg, unsafeCSS };
