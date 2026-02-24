class bt {
  /** Plugin IDs that must complete onRegister() before this plugin starts. */
  dependencies = [];
  #t;
  /** Event bus — injected by SDK via _init(). Available inside onRegister(). */
  get bus() {
    if (!this.#t)
      throw new Error(`Plugin '${this.id}' accessed bus before _init() was called`);
    return this.#t;
  }
  /** @internal SDK use only. */
  async _init(t) {
    this.#t = t, await this.onRegister?.(), t.emit("register-finished", { pluginId: this.id });
  }
  /** @internal SDK use only. */
  async _destroy() {
    await this.onUnregister?.();
  }
}
const x = "tasks", _t = (i, t) => i.request(x, "list", t ?? {}), yt = (i, t) => i.request(x, "create", t), vt = (i, t) => i.request(x, "get", { task_id: t }), kt = (i, t) => i.request(x, "update", t), xt = (i, t) => i.request(x, "delete", { task_id: t }), wt = (i, t, e) => i.request(x, "search", { query: t, limit: e }), X = (i) => i.request(x, "stats", {});
function z() {
  return [...window.sdk.getConnections().values()].filter((i) => i.services.includes("tasks"));
}
function R(i) {
  const t = window.sdk.getConnections().get(i);
  if (!t) throw new Error(`Connection '${i}' not found`);
  return t;
}
function H() {
  return {
    total_tasks: 0,
    todo_count: 0,
    in_progress_count: 0,
    done_count: 0,
    blocked_count: 0,
    cancelled_count: 0,
    total_dependencies: 0,
    has_cycles: !1
  };
}
function Y(i, t) {
  return {
    total_tasks: i.total_tasks + t.total_tasks,
    todo_count: i.todo_count + t.todo_count,
    in_progress_count: i.in_progress_count + t.in_progress_count,
    done_count: i.done_count + t.done_count,
    blocked_count: i.blocked_count + t.blocked_count,
    cancelled_count: i.cancelled_count + t.cancelled_count,
    total_dependencies: i.total_dependencies + t.total_dependencies,
    has_cycles: i.has_cycles || t.has_cycles
  };
}
class ae extends bt {
  constructor() {
    super(...arguments), this.id = "adi.tasks", this.version = "0.1.0";
  }
  async onRegister() {
    const { AdiTasksElement: t } = await Promise.resolve().then(() => ne);
    customElements.get("adi-tasks") || customElements.define("adi-tasks", t), this.bus.emit("route:register", { path: "/tasks", element: "adi-tasks" }), this.bus.send("nav:add", { id: "tasks", label: "Tasks", path: "/tasks" }).handle(() => {
    }), this.bus.emit("command:register", { id: "tasks:open", label: "Go to Tasks page" }), this.bus.on("command:execute", ({ id: e }) => {
      e === "tasks:open" && this.bus.emit("router:navigate", { path: "/tasks" });
    }), this.bus.on("tasks:list", async (e) => {
      const { _cid: s, status: r } = e;
      try {
        const o = z(), [n, l] = await Promise.all([
          Promise.allSettled(o.map((h) => _t(h, { status: r }))),
          Promise.allSettled(o.map((h) => X(h)))
        ]), a = n.flatMap(
          (h, d) => h.status === "fulfilled" ? h.value.map((g) => ({ ...g, cocoonId: o[d].id })) : []
        ), c = l.reduce(
          (h, d) => d.status === "fulfilled" ? Y(h, d.value) : h,
          H()
        );
        this.bus.emit("tasks:list:ok", { tasks: a, stats: c, _cid: s });
      } catch (o) {
        console.error("[TasksPlugin] tasks:list error:", o), this.bus.emit("tasks:list:ok", { tasks: [], stats: H(), _cid: s });
      }
    }), this.bus.on("tasks:search", async (e) => {
      const { _cid: s, query: r, limit: o } = e;
      try {
        const n = z(), a = (await Promise.allSettled(n.map((c) => wt(c, r, o)))).flatMap(
          (c, h) => c.status === "fulfilled" ? c.value.map((d) => ({ ...d, cocoonId: n[h].id })) : []
        );
        this.bus.emit("tasks:search:ok", { tasks: a, _cid: s });
      } catch (n) {
        console.error("[TasksPlugin] tasks:search error:", n), this.bus.emit("tasks:search:ok", { tasks: [], _cid: s });
      }
    }), this.bus.on("tasks:stats", async (e) => {
      const { _cid: s } = e;
      try {
        const r = z(), n = (await Promise.allSettled(r.map((l) => X(l)))).reduce(
          (l, a) => a.status === "fulfilled" ? Y(l, a.value) : l,
          H()
        );
        this.bus.emit("tasks:stats:ok", { stats: n, _cid: s });
      } catch (r) {
        console.error("[TasksPlugin] tasks:stats error:", r), this.bus.emit("tasks:stats:ok", { stats: H(), _cid: s });
      }
    }), this.bus.on("tasks:get", async (e) => {
      const { _cid: s, task_id: r, cocoonId: o } = e;
      try {
        const n = await vt(R(o), r), l = {
          ...n,
          task: { ...n.task, cocoonId: o },
          depends_on: n.depends_on.map((a) => ({ ...a, cocoonId: o })),
          dependents: n.dependents.map((a) => ({ ...a, cocoonId: o }))
        };
        this.bus.emit("tasks:get:ok", { task: l, _cid: s });
      } catch (n) {
        console.error("[TasksPlugin] tasks:get error:", n);
      }
    }), this.bus.on("tasks:create", async (e) => {
      const { _cid: s, cocoonId: r, title: o, description: n, depends_on: l } = e;
      try {
        const a = await yt(R(r), { title: o, description: n, depends_on: l });
        this.bus.emit("tasks:create:ok", { task: { ...a, cocoonId: r }, _cid: s });
      } catch (a) {
        console.error("[TasksPlugin] tasks:create error:", a);
      }
    }), this.bus.on("tasks:update", async (e) => {
      const { _cid: s, cocoonId: r, task_id: o, title: n, description: l, status: a } = e;
      try {
        const c = await kt(R(r), { task_id: o, title: n, description: l, status: a });
        this.bus.emit("tasks:update:ok", { task: { ...c, cocoonId: r }, _cid: s });
      } catch (c) {
        console.error("[TasksPlugin] tasks:update error:", c);
      }
    }), this.bus.on("tasks:delete", async (e) => {
      const { _cid: s, cocoonId: r, task_id: o } = e;
      try {
        await xt(R(r), o), this.bus.emit("tasks:delete:ok", { _cid: s });
      } catch (n) {
        console.error("[TasksPlugin] tasks:delete error:", n);
      }
    });
  }
}
/**
 * @license
 * Copyright 2019 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const I = globalThis, W = I.ShadowRoot && (I.ShadyCSS === void 0 || I.ShadyCSS.nativeShadow) && "adoptedStyleSheets" in Document.prototype && "replace" in CSSStyleSheet.prototype, pt = Symbol(), tt = /* @__PURE__ */ new WeakMap();
let At = class {
  constructor(t, e, s) {
    if (this._$cssResult$ = !0, s !== pt) throw Error("CSSResult is not constructable. Use `unsafeCSS` or `css` instead.");
    this.cssText = t, this.t = e;
  }
  get styleSheet() {
    let t = this.o;
    const e = this.t;
    if (W && t === void 0) {
      const s = e !== void 0 && e.length === 1;
      s && (t = tt.get(e)), t === void 0 && ((this.o = t = new CSSStyleSheet()).replaceSync(this.cssText), s && tt.set(e, t));
    }
    return t;
  }
  toString() {
    return this.cssText;
  }
};
const St = (i) => new At(typeof i == "string" ? i : i + "", void 0, pt), Et = (i, t) => {
  if (W) i.adoptedStyleSheets = t.map((e) => e instanceof CSSStyleSheet ? e : e.styleSheet);
  else for (const e of t) {
    const s = document.createElement("style"), r = I.litNonce;
    r !== void 0 && s.setAttribute("nonce", r), s.textContent = e.cssText, i.appendChild(s);
  }
}, et = W ? (i) => i : (i) => i instanceof CSSStyleSheet ? ((t) => {
  let e = "";
  for (const s of t.cssRules) e += s.cssText;
  return St(e);
})(i) : i;
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const { is: Tt, defineProperty: Ct, getOwnPropertyDescriptor: Pt, getOwnPropertyNames: Dt, getOwnPropertySymbols: Ot, getPrototypeOf: Ut } = Object, j = globalThis, st = j.trustedTypes, Mt = st ? st.emptyScript : "", Nt = j.reactiveElementPolyfillSupport, C = (i, t) => i, B = { toAttribute(i, t) {
  switch (t) {
    case Boolean:
      i = i ? Mt : null;
      break;
    case Object:
    case Array:
      i = i == null ? i : JSON.stringify(i);
  }
  return i;
}, fromAttribute(i, t) {
  let e = i;
  switch (t) {
    case Boolean:
      e = i !== null;
      break;
    case Number:
      e = i === null ? null : Number(i);
      break;
    case Object:
    case Array:
      try {
        e = JSON.parse(i);
      } catch {
        e = null;
      }
  }
  return e;
} }, Q = (i, t) => !Tt(i, t), it = { attribute: !0, type: String, converter: B, reflect: !1, useDefault: !1, hasChanged: Q };
Symbol.metadata ??= Symbol("metadata"), j.litPropertyMetadata ??= /* @__PURE__ */ new WeakMap();
let A = class extends HTMLElement {
  static addInitializer(t) {
    this._$Ei(), (this.l ??= []).push(t);
  }
  static get observedAttributes() {
    return this.finalize(), this._$Eh && [...this._$Eh.keys()];
  }
  static createProperty(t, e = it) {
    if (e.state && (e.attribute = !1), this._$Ei(), this.prototype.hasOwnProperty(t) && ((e = Object.create(e)).wrapped = !0), this.elementProperties.set(t, e), !e.noAccessor) {
      const s = Symbol(), r = this.getPropertyDescriptor(t, s, e);
      r !== void 0 && Ct(this.prototype, t, r);
    }
  }
  static getPropertyDescriptor(t, e, s) {
    const { get: r, set: o } = Pt(this.prototype, t) ?? { get() {
      return this[e];
    }, set(n) {
      this[e] = n;
    } };
    return { get: r, set(n) {
      const l = r?.call(this);
      o?.call(this, n), this.requestUpdate(t, l, s);
    }, configurable: !0, enumerable: !0 };
  }
  static getPropertyOptions(t) {
    return this.elementProperties.get(t) ?? it;
  }
  static _$Ei() {
    if (this.hasOwnProperty(C("elementProperties"))) return;
    const t = Ut(this);
    t.finalize(), t.l !== void 0 && (this.l = [...t.l]), this.elementProperties = new Map(t.elementProperties);
  }
  static finalize() {
    if (this.hasOwnProperty(C("finalized"))) return;
    if (this.finalized = !0, this._$Ei(), this.hasOwnProperty(C("properties"))) {
      const e = this.properties, s = [...Dt(e), ...Ot(e)];
      for (const r of s) this.createProperty(r, e[r]);
    }
    const t = this[Symbol.metadata];
    if (t !== null) {
      const e = litPropertyMetadata.get(t);
      if (e !== void 0) for (const [s, r] of e) this.elementProperties.set(s, r);
    }
    this._$Eh = /* @__PURE__ */ new Map();
    for (const [e, s] of this.elementProperties) {
      const r = this._$Eu(e, s);
      r !== void 0 && this._$Eh.set(r, e);
    }
    this.elementStyles = this.finalizeStyles(this.styles);
  }
  static finalizeStyles(t) {
    const e = [];
    if (Array.isArray(t)) {
      const s = new Set(t.flat(1 / 0).reverse());
      for (const r of s) e.unshift(et(r));
    } else t !== void 0 && e.push(et(t));
    return e;
  }
  static _$Eu(t, e) {
    const s = e.attribute;
    return s === !1 ? void 0 : typeof s == "string" ? s : typeof t == "string" ? t.toLowerCase() : void 0;
  }
  constructor() {
    super(), this._$Ep = void 0, this.isUpdatePending = !1, this.hasUpdated = !1, this._$Em = null, this._$Ev();
  }
  _$Ev() {
    this._$ES = new Promise((t) => this.enableUpdating = t), this._$AL = /* @__PURE__ */ new Map(), this._$E_(), this.requestUpdate(), this.constructor.l?.forEach((t) => t(this));
  }
  addController(t) {
    (this._$EO ??= /* @__PURE__ */ new Set()).add(t), this.renderRoot !== void 0 && this.isConnected && t.hostConnected?.();
  }
  removeController(t) {
    this._$EO?.delete(t);
  }
  _$E_() {
    const t = /* @__PURE__ */ new Map(), e = this.constructor.elementProperties;
    for (const s of e.keys()) this.hasOwnProperty(s) && (t.set(s, this[s]), delete this[s]);
    t.size > 0 && (this._$Ep = t);
  }
  createRenderRoot() {
    const t = this.shadowRoot ?? this.attachShadow(this.constructor.shadowRootOptions);
    return Et(t, this.constructor.elementStyles), t;
  }
  connectedCallback() {
    this.renderRoot ??= this.createRenderRoot(), this.enableUpdating(!0), this._$EO?.forEach((t) => t.hostConnected?.());
  }
  enableUpdating(t) {
  }
  disconnectedCallback() {
    this._$EO?.forEach((t) => t.hostDisconnected?.());
  }
  attributeChangedCallback(t, e, s) {
    this._$AK(t, s);
  }
  _$ET(t, e) {
    const s = this.constructor.elementProperties.get(t), r = this.constructor._$Eu(t, s);
    if (r !== void 0 && s.reflect === !0) {
      const o = (s.converter?.toAttribute !== void 0 ? s.converter : B).toAttribute(e, s.type);
      this._$Em = t, o == null ? this.removeAttribute(r) : this.setAttribute(r, o), this._$Em = null;
    }
  }
  _$AK(t, e) {
    const s = this.constructor, r = s._$Eh.get(t);
    if (r !== void 0 && this._$Em !== r) {
      const o = s.getPropertyOptions(r), n = typeof o.converter == "function" ? { fromAttribute: o.converter } : o.converter?.fromAttribute !== void 0 ? o.converter : B;
      this._$Em = r;
      const l = n.fromAttribute(e, o.type);
      this[r] = l ?? this._$Ej?.get(r) ?? l, this._$Em = null;
    }
  }
  requestUpdate(t, e, s, r = !1, o) {
    if (t !== void 0) {
      const n = this.constructor;
      if (r === !1 && (o = this[t]), s ??= n.getPropertyOptions(t), !((s.hasChanged ?? Q)(o, e) || s.useDefault && s.reflect && o === this._$Ej?.get(t) && !this.hasAttribute(n._$Eu(t, s)))) return;
      this.C(t, e, s);
    }
    this.isUpdatePending === !1 && (this._$ES = this._$EP());
  }
  C(t, e, { useDefault: s, reflect: r, wrapped: o }, n) {
    s && !(this._$Ej ??= /* @__PURE__ */ new Map()).has(t) && (this._$Ej.set(t, n ?? e ?? this[t]), o !== !0 || n !== void 0) || (this._$AL.has(t) || (this.hasUpdated || s || (e = void 0), this._$AL.set(t, e)), r === !0 && this._$Em !== t && (this._$Eq ??= /* @__PURE__ */ new Set()).add(t));
  }
  async _$EP() {
    this.isUpdatePending = !0;
    try {
      await this._$ES;
    } catch (e) {
      Promise.reject(e);
    }
    const t = this.scheduleUpdate();
    return t != null && await t, !this.isUpdatePending;
  }
  scheduleUpdate() {
    return this.performUpdate();
  }
  performUpdate() {
    if (!this.isUpdatePending) return;
    if (!this.hasUpdated) {
      if (this.renderRoot ??= this.createRenderRoot(), this._$Ep) {
        for (const [r, o] of this._$Ep) this[r] = o;
        this._$Ep = void 0;
      }
      const s = this.constructor.elementProperties;
      if (s.size > 0) for (const [r, o] of s) {
        const { wrapped: n } = o, l = this[r];
        n !== !0 || this._$AL.has(r) || l === void 0 || this.C(r, void 0, o, l);
      }
    }
    let t = !1;
    const e = this._$AL;
    try {
      t = this.shouldUpdate(e), t ? (this.willUpdate(e), this._$EO?.forEach((s) => s.hostUpdate?.()), this.update(e)) : this._$EM();
    } catch (s) {
      throw t = !1, this._$EM(), s;
    }
    t && this._$AE(e);
  }
  willUpdate(t) {
  }
  _$AE(t) {
    this._$EO?.forEach((e) => e.hostUpdated?.()), this.hasUpdated || (this.hasUpdated = !0, this.firstUpdated(t)), this.updated(t);
  }
  _$EM() {
    this._$AL = /* @__PURE__ */ new Map(), this.isUpdatePending = !1;
  }
  get updateComplete() {
    return this.getUpdateComplete();
  }
  getUpdateComplete() {
    return this._$ES;
  }
  shouldUpdate(t) {
    return !0;
  }
  update(t) {
    this._$Eq &&= this._$Eq.forEach((e) => this._$ET(e, this[e])), this._$EM();
  }
  updated(t) {
  }
  firstUpdated(t) {
  }
};
A.elementStyles = [], A.shadowRootOptions = { mode: "open" }, A[C("elementProperties")] = /* @__PURE__ */ new Map(), A[C("finalized")] = /* @__PURE__ */ new Map(), Nt?.({ ReactiveElement: A }), (j.reactiveElementVersions ??= []).push("2.1.2");
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const V = globalThis, rt = (i) => i, L = V.trustedTypes, ot = L ? L.createPolicy("lit-html", { createHTML: (i) => i }) : void 0, gt = "$lit$", _ = `lit$${Math.random().toFixed(9).slice(2)}$`, ft = "?" + _, Rt = `<${ft}>`, k = document, D = () => k.createComment(""), O = (i) => i === null || typeof i != "object" && typeof i != "function", Z = Array.isArray, Ht = (i) => Z(i) || typeof i?.[Symbol.iterator] == "function", F = `[ 	
\f\r]`, T = /<(?:(!--|\/[^a-zA-Z])|(\/?[a-zA-Z][^>\s]*)|(\/?$))/g, nt = /-->/g, at = />/g, y = RegExp(`>|${F}(?:([^\\s"'>=/]+)(${F}*=${F}*(?:[^ 	
\f\r"'\`<>=]|("|')|))|$)`, "g"), lt = /'/g, ct = /"/g, $t = /^(?:script|style|textarea|title)$/i, It = (i) => (t, ...e) => ({ _$litType$: i, strings: t, values: e }), p = It(1), S = Symbol.for("lit-noChange"), u = Symbol.for("lit-nothing"), dt = /* @__PURE__ */ new WeakMap(), v = k.createTreeWalker(k, 129);
function mt(i, t) {
  if (!Z(i) || !i.hasOwnProperty("raw")) throw Error("invalid template strings array");
  return ot !== void 0 ? ot.createHTML(t) : t;
}
const Bt = (i, t) => {
  const e = i.length - 1, s = [];
  let r, o = t === 2 ? "<svg>" : t === 3 ? "<math>" : "", n = T;
  for (let l = 0; l < e; l++) {
    const a = i[l];
    let c, h, d = -1, g = 0;
    for (; g < a.length && (n.lastIndex = g, h = n.exec(a), h !== null); ) g = n.lastIndex, n === T ? h[1] === "!--" ? n = nt : h[1] !== void 0 ? n = at : h[2] !== void 0 ? ($t.test(h[2]) && (r = RegExp("</" + h[2], "g")), n = y) : h[3] !== void 0 && (n = y) : n === y ? h[0] === ">" ? (n = r ?? T, d = -1) : h[1] === void 0 ? d = -2 : (d = n.lastIndex - h[2].length, c = h[1], n = h[3] === void 0 ? y : h[3] === '"' ? ct : lt) : n === ct || n === lt ? n = y : n === nt || n === at ? n = T : (n = y, r = void 0);
    const b = n === y && i[l + 1].startsWith("/>") ? " " : "";
    o += n === T ? a + Rt : d >= 0 ? (s.push(c), a.slice(0, d) + gt + a.slice(d) + _ + b) : a + _ + (d === -2 ? l : b);
  }
  return [mt(i, o + (i[e] || "<?>") + (t === 2 ? "</svg>" : t === 3 ? "</math>" : "")), s];
};
class U {
  constructor({ strings: t, _$litType$: e }, s) {
    let r;
    this.parts = [];
    let o = 0, n = 0;
    const l = t.length - 1, a = this.parts, [c, h] = Bt(t, e);
    if (this.el = U.createElement(c, s), v.currentNode = this.el.content, e === 2 || e === 3) {
      const d = this.el.content.firstChild;
      d.replaceWith(...d.childNodes);
    }
    for (; (r = v.nextNode()) !== null && a.length < l; ) {
      if (r.nodeType === 1) {
        if (r.hasAttributes()) for (const d of r.getAttributeNames()) if (d.endsWith(gt)) {
          const g = h[n++], b = r.getAttribute(d).split(_), N = /([.?@])?(.*)/.exec(g);
          a.push({ type: 1, index: o, name: N[2], strings: b, ctor: N[1] === "." ? jt : N[1] === "?" ? qt : N[1] === "@" ? zt : q }), r.removeAttribute(d);
        } else d.startsWith(_) && (a.push({ type: 6, index: o }), r.removeAttribute(d));
        if ($t.test(r.tagName)) {
          const d = r.textContent.split(_), g = d.length - 1;
          if (g > 0) {
            r.textContent = L ? L.emptyScript : "";
            for (let b = 0; b < g; b++) r.append(d[b], D()), v.nextNode(), a.push({ type: 2, index: ++o });
            r.append(d[g], D());
          }
        }
      } else if (r.nodeType === 8) if (r.data === ft) a.push({ type: 2, index: o });
      else {
        let d = -1;
        for (; (d = r.data.indexOf(_, d + 1)) !== -1; ) a.push({ type: 7, index: o }), d += _.length - 1;
      }
      o++;
    }
  }
  static createElement(t, e) {
    const s = k.createElement("template");
    return s.innerHTML = t, s;
  }
}
function E(i, t, e = i, s) {
  if (t === S) return t;
  let r = s !== void 0 ? e._$Co?.[s] : e._$Cl;
  const o = O(t) ? void 0 : t._$litDirective$;
  return r?.constructor !== o && (r?._$AO?.(!1), o === void 0 ? r = void 0 : (r = new o(i), r._$AT(i, e, s)), s !== void 0 ? (e._$Co ??= [])[s] = r : e._$Cl = r), r !== void 0 && (t = E(i, r._$AS(i, t.values), r, s)), t;
}
class Lt {
  constructor(t, e) {
    this._$AV = [], this._$AN = void 0, this._$AD = t, this._$AM = e;
  }
  get parentNode() {
    return this._$AM.parentNode;
  }
  get _$AU() {
    return this._$AM._$AU;
  }
  u(t) {
    const { el: { content: e }, parts: s } = this._$AD, r = (t?.creationScope ?? k).importNode(e, !0);
    v.currentNode = r;
    let o = v.nextNode(), n = 0, l = 0, a = s[0];
    for (; a !== void 0; ) {
      if (n === a.index) {
        let c;
        a.type === 2 ? c = new M(o, o.nextSibling, this, t) : a.type === 1 ? c = new a.ctor(o, a.name, a.strings, this, t) : a.type === 6 && (c = new Ft(o, this, t)), this._$AV.push(c), a = s[++l];
      }
      n !== a?.index && (o = v.nextNode(), n++);
    }
    return v.currentNode = k, r;
  }
  p(t) {
    let e = 0;
    for (const s of this._$AV) s !== void 0 && (s.strings !== void 0 ? (s._$AI(t, s, e), e += s.strings.length - 2) : s._$AI(t[e])), e++;
  }
}
class M {
  get _$AU() {
    return this._$AM?._$AU ?? this._$Cv;
  }
  constructor(t, e, s, r) {
    this.type = 2, this._$AH = u, this._$AN = void 0, this._$AA = t, this._$AB = e, this._$AM = s, this.options = r, this._$Cv = r?.isConnected ?? !0;
  }
  get parentNode() {
    let t = this._$AA.parentNode;
    const e = this._$AM;
    return e !== void 0 && t?.nodeType === 11 && (t = e.parentNode), t;
  }
  get startNode() {
    return this._$AA;
  }
  get endNode() {
    return this._$AB;
  }
  _$AI(t, e = this) {
    t = E(this, t, e), O(t) ? t === u || t == null || t === "" ? (this._$AH !== u && this._$AR(), this._$AH = u) : t !== this._$AH && t !== S && this._(t) : t._$litType$ !== void 0 ? this.$(t) : t.nodeType !== void 0 ? this.T(t) : Ht(t) ? this.k(t) : this._(t);
  }
  O(t) {
    return this._$AA.parentNode.insertBefore(t, this._$AB);
  }
  T(t) {
    this._$AH !== t && (this._$AR(), this._$AH = this.O(t));
  }
  _(t) {
    this._$AH !== u && O(this._$AH) ? this._$AA.nextSibling.data = t : this.T(k.createTextNode(t)), this._$AH = t;
  }
  $(t) {
    const { values: e, _$litType$: s } = t, r = typeof s == "number" ? this._$AC(t) : (s.el === void 0 && (s.el = U.createElement(mt(s.h, s.h[0]), this.options)), s);
    if (this._$AH?._$AD === r) this._$AH.p(e);
    else {
      const o = new Lt(r, this), n = o.u(this.options);
      o.p(e), this.T(n), this._$AH = o;
    }
  }
  _$AC(t) {
    let e = dt.get(t.strings);
    return e === void 0 && dt.set(t.strings, e = new U(t)), e;
  }
  k(t) {
    Z(this._$AH) || (this._$AH = [], this._$AR());
    const e = this._$AH;
    let s, r = 0;
    for (const o of t) r === e.length ? e.push(s = new M(this.O(D()), this.O(D()), this, this.options)) : s = e[r], s._$AI(o), r++;
    r < e.length && (this._$AR(s && s._$AB.nextSibling, r), e.length = r);
  }
  _$AR(t = this._$AA.nextSibling, e) {
    for (this._$AP?.(!1, !0, e); t !== this._$AB; ) {
      const s = rt(t).nextSibling;
      rt(t).remove(), t = s;
    }
  }
  setConnected(t) {
    this._$AM === void 0 && (this._$Cv = t, this._$AP?.(t));
  }
}
class q {
  get tagName() {
    return this.element.tagName;
  }
  get _$AU() {
    return this._$AM._$AU;
  }
  constructor(t, e, s, r, o) {
    this.type = 1, this._$AH = u, this._$AN = void 0, this.element = t, this.name = e, this._$AM = r, this.options = o, s.length > 2 || s[0] !== "" || s[1] !== "" ? (this._$AH = Array(s.length - 1).fill(new String()), this.strings = s) : this._$AH = u;
  }
  _$AI(t, e = this, s, r) {
    const o = this.strings;
    let n = !1;
    if (o === void 0) t = E(this, t, e, 0), n = !O(t) || t !== this._$AH && t !== S, n && (this._$AH = t);
    else {
      const l = t;
      let a, c;
      for (t = o[0], a = 0; a < o.length - 1; a++) c = E(this, l[s + a], e, a), c === S && (c = this._$AH[a]), n ||= !O(c) || c !== this._$AH[a], c === u ? t = u : t !== u && (t += (c ?? "") + o[a + 1]), this._$AH[a] = c;
    }
    n && !r && this.j(t);
  }
  j(t) {
    t === u ? this.element.removeAttribute(this.name) : this.element.setAttribute(this.name, t ?? "");
  }
}
class jt extends q {
  constructor() {
    super(...arguments), this.type = 3;
  }
  j(t) {
    this.element[this.name] = t === u ? void 0 : t;
  }
}
class qt extends q {
  constructor() {
    super(...arguments), this.type = 4;
  }
  j(t) {
    this.element.toggleAttribute(this.name, !!t && t !== u);
  }
}
class zt extends q {
  constructor(t, e, s, r, o) {
    super(t, e, s, r, o), this.type = 5;
  }
  _$AI(t, e = this) {
    if ((t = E(this, t, e, 0) ?? u) === S) return;
    const s = this._$AH, r = t === u && s !== u || t.capture !== s.capture || t.once !== s.once || t.passive !== s.passive, o = t !== u && (s === u || r);
    r && this.element.removeEventListener(this.name, this, s), o && this.element.addEventListener(this.name, this, t), this._$AH = t;
  }
  handleEvent(t) {
    typeof this._$AH == "function" ? this._$AH.call(this.options?.host ?? this.element, t) : this._$AH.handleEvent(t);
  }
}
class Ft {
  constructor(t, e, s) {
    this.element = t, this.type = 6, this._$AN = void 0, this._$AM = e, this.options = s;
  }
  get _$AU() {
    return this._$AM._$AU;
  }
  _$AI(t) {
    E(this, t);
  }
}
const Wt = V.litHtmlPolyfillSupport;
Wt?.(U, M), (V.litHtmlVersions ??= []).push("3.3.2");
const Qt = (i, t, e) => {
  const s = e?.renderBefore ?? t;
  let r = s._$litPart$;
  if (r === void 0) {
    const o = e?.renderBefore ?? null;
    s._$litPart$ = r = new M(t.insertBefore(D(), o), o, void 0, e ?? {});
  }
  return r._$AI(i), r;
};
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const J = globalThis;
class P extends A {
  constructor() {
    super(...arguments), this.renderOptions = { host: this }, this._$Do = void 0;
  }
  createRenderRoot() {
    const t = super.createRenderRoot();
    return this.renderOptions.renderBefore ??= t.firstChild, t;
  }
  update(t) {
    const e = this.render();
    this.hasUpdated || (this.renderOptions.isConnected = this.isConnected), super.update(t), this._$Do = Qt(e, this.renderRoot, this.renderOptions);
  }
  connectedCallback() {
    super.connectedCallback(), this._$Do?.setConnected(!0);
  }
  disconnectedCallback() {
    super.disconnectedCallback(), this._$Do?.setConnected(!1);
  }
  render() {
    return S;
  }
}
P._$litElement$ = !0, P.finalized = !0, J.litElementHydrateSupport?.({ LitElement: P });
const Vt = J.litElementPolyfillSupport;
Vt?.({ LitElement: P });
(J.litElementVersions ??= []).push("4.2.2");
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const Zt = { attribute: !0, type: String, converter: B, reflect: !1, hasChanged: Q }, Jt = (i = Zt, t, e) => {
  const { kind: s, metadata: r } = e;
  let o = globalThis.litPropertyMetadata.get(r);
  if (o === void 0 && globalThis.litPropertyMetadata.set(r, o = /* @__PURE__ */ new Map()), s === "setter" && ((i = Object.create(i)).wrapped = !0), o.set(e.name, i), s === "accessor") {
    const { name: n } = e;
    return { set(l) {
      const a = t.get.call(this);
      t.set.call(this, l), this.requestUpdate(n, a, i, !0, l);
    }, init(l) {
      return l !== void 0 && this.C(n, void 0, i, l), l;
    } };
  }
  if (s === "setter") {
    const { name: n } = e;
    return function(l) {
      const a = this[n];
      t.call(this, l), this.requestUpdate(n, a, i, !0, l);
    };
  }
  throw Error("Unsupported decorator location: " + s);
};
function Kt(i) {
  return (t, e) => typeof e == "object" ? Jt(i, t, e) : ((s, r, o) => {
    const n = r.hasOwnProperty(o);
    return r.constructor.createProperty(o, s), n ? Object.getOwnPropertyDescriptor(r, o) : void 0;
  })(i, t, e);
}
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
function $(i) {
  return Kt({ ...i, state: !0, attribute: !1 });
}
const K = {
  todo: "bg-gray-500/20 text-gray-300",
  in_progress: "bg-blue-500/20 text-blue-300",
  done: "bg-green-500/20 text-green-300",
  blocked: "bg-red-500/20 text-red-300",
  cancelled: "bg-gray-600/20 text-gray-400"
}, G = {
  todo: "Todo",
  in_progress: "In Progress",
  done: "Done",
  blocked: "Blocked",
  cancelled: "Cancelled"
};
function Gt(i) {
  const t = Math.floor((Date.now() - i * 1e3) / 1e3);
  if (t < 60) return `${t}s ago`;
  const e = Math.floor(t / 60);
  if (e < 60) return `${e}m ago`;
  const s = Math.floor(e / 60);
  return s < 24 ? `${s}h ago` : `${Math.floor(s / 24)}d ago`;
}
const w = (i, t, e) => p`
  <div class="flex flex-col items-center px-3 py-1.5 rounded-lg ${e}">
    <span class="text-lg font-semibold">${t}</span>
    <span class="text-xs opacity-70">${i}</span>
  </div>
`, Xt = (i) => i ? p`
    <div class="flex gap-2 flex-wrap mb-4">
      ${w("Total", i.total_tasks, "bg-white/5 text-gray-300")}
      ${w("Todo", i.todo_count, "bg-gray-500/10 text-gray-300")}
      ${w("In Progress", i.in_progress_count, "bg-blue-500/10 text-blue-300")}
      ${w("Done", i.done_count, "bg-green-500/10 text-green-300")}
      ${w("Blocked", i.blocked_count, "bg-red-500/10 text-red-300")}
      ${w("Cancelled", i.cancelled_count, "bg-gray-600/10 text-gray-400")}
    </div>
  ` : u, Yt = (i, t) => p`
    <div class="flex gap-1 mb-4 flex-wrap">
      ${[
  { label: "All", value: void 0 },
  { label: "Todo", value: "todo" },
  { label: "In Progress", value: "in_progress" },
  { label: "Done", value: "done" },
  { label: "Blocked", value: "blocked" },
  { label: "Cancelled", value: "cancelled" }
].map(
  (s) => p`
          <button
            class="px-3 py-1 rounded-full text-sm transition-colors ${i === s.value ? "bg-purple-500/30 text-purple-200 font-medium" : "bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-200"}"
            @click=${() => t(s.value)}
          >
            ${s.label}
          </button>
        `
)}
    </div>
  `, te = (i, t) => p`
  <button
    class="w-full text-left p-3 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-start gap-3 group"
    @click=${() => t(i)}
  >
    <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium shrink-0 mt-0.5 ${K[i.status]}">
      ${G[i.status]}
    </span>
    <div class="flex-1 min-w-0">
      <div class="text-sm text-gray-200 group-hover:text-white truncate">${i.title}</div>
      ${i.description ? p`<div class="text-xs text-gray-500 mt-0.5 truncate">${i.description}</div>` : u}
    </div>
    <span class="text-xs text-gray-600 shrink-0 mt-0.5">${Gt(i.updated_at)}</span>
  </button>
`;
function ee(i) {
  return p`
    <div class="space-y-0_75">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-lg font-semibold text-gray-200">Tasks</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium"
          @click=${i.onNewTask}
        >
          + New Task
        </button>
      </div>

      ${Xt(i.stats)}
      ${Yt(i.filter, i.onFilterChange)}

      <div class="relative mb-3">
        <input
          type="text"
          placeholder="Search tasks..."
          .value=${i.searchQuery}
          @input=${(t) => i.onSearch(t.target.value)}
          class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50"
        />
      </div>

      ${i.error ? p`<div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-300">${i.error}</div>` : i.loading ? p`<div class="text-center py-8 text-gray-500 text-sm">Loading...</div>` : i.tasks.length === 0 ? p`<div class="text-center py-8 text-gray-500 text-sm">No tasks found</div>` : p`
                <div class="space-y-0_5">
                  ${i.tasks.map((t) => te(t, i.onSelectTask))}
                </div>
              `}
    </div>
  `;
}
const se = ["todo", "in_progress", "done", "blocked", "cancelled"], ht = (i) => new Date(i * 1e3).toLocaleString(), ut = (i, t, e) => t.length === 0 ? u : p`
    <div class="mt-4">
      <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-2">${i}</h4>
      <div class="space-y-0_25">
        ${t.map(
  (s) => p`
            <button
              class="w-full text-left px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-center gap-2 text-sm"
              @click=${() => e(s)}
            >
              <span class="inline-flex px-1.5 py-0.5 rounded text-xs ${K[s.status]}">
                ${G[s.status]}
              </span>
              <span class="text-gray-300 truncate">${s.title}</span>
            </button>
          `
)}
      </div>
    </div>
  `;
function ie(i) {
  const { task: t, submitting: e, confirmingDelete: s, onBack: r, onCancelDelete: o, onStatusChange: n, onDelete: l, onNavigate: a } = i, { task: c, depends_on: h, dependents: d } = t;
  return p`
    <div class="space-y-1">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${r}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4 space-y-4">
        <h2 class="text-lg font-semibold text-gray-100">${c.title}</h2>

        ${c.description ? p`<p class="text-sm text-gray-400 whitespace-pre-wrap">${c.description}</p>` : u}

        <div class="flex items-center gap-3 flex-wrap">
          <label class="text-xs text-gray-500 uppercase tracking-wider">Status</label>
          <div class="flex gap-1 flex-wrap">
            ${se.map(
    (g) => p`
                <button
                  class="px-2.5 py-1 rounded text-xs transition-colors ${c.status === g ? K[g] + " font-medium ring-1 ring-white/20" : "bg-white/5 text-gray-500 hover:bg-white/10 hover:text-gray-300"}"
                  ?disabled=${e}
                  @click=${() => {
      c.status !== g && n(g);
    }}
                >
                  ${G[g]}
                </button>
              `
  )}
          </div>
        </div>

        <div class="flex gap-4 text-xs text-gray-500">
          <span>Created: ${ht(c.created_at)}</span>
          <span>Updated: ${ht(c.updated_at)}</span>
        </div>

        ${ut("Depends on", h, a)}
        ${ut("Blocked by this", d, a)}

        <div class="pt-3 border-t border-white/10">
          ${s ? p`
                <div class="flex items-center gap-2">
                  <span class="text-sm text-red-400">Delete this task?</span>
                  <button
                    class="px-3 py-1 rounded text-sm bg-red-500/20 text-red-300 hover:bg-red-500/30 transition-colors"
                    ?disabled=${e}
                    @click=${l}
                  >
                    Confirm
                  </button>
                  <button
                    class="px-3 py-1 rounded text-sm bg-white/5 text-gray-400 hover:bg-white/10 transition-colors"
                    @click=${o}
                  >
                    Cancel
                  </button>
                </div>
              ` : p`
                <button
                  class="px-3 py-1 rounded text-sm bg-red-500/10 text-red-400 hover:bg-red-500/20 transition-colors"
                  ?disabled=${e}
                  @click=${l}
                >
                  Delete Task
                </button>
              `}
        </div>
      </div>
    </div>
  `;
}
function re(i) {
  const { connections: t, submitting: e, onBack: s, onCreate: r } = i;
  return p`
    <div class="space-y-0_75">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${s}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4">
        <h2 class="text-lg font-semibold text-gray-200 mb-4">New Task</h2>

        <form @submit=${(n) => {
    n.preventDefault();
    const l = n.target, a = new FormData(l), c = (a.get("title") ?? "").trim(), h = (a.get("description") ?? "").trim(), d = a.get("cocoonId");
    c && d && r({ title: c, description: h || void 0, cocoonId: d });
  }} class="space-y-1">
          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Connection</label>
            <select
              name="cocoonId"
              required
              ?disabled=${e}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              ${t.map((n) => p`<option value=${n.id}>${n.id}</option>`)}
            </select>
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Title</label>
            <input
              type="text"
              name="title"
              required
              ?disabled=${e}
              placeholder="What needs to be done?"
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            />
          </div>

          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Description</label>
            <textarea
              name="description"
              rows="3"
              ?disabled=${e}
              placeholder="Optional details..."
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50 resize-none disabled:opacity-50"
            ></textarea>
          </div>

          <div class="flex gap-2">
            <button
              type="submit"
              ?disabled=${e}
              class="px-4 py-2 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium disabled:opacity-50"
            >
              ${e ? "Creating..." : "Create Task"}
            </button>
            <button
              type="button"
              ?disabled=${e}
              @click=${s}
              class="px-4 py-2 rounded-lg bg-white/5 text-gray-400 hover:bg-white/10 transition-colors text-sm disabled:opacity-50"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>
    </div>
  `;
}
var oe = Object.defineProperty, m = (i, t, e, s) => {
  for (var r = void 0, o = i.length - 1, n; o >= 0; o--)
    (n = i[o]) && (r = n(t, e, r) || r);
  return r && oe(t, e, r), r;
};
class f extends P {
  constructor() {
    super(...arguments), this.tasks = [], this.stats = null, this.selectedTask = null, this.filter = void 0, this.searchQuery = "", this.view = "list", this.loading = !1, this.submitting = !1, this.confirmingDelete = !1, this.error = null;
  }
  createRenderRoot() {
    return this;
  }
  connectedCallback() {
    super.connectedCallback(), this.loadData();
  }
  get bus() {
    return window.sdk.bus;
  }
  async loadData() {
    this.loading = !0, this.error = null;
    try {
      if (this.searchQuery.trim()) {
        this.stats = null;
        const t = await this.bus.send("tasks:search", { query: this.searchQuery }).wait();
        this.tasks = t.tasks;
      } else {
        const t = await this.bus.send("tasks:list", { status: this.filter }).wait();
        this.tasks = t.tasks, this.stats = t.stats;
      }
    } catch (t) {
      this.error = t instanceof Error ? t.message : "Failed to load tasks";
    } finally {
      this.loading = !1;
    }
  }
  async loadDetail(t) {
    this.loading = !0;
    try {
      const e = await this.bus.send("tasks:get", { task_id: t.id, cocoonId: t.cocoonId }).wait();
      this.selectedTask = e.task, this.view = "detail";
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Failed to load task";
    } finally {
      this.loading = !1;
    }
  }
  async handleStatusChange(t, e) {
    try {
      const s = await this.bus.send("tasks:update", { task_id: t.id, cocoonId: t.cocoonId, status: e }).wait();
      this.tasks = this.tasks.map(
        (r) => r.id === t.id && r.cocoonId === t.cocoonId ? s.task : r
      ), this.selectedTask?.task.id === t.id && (this.selectedTask = { ...this.selectedTask, task: s.task });
    } catch (s) {
      this.error = s instanceof Error ? s.message : "Failed to update task";
    }
  }
  async handleDelete(t) {
    if (!this.confirmingDelete) {
      this.confirmingDelete = !0;
      return;
    }
    this.submitting = !0;
    try {
      await this.bus.send("tasks:delete", { task_id: t.id, cocoonId: t.cocoonId }).wait(), this.tasks = this.tasks.filter((e) => !(e.id === t.id && e.cocoonId === t.cocoonId)), this.view = "list", this.confirmingDelete = !1;
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Failed to delete task", this.confirmingDelete = !1;
    } finally {
      this.submitting = !1;
    }
  }
  async handleCreate(t) {
    this.submitting = !0;
    try {
      const e = await this.bus.send("tasks:create", t).wait();
      this.tasks = [...this.tasks, e.task], this.view = "list";
    } catch (e) {
      this.error = e instanceof Error ? e.message : "Failed to create task";
    } finally {
      this.submitting = !1;
    }
  }
  handleFilterChange(t) {
    this.filter = t, this.loadData();
  }
  handleSearch(t) {
    this.searchQuery = t, this.loadData();
  }
  render() {
    const t = [...window.sdk.getConnections().values()];
    return this.view === "detail" && this.selectedTask ? ie({
      task: this.selectedTask,
      submitting: this.submitting,
      confirmingDelete: this.confirmingDelete,
      onBack: () => {
        this.view = "list", this.selectedTask = null, this.confirmingDelete = !1;
      },
      onCancelDelete: () => {
        this.confirmingDelete = !1;
      },
      onStatusChange: (e) => this.handleStatusChange(this.selectedTask.task, e),
      onDelete: () => this.handleDelete(this.selectedTask.task),
      onNavigate: (e) => this.loadDetail(e)
    }) : this.view === "create" ? re({
      connections: t,
      submitting: this.submitting,
      onBack: () => {
        this.view = "list";
      },
      onCreate: (e) => this.handleCreate(e)
    }) : ee({
      tasks: this.tasks,
      stats: this.stats,
      filter: this.filter,
      searchQuery: this.searchQuery,
      loading: this.loading,
      error: this.error,
      onSelectTask: (e) => this.loadDetail(e),
      onFilterChange: (e) => this.handleFilterChange(e),
      onSearch: (e) => this.handleSearch(e),
      onNewTask: () => {
        this.view = "create";
      }
    });
  }
}
m([
  $()
], f.prototype, "tasks");
m([
  $()
], f.prototype, "stats");
m([
  $()
], f.prototype, "selectedTask");
m([
  $()
], f.prototype, "filter");
m([
  $()
], f.prototype, "searchQuery");
m([
  $()
], f.prototype, "view");
m([
  $()
], f.prototype, "loading");
m([
  $()
], f.prototype, "submitting");
m([
  $()
], f.prototype, "confirmingDelete");
m([
  $()
], f.prototype, "error");
const ne = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AdiTasksElement: f
}, Symbol.toStringTag, { value: "Module" }));
export {
  f as AdiTasksElement,
  ae as PluginShell,
  ae as TasksPlugin
};
