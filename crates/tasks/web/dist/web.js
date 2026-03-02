class mt {
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
const x = "tasks", _t = (i, t) => i.request(x, "list", t ?? {}), yt = (i, t) => i.request(x, "create", t), kt = (i, t) => i.request(x, "get", { task_id: t }), vt = (i, t) => i.request(x, "update", t), xt = (i, t) => i.request(x, "delete", { task_id: t }), wt = (i, t, e) => i.request(x, "search", { query: t, limit: e }), X = (i) => i.request(x, "stats", {});
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
class ae extends mt {
  constructor() {
    super(...arguments), this.id = "adi.tasks", this.version = "0.1.0";
  }
  async onRegister() {
    const { AdiTasksElement: t } = await Promise.resolve().then(() => oe);
    customElements.get("adi-tasks") || customElements.define("adi-tasks", t), this.bus.emit("route:register", { path: "/tasks", element: "adi-tasks" }, "tasks"), this.bus.emit("nav:add", { id: "tasks", label: "Tasks", path: "/tasks" }, "tasks"), this.bus.emit("command:register", { id: "tasks:open", label: "Go to Tasks page" }, "tasks"), this.bus.on("command:execute", ({ id: e }) => {
      e === "tasks:open" && this.bus.emit("router:navigate", { path: "/tasks" }, "tasks");
    }, "tasks"), this.bus.on("tasks:list", async ({ status: e }) => {
      try {
        const s = z(), [n, r] = await Promise.all([
          Promise.allSettled(s.map((a) => _t(a, { status: e }))),
          Promise.allSettled(s.map((a) => X(a)))
        ]), o = n.flatMap(
          (a, c) => a.status === "fulfilled" ? a.value.map((h) => ({ ...h, cocoonId: s[c].id })) : []
        ), l = r.reduce(
          (a, c) => c.status === "fulfilled" ? Y(a, c.value) : a,
          H()
        );
        this.bus.emit("tasks:list-changed", { tasks: o, stats: l }, "tasks");
      } catch (s) {
        console.error("[TasksPlugin] tasks:list error:", s), this.bus.emit("tasks:list-changed", { tasks: [], stats: H() }, "tasks");
      }
    }, "tasks"), this.bus.on("tasks:search", async ({ query: e, limit: s }) => {
      try {
        const n = z(), o = (await Promise.allSettled(n.map((l) => wt(l, e, s)))).flatMap(
          (l, a) => l.status === "fulfilled" ? l.value.map((c) => ({ ...c, cocoonId: n[a].id })) : []
        );
        this.bus.emit("tasks:search-changed", { tasks: o }, "tasks");
      } catch (n) {
        console.error("[TasksPlugin] tasks:search error:", n), this.bus.emit("tasks:search-changed", { tasks: [] }, "tasks");
      }
    }, "tasks"), this.bus.on("tasks:stats", async () => {
      try {
        const e = z(), n = (await Promise.allSettled(e.map((r) => X(r)))).reduce(
          (r, o) => o.status === "fulfilled" ? Y(r, o.value) : r,
          H()
        );
        this.bus.emit("tasks:stats-changed", { stats: n }, "tasks");
      } catch (e) {
        console.error("[TasksPlugin] tasks:stats error:", e), this.bus.emit("tasks:stats-changed", { stats: H() }, "tasks");
      }
    }, "tasks"), this.bus.on("tasks:get", async ({ task_id: e, cocoonId: s }) => {
      try {
        const n = await kt(R(s), e);
        this.bus.emit("tasks:detail-changed", {
          task: {
            task: { ...n.task, cocoonId: s },
            depends_on: n.depends_on.map((r) => ({ ...r, cocoonId: s })),
            dependents: n.dependents.map((r) => ({ ...r, cocoonId: s }))
          }
        }, "tasks");
      } catch (n) {
        console.error("[TasksPlugin] tasks:get error:", n);
      }
    }, "tasks"), this.bus.on("tasks:create", async ({ cocoonId: e, title: s, description: n, depends_on: r }) => {
      try {
        const o = await yt(R(e), { title: s, description: n, depends_on: r });
        this.bus.emit("tasks:task-mutated", { task: { ...o, cocoonId: e } }, "tasks");
      } catch (o) {
        console.error("[TasksPlugin] tasks:create error:", o);
      }
    }, "tasks"), this.bus.on("tasks:update", async ({ cocoonId: e, task_id: s, title: n, description: r, status: o }) => {
      try {
        const l = await vt(R(e), { task_id: s, title: n, description: r, status: o });
        this.bus.emit("tasks:task-mutated", { task: { ...l, cocoonId: e } }, "tasks");
      } catch (l) {
        console.error("[TasksPlugin] tasks:update error:", l);
      }
    }, "tasks"), this.bus.on("tasks:delete", async ({ cocoonId: e, task_id: s }) => {
      try {
        await xt(R(e), s), this.bus.emit("tasks:task-deleted", { task_id: s, cocoonId: e }, "tasks");
      } catch (n) {
        console.error("[TasksPlugin] tasks:delete error:", n);
      }
    }, "tasks");
  }
}
/**
 * @license
 * Copyright 2019 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const B = globalThis, Q = B.ShadowRoot && (B.ShadyCSS === void 0 || B.ShadyCSS.nativeShadow) && "adoptedStyleSheets" in Document.prototype && "replace" in CSSStyleSheet.prototype, pt = Symbol(), tt = /* @__PURE__ */ new WeakMap();
let At = class {
  constructor(t, e, s) {
    if (this._$cssResult$ = !0, s !== pt) throw Error("CSSResult is not constructable. Use `unsafeCSS` or `css` instead.");
    this.cssText = t, this.t = e;
  }
  get styleSheet() {
    let t = this.o;
    const e = this.t;
    if (Q && t === void 0) {
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
  if (Q) i.adoptedStyleSheets = t.map((e) => e instanceof CSSStyleSheet ? e : e.styleSheet);
  else for (const e of t) {
    const s = document.createElement("style"), n = B.litNonce;
    n !== void 0 && s.setAttribute("nonce", n), s.textContent = e.cssText, i.appendChild(s);
  }
}, et = Q ? (i) => i : (i) => i instanceof CSSStyleSheet ? ((t) => {
  let e = "";
  for (const s of t.cssRules) e += s.cssText;
  return St(e);
})(i) : i;
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const { is: Ct, defineProperty: Tt, getOwnPropertyDescriptor: Pt, getOwnPropertyNames: Dt, getOwnPropertySymbols: Ot, getPrototypeOf: Ut } = Object, j = globalThis, st = j.trustedTypes, Mt = st ? st.emptyScript : "", Nt = j.reactiveElementPolyfillSupport, T = (i, t) => i, I = { toAttribute(i, t) {
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
} }, V = (i, t) => !Ct(i, t), it = { attribute: !0, type: String, converter: I, reflect: !1, useDefault: !1, hasChanged: V };
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
      const s = Symbol(), n = this.getPropertyDescriptor(t, s, e);
      n !== void 0 && Tt(this.prototype, t, n);
    }
  }
  static getPropertyDescriptor(t, e, s) {
    const { get: n, set: r } = Pt(this.prototype, t) ?? { get() {
      return this[e];
    }, set(o) {
      this[e] = o;
    } };
    return { get: n, set(o) {
      const l = n?.call(this);
      r?.call(this, o), this.requestUpdate(t, l, s);
    }, configurable: !0, enumerable: !0 };
  }
  static getPropertyOptions(t) {
    return this.elementProperties.get(t) ?? it;
  }
  static _$Ei() {
    if (this.hasOwnProperty(T("elementProperties"))) return;
    const t = Ut(this);
    t.finalize(), t.l !== void 0 && (this.l = [...t.l]), this.elementProperties = new Map(t.elementProperties);
  }
  static finalize() {
    if (this.hasOwnProperty(T("finalized"))) return;
    if (this.finalized = !0, this._$Ei(), this.hasOwnProperty(T("properties"))) {
      const e = this.properties, s = [...Dt(e), ...Ot(e)];
      for (const n of s) this.createProperty(n, e[n]);
    }
    const t = this[Symbol.metadata];
    if (t !== null) {
      const e = litPropertyMetadata.get(t);
      if (e !== void 0) for (const [s, n] of e) this.elementProperties.set(s, n);
    }
    this._$Eh = /* @__PURE__ */ new Map();
    for (const [e, s] of this.elementProperties) {
      const n = this._$Eu(e, s);
      n !== void 0 && this._$Eh.set(n, e);
    }
    this.elementStyles = this.finalizeStyles(this.styles);
  }
  static finalizeStyles(t) {
    const e = [];
    if (Array.isArray(t)) {
      const s = new Set(t.flat(1 / 0).reverse());
      for (const n of s) e.unshift(et(n));
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
    const s = this.constructor.elementProperties.get(t), n = this.constructor._$Eu(t, s);
    if (n !== void 0 && s.reflect === !0) {
      const r = (s.converter?.toAttribute !== void 0 ? s.converter : I).toAttribute(e, s.type);
      this._$Em = t, r == null ? this.removeAttribute(n) : this.setAttribute(n, r), this._$Em = null;
    }
  }
  _$AK(t, e) {
    const s = this.constructor, n = s._$Eh.get(t);
    if (n !== void 0 && this._$Em !== n) {
      const r = s.getPropertyOptions(n), o = typeof r.converter == "function" ? { fromAttribute: r.converter } : r.converter?.fromAttribute !== void 0 ? r.converter : I;
      this._$Em = n;
      const l = o.fromAttribute(e, r.type);
      this[n] = l ?? this._$Ej?.get(n) ?? l, this._$Em = null;
    }
  }
  requestUpdate(t, e, s, n = !1, r) {
    if (t !== void 0) {
      const o = this.constructor;
      if (n === !1 && (r = this[t]), s ??= o.getPropertyOptions(t), !((s.hasChanged ?? V)(r, e) || s.useDefault && s.reflect && r === this._$Ej?.get(t) && !this.hasAttribute(o._$Eu(t, s)))) return;
      this.C(t, e, s);
    }
    this.isUpdatePending === !1 && (this._$ES = this._$EP());
  }
  C(t, e, { useDefault: s, reflect: n, wrapped: r }, o) {
    s && !(this._$Ej ??= /* @__PURE__ */ new Map()).has(t) && (this._$Ej.set(t, o ?? e ?? this[t]), r !== !0 || o !== void 0) || (this._$AL.has(t) || (this.hasUpdated || s || (e = void 0), this._$AL.set(t, e)), n === !0 && this._$Em !== t && (this._$Eq ??= /* @__PURE__ */ new Set()).add(t));
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
        for (const [n, r] of this._$Ep) this[n] = r;
        this._$Ep = void 0;
      }
      const s = this.constructor.elementProperties;
      if (s.size > 0) for (const [n, r] of s) {
        const { wrapped: o } = r, l = this[n];
        o !== !0 || this._$AL.has(n) || l === void 0 || this.C(n, void 0, r, l);
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
A.elementStyles = [], A.shadowRootOptions = { mode: "open" }, A[T("elementProperties")] = /* @__PURE__ */ new Map(), A[T("finalized")] = /* @__PURE__ */ new Map(), Nt?.({ ReactiveElement: A }), (j.reactiveElementVersions ??= []).push("2.1.2");
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const F = globalThis, nt = (i) => i, L = F.trustedTypes, rt = L ? L.createPolicy("lit-html", { createHTML: (i) => i }) : void 0, gt = "$lit$", _ = `lit$${Math.random().toFixed(9).slice(2)}$`, ft = "?" + _, Rt = `<${ft}>`, v = document, D = () => v.createComment(""), O = (i) => i === null || typeof i != "object" && typeof i != "function", Z = Array.isArray, Ht = (i) => Z(i) || typeof i?.[Symbol.iterator] == "function", W = `[ 	
\f\r]`, C = /<(?:(!--|\/[^a-zA-Z])|(\/?[a-zA-Z][^>\s]*)|(\/?$))/g, ot = /-->/g, at = />/g, y = RegExp(`>|${W}(?:([^\\s"'>=/]+)(${W}*=${W}*(?:[^ 	
\f\r"'\`<>=]|("|')|))|$)`, "g"), lt = /'/g, ct = /"/g, $t = /^(?:script|style|textarea|title)$/i, Bt = (i) => (t, ...e) => ({ _$litType$: i, strings: t, values: e }), p = Bt(1), S = Symbol.for("lit-noChange"), u = Symbol.for("lit-nothing"), dt = /* @__PURE__ */ new WeakMap(), k = v.createTreeWalker(v, 129);
function bt(i, t) {
  if (!Z(i) || !i.hasOwnProperty("raw")) throw Error("invalid template strings array");
  return rt !== void 0 ? rt.createHTML(t) : t;
}
const It = (i, t) => {
  const e = i.length - 1, s = [];
  let n, r = t === 2 ? "<svg>" : t === 3 ? "<math>" : "", o = C;
  for (let l = 0; l < e; l++) {
    const a = i[l];
    let c, h, d = -1, g = 0;
    for (; g < a.length && (o.lastIndex = g, h = o.exec(a), h !== null); ) g = o.lastIndex, o === C ? h[1] === "!--" ? o = ot : h[1] !== void 0 ? o = at : h[2] !== void 0 ? ($t.test(h[2]) && (n = RegExp("</" + h[2], "g")), o = y) : h[3] !== void 0 && (o = y) : o === y ? h[0] === ">" ? (o = n ?? C, d = -1) : h[1] === void 0 ? d = -2 : (d = o.lastIndex - h[2].length, c = h[1], o = h[3] === void 0 ? y : h[3] === '"' ? ct : lt) : o === ct || o === lt ? o = y : o === ot || o === at ? o = C : (o = y, n = void 0);
    const m = o === y && i[l + 1].startsWith("/>") ? " " : "";
    r += o === C ? a + Rt : d >= 0 ? (s.push(c), a.slice(0, d) + gt + a.slice(d) + _ + m) : a + _ + (d === -2 ? l : m);
  }
  return [bt(i, r + (i[e] || "<?>") + (t === 2 ? "</svg>" : t === 3 ? "</math>" : "")), s];
};
class U {
  constructor({ strings: t, _$litType$: e }, s) {
    let n;
    this.parts = [];
    let r = 0, o = 0;
    const l = t.length - 1, a = this.parts, [c, h] = It(t, e);
    if (this.el = U.createElement(c, s), k.currentNode = this.el.content, e === 2 || e === 3) {
      const d = this.el.content.firstChild;
      d.replaceWith(...d.childNodes);
    }
    for (; (n = k.nextNode()) !== null && a.length < l; ) {
      if (n.nodeType === 1) {
        if (n.hasAttributes()) for (const d of n.getAttributeNames()) if (d.endsWith(gt)) {
          const g = h[o++], m = n.getAttribute(d).split(_), N = /([.?@])?(.*)/.exec(g);
          a.push({ type: 1, index: r, name: N[2], strings: m, ctor: N[1] === "." ? jt : N[1] === "?" ? qt : N[1] === "@" ? zt : q }), n.removeAttribute(d);
        } else d.startsWith(_) && (a.push({ type: 6, index: r }), n.removeAttribute(d));
        if ($t.test(n.tagName)) {
          const d = n.textContent.split(_), g = d.length - 1;
          if (g > 0) {
            n.textContent = L ? L.emptyScript : "";
            for (let m = 0; m < g; m++) n.append(d[m], D()), k.nextNode(), a.push({ type: 2, index: ++r });
            n.append(d[g], D());
          }
        }
      } else if (n.nodeType === 8) if (n.data === ft) a.push({ type: 2, index: r });
      else {
        let d = -1;
        for (; (d = n.data.indexOf(_, d + 1)) !== -1; ) a.push({ type: 7, index: r }), d += _.length - 1;
      }
      r++;
    }
  }
  static createElement(t, e) {
    const s = v.createElement("template");
    return s.innerHTML = t, s;
  }
}
function E(i, t, e = i, s) {
  if (t === S) return t;
  let n = s !== void 0 ? e._$Co?.[s] : e._$Cl;
  const r = O(t) ? void 0 : t._$litDirective$;
  return n?.constructor !== r && (n?._$AO?.(!1), r === void 0 ? n = void 0 : (n = new r(i), n._$AT(i, e, s)), s !== void 0 ? (e._$Co ??= [])[s] = n : e._$Cl = n), n !== void 0 && (t = E(i, n._$AS(i, t.values), n, s)), t;
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
    const { el: { content: e }, parts: s } = this._$AD, n = (t?.creationScope ?? v).importNode(e, !0);
    k.currentNode = n;
    let r = k.nextNode(), o = 0, l = 0, a = s[0];
    for (; a !== void 0; ) {
      if (o === a.index) {
        let c;
        a.type === 2 ? c = new M(r, r.nextSibling, this, t) : a.type === 1 ? c = new a.ctor(r, a.name, a.strings, this, t) : a.type === 6 && (c = new Wt(r, this, t)), this._$AV.push(c), a = s[++l];
      }
      o !== a?.index && (r = k.nextNode(), o++);
    }
    return k.currentNode = v, n;
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
  constructor(t, e, s, n) {
    this.type = 2, this._$AH = u, this._$AN = void 0, this._$AA = t, this._$AB = e, this._$AM = s, this.options = n, this._$Cv = n?.isConnected ?? !0;
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
    this._$AH !== u && O(this._$AH) ? this._$AA.nextSibling.data = t : this.T(v.createTextNode(t)), this._$AH = t;
  }
  $(t) {
    const { values: e, _$litType$: s } = t, n = typeof s == "number" ? this._$AC(t) : (s.el === void 0 && (s.el = U.createElement(bt(s.h, s.h[0]), this.options)), s);
    if (this._$AH?._$AD === n) this._$AH.p(e);
    else {
      const r = new Lt(n, this), o = r.u(this.options);
      r.p(e), this.T(o), this._$AH = r;
    }
  }
  _$AC(t) {
    let e = dt.get(t.strings);
    return e === void 0 && dt.set(t.strings, e = new U(t)), e;
  }
  k(t) {
    Z(this._$AH) || (this._$AH = [], this._$AR());
    const e = this._$AH;
    let s, n = 0;
    for (const r of t) n === e.length ? e.push(s = new M(this.O(D()), this.O(D()), this, this.options)) : s = e[n], s._$AI(r), n++;
    n < e.length && (this._$AR(s && s._$AB.nextSibling, n), e.length = n);
  }
  _$AR(t = this._$AA.nextSibling, e) {
    for (this._$AP?.(!1, !0, e); t !== this._$AB; ) {
      const s = nt(t).nextSibling;
      nt(t).remove(), t = s;
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
  constructor(t, e, s, n, r) {
    this.type = 1, this._$AH = u, this._$AN = void 0, this.element = t, this.name = e, this._$AM = n, this.options = r, s.length > 2 || s[0] !== "" || s[1] !== "" ? (this._$AH = Array(s.length - 1).fill(new String()), this.strings = s) : this._$AH = u;
  }
  _$AI(t, e = this, s, n) {
    const r = this.strings;
    let o = !1;
    if (r === void 0) t = E(this, t, e, 0), o = !O(t) || t !== this._$AH && t !== S, o && (this._$AH = t);
    else {
      const l = t;
      let a, c;
      for (t = r[0], a = 0; a < r.length - 1; a++) c = E(this, l[s + a], e, a), c === S && (c = this._$AH[a]), o ||= !O(c) || c !== this._$AH[a], c === u ? t = u : t !== u && (t += (c ?? "") + r[a + 1]), this._$AH[a] = c;
    }
    o && !n && this.j(t);
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
  constructor(t, e, s, n, r) {
    super(t, e, s, n, r), this.type = 5;
  }
  _$AI(t, e = this) {
    if ((t = E(this, t, e, 0) ?? u) === S) return;
    const s = this._$AH, n = t === u && s !== u || t.capture !== s.capture || t.once !== s.once || t.passive !== s.passive, r = t !== u && (s === u || n);
    n && this.element.removeEventListener(this.name, this, s), r && this.element.addEventListener(this.name, this, t), this._$AH = t;
  }
  handleEvent(t) {
    typeof this._$AH == "function" ? this._$AH.call(this.options?.host ?? this.element, t) : this._$AH.handleEvent(t);
  }
}
class Wt {
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
const Qt = F.litHtmlPolyfillSupport;
Qt?.(U, M), (F.litHtmlVersions ??= []).push("3.3.2");
const Vt = (i, t, e) => {
  const s = e?.renderBefore ?? t;
  let n = s._$litPart$;
  if (n === void 0) {
    const r = e?.renderBefore ?? null;
    s._$litPart$ = n = new M(t.insertBefore(D(), r), r, void 0, e ?? {});
  }
  return n._$AI(i), n;
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
    this.hasUpdated || (this.renderOptions.isConnected = this.isConnected), super.update(t), this._$Do = Vt(e, this.renderRoot, this.renderOptions);
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
const Ft = J.litElementPolyfillSupport;
Ft?.({ LitElement: P });
(J.litElementVersions ??= []).push("4.2.2");
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const Zt = { attribute: !0, type: String, converter: I, reflect: !1, hasChanged: V }, Jt = (i = Zt, t, e) => {
  const { kind: s, metadata: n } = e;
  let r = globalThis.litPropertyMetadata.get(n);
  if (r === void 0 && globalThis.litPropertyMetadata.set(n, r = /* @__PURE__ */ new Map()), s === "setter" && ((i = Object.create(i)).wrapped = !0), r.set(e.name, i), s === "accessor") {
    const { name: o } = e;
    return { set(l) {
      const a = t.get.call(this);
      t.set.call(this, l), this.requestUpdate(o, a, i, !0, l);
    }, init(l) {
      return l !== void 0 && this.C(o, void 0, i, l), l;
    } };
  }
  if (s === "setter") {
    const { name: o } = e;
    return function(l) {
      const a = this[o];
      t.call(this, l), this.requestUpdate(o, a, i, !0, l);
    };
  }
  throw Error("Unsupported decorator location: " + s);
};
function Kt(i) {
  return (t, e) => typeof e == "object" ? Jt(i, t, e) : ((s, n, r) => {
    const o = n.hasOwnProperty(r);
    return n.constructor.createProperty(r, s), o ? Object.getOwnPropertyDescriptor(n, r) : void 0;
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
  const { task: t, submitting: e, confirmingDelete: s, onBack: n, onCancelDelete: r, onStatusChange: o, onDelete: l, onNavigate: a } = i, { task: c, depends_on: h, dependents: d } = t;
  return p`
    <div class="space-y-1">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${n}>
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
      c.status !== g && o(g);
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
                    @click=${r}
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
function ne(i) {
  const { connections: t, submitting: e, onBack: s, onCreate: n } = i;
  return p`
    <div class="space-y-0_75">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${s}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4">
        <h2 class="text-lg font-semibold text-gray-200 mb-4">New Task</h2>

        <form @submit=${(o) => {
    o.preventDefault();
    const l = o.target, a = new FormData(l), c = (a.get("title") ?? "").trim(), h = (a.get("description") ?? "").trim(), d = a.get("cocoonId");
    c && d && n({ title: c, description: h || void 0, cocoonId: d });
  }} class="space-y-1">
          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Connection</label>
            <select
              name="cocoonId"
              required
              ?disabled=${e}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              ${t.map((o) => p`<option value=${o.id}>${o.id}</option>`)}
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
var re = Object.defineProperty, b = (i, t, e, s) => {
  for (var n = void 0, r = i.length - 1, o; r >= 0; r--)
    (o = i[r]) && (n = o(t, e, n) || n);
  return n && re(t, e, n), n;
};
class f extends P {
  constructor() {
    super(...arguments), this.tasks = [], this.stats = null, this.selectedTask = null, this.filter = void 0, this.searchQuery = "", this.view = "list", this.loading = !1, this.submitting = !1, this.confirmingDelete = !1, this.error = null, this.unsubs = [];
  }
  createRenderRoot() {
    return this;
  }
  connectedCallback() {
    super.connectedCallback(), this.unsubs.push(
      this.bus.on("tasks:list-changed", ({ tasks: t, stats: e }) => {
        this.tasks = t, this.stats = e, this.loading = !1;
      }, "tasks-ui"),
      this.bus.on("tasks:search-changed", ({ tasks: t }) => {
        this.tasks = t, this.loading = !1;
      }, "tasks-ui"),
      this.bus.on("tasks:detail-changed", ({ task: t }) => {
        this.selectedTask = t, this.loading = !1;
      }, "tasks-ui"),
      this.bus.on("tasks:task-mutated", () => {
        this.submitting = !1, this.loadData();
      }, "tasks-ui"),
      this.bus.on("tasks:task-deleted", ({ task_id: t, cocoonId: e }) => {
        this.tasks = this.tasks.filter((s) => !(s.id === t && s.cocoonId === e)), this.view = "list", this.selectedTask = null, this.confirmingDelete = !1, this.submitting = !1;
      }, "tasks-ui"),
      this.bus.on("tasks:stats-changed", ({ stats: t }) => {
        this.stats = t;
      }, "tasks-ui")
    ), this.loadData();
  }
  disconnectedCallback() {
    super.disconnectedCallback(), this.unsubs.forEach((t) => t()), this.unsubs = [];
  }
  get bus() {
    return window.sdk.bus;
  }
  loadData() {
    this.loading = !0, this.error = null, this.searchQuery.trim() ? (this.stats = null, this.bus.emit("tasks:search", { query: this.searchQuery }, "tasks-ui")) : this.bus.emit("tasks:list", { status: this.filter }, "tasks-ui");
  }
  loadDetail(t) {
    this.loading = !0, this.view = "detail", this.bus.emit("tasks:get", { task_id: t.id, cocoonId: t.cocoonId }, "tasks-ui");
  }
  handleStatusChange(t, e) {
    this.bus.emit("tasks:update", { task_id: t.id, cocoonId: t.cocoonId, status: e }, "tasks-ui");
  }
  handleDelete(t) {
    if (!this.confirmingDelete) {
      this.confirmingDelete = !0;
      return;
    }
    this.submitting = !0, this.bus.emit("tasks:delete", { task_id: t.id, cocoonId: t.cocoonId }, "tasks-ui");
  }
  handleCreate(t) {
    this.submitting = !0, this.bus.emit("tasks:create", t, "tasks-ui"), this.view = "list";
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
    }) : this.view === "create" ? ne({
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
b([
  $()
], f.prototype, "tasks");
b([
  $()
], f.prototype, "stats");
b([
  $()
], f.prototype, "selectedTask");
b([
  $()
], f.prototype, "filter");
b([
  $()
], f.prototype, "searchQuery");
b([
  $()
], f.prototype, "view");
b([
  $()
], f.prototype, "loading");
b([
  $()
], f.prototype, "submitting");
b([
  $()
], f.prototype, "confirmingDelete");
b([
  $()
], f.prototype, "error");
const oe = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AdiTasksElement: f
}, Symbol.toStringTag, { value: "Module" }));
export {
  f as AdiTasksElement,
  ae as PluginShell,
  ae as TasksPlugin
};
