import { AdiPlugin as yt } from "@adi-family/sdk-plugin";
var kt = /* @__PURE__ */ ((s) => (s.Todo = "todo", s.InProgress = "in_progress", s.Done = "done", s.Blocked = "blocked", s.Cancelled = "cancelled", s))(kt || {});
const ue = "adi.tasks", pe = "ADI Tasks", ge = "0.8.8", fe = "core,extension";
var gt = /* @__PURE__ */ ((s) => (s.Navigate = "adi.router:navigate", s.Changed = "adi.router:changed", s.RegisterRoute = "adi.router:register-route", s))(gt || {});
const w = "tasks", xt = (s, t) => s.request(w, "list", t ?? {}), wt = (s, t) => s.request(w, "create", t), At = (s, t) => s.request(w, "get", { task_id: t }), Ct = (s, t) => s.request(w, "update", t), Et = (s, t) => s.request(w, "delete", { task_id: t }), St = (s, t, e) => s.request(w, "search", { query: t, limit: e }), K = (s) => s.request(w, "stats", {});
var ft = /* @__PURE__ */ ((s) => (s.State = "adi.signaling:state", s.AuthOk = "adi.signaling:auth-ok", s.AuthError = "adi.signaling:auth-error", s.ConnectionInfo = "adi.signaling:connection-info", s.Devices = "adi.signaling:devices", s.PeerConnected = "adi.signaling:peer-connected", s.PeerDisconnected = "adi.signaling:peer-disconnected", s.AuthAnonymous = "adi.signaling:auth-anonymous", s.DeviceRegistered = "adi.signaling:device-registered", s.DeviceDeregistered = "adi.signaling:device-deregistered", s.TagsUpdated = "adi.signaling:tags-updated", s.DeviceUpdated = "adi.signaling:device-updated", s.PairingCode = "adi.signaling:pairing-code", s.PairingConnected = "adi.signaling:pairing-connected", s.PairingFailed = "adi.signaling:pairing-failed", s.SyncData = "adi.signaling:sync-data", s))(ft || {}), L;
(function(s) {
  s.ConnectionAdded = "adi.cocoon:connection-added", s.ConnectionRemoved = "adi.cocoon:connection-removed";
})(L || (L = {}));
class V {
  connections = /* @__PURE__ */ new Map();
  devices = /* @__PURE__ */ new Map();
  pluginId;
  _bus;
  unsubs = [];
  constructor(t) {
    this.pluginId = t;
  }
  static create(t) {
    return new V(t);
  }
  init(t) {
    this._bus = t, this.unsubs.push(t.on(L.ConnectionAdded, ({ id: e, connection: i }) => {
      this.connections.set(e, i);
    }, this.pluginId), t.on(L.ConnectionRemoved, ({ id: e }) => {
      this.connections.delete(e);
    }, this.pluginId), t.on(ft.Devices, ({ devices: e }) => {
      this.devices.clear();
      for (const i of e)
        this.devices.set(i.device_id, i);
    }, this.pluginId));
  }
  destroy() {
    this.unsubs.forEach((t) => t()), this.unsubs = [], this.connections.clear(), this.devices.clear(), this._bus = void 0;
  }
  get bus() {
    if (!this._bus)
      throw new Error(`${this.pluginId}: bus not initialized`);
    return this._bus;
  }
  getConnection(t) {
    const e = this.connections.get(t);
    if (!e)
      throw new Error(`Connection '${t}' not found`);
    return e;
  }
  connectionsWithService(t) {
    return [...this.connections.values()].filter((e) => e.services.includes(t));
  }
  allConnections() {
    return [...this.connections.values()];
  }
  allDevices() {
    return [...this.devices.values()];
  }
  cocoonDevices() {
    return this.allDevices().filter((t) => t.device_type === "cocoon");
  }
}
const $ = V.create("adi.tasks");
function I() {
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
function tt(s, t) {
  return {
    total_tasks: s.total_tasks + t.total_tasks,
    todo_count: s.todo_count + t.todo_count,
    in_progress_count: s.in_progress_count + t.in_progress_count,
    done_count: s.done_count + t.done_count,
    blocked_count: s.blocked_count + t.blocked_count,
    cancelled_count: s.cancelled_count + t.cancelled_count,
    total_dependencies: s.total_dependencies + t.total_dependencies,
    has_cycles: s.has_cycles || t.has_cycles
  };
}
class $e extends yt {
  constructor() {
    super(...arguments), this.id = "adi.tasks", this.version = "0.1.0";
  }
  async onRegister() {
    $.init(this.bus);
    const { AdiTasksElement: t } = await Promise.resolve().then(() => de);
    customElements.get("adi-tasks") || customElements.define("adi-tasks", t), this.bus.emit(gt.RegisterRoute, { pluginId: this.id, path: "", init: () => document.createElement("adi-tasks"), label: "Tasks" }, this.id), this.bus.emit("nav:add", { id: this.id, label: "Tasks", path: `/${this.id}` }, this.id), this.bus.on("tasks:list", async ({ status: e }) => {
      try {
        const i = $.connectionsWithService("tasks"), [n, o] = await Promise.all([
          Promise.allSettled(i.map((a) => xt(a, { status: e }))),
          Promise.allSettled(i.map((a) => K(a)))
        ]), r = n.flatMap(
          (a, c) => a.status === "fulfilled" ? a.value.map((h) => ({ ...h, cocoonId: i[c].id })) : []
        ), l = o.reduce(
          (a, c) => c.status === "fulfilled" ? tt(a, c.value) : a,
          I()
        );
        this.bus.emit("tasks:list-changed", { tasks: r, stats: l }, "tasks");
      } catch (i) {
        console.error("[TasksPlugin] tasks:list error:", i), this.bus.emit("tasks:list-changed", { tasks: [], stats: I() }, "tasks");
      }
    }, "tasks"), this.bus.on("tasks:search", async ({ query: e, limit: i }) => {
      try {
        const n = $.connectionsWithService("tasks"), r = (await Promise.allSettled(n.map((l) => St(l, e, i)))).flatMap(
          (l, a) => l.status === "fulfilled" ? l.value.map((c) => ({ ...c, cocoonId: n[a].id })) : []
        );
        this.bus.emit("tasks:search-changed", { tasks: r }, "tasks");
      } catch (n) {
        console.error("[TasksPlugin] tasks:search error:", n), this.bus.emit("tasks:search-changed", { tasks: [] }, "tasks");
      }
    }, "tasks"), this.bus.on("tasks:stats", async () => {
      try {
        const e = $.connectionsWithService("tasks"), n = (await Promise.allSettled(e.map((o) => K(o)))).reduce(
          (o, r) => r.status === "fulfilled" ? tt(o, r.value) : o,
          I()
        );
        this.bus.emit("tasks:stats-changed", { stats: n }, "tasks");
      } catch (e) {
        console.error("[TasksPlugin] tasks:stats error:", e), this.bus.emit("tasks:stats-changed", { stats: I() }, "tasks");
      }
    }, "tasks"), this.bus.on("tasks:get", async ({ task_id: e, cocoonId: i }) => {
      try {
        const n = await At($.getConnection(i), e);
        this.bus.emit("tasks:detail-changed", {
          task: {
            task: { ...n.task, cocoonId: i },
            depends_on: n.depends_on.map((o) => ({ ...o, cocoonId: i })),
            dependents: n.dependents.map((o) => ({ ...o, cocoonId: i }))
          }
        }, "tasks");
      } catch (n) {
        console.error("[TasksPlugin] tasks:get error:", n);
      }
    }, "tasks"), this.bus.on("tasks:create", async ({ cocoonId: e, title: i, description: n, depends_on: o }) => {
      try {
        const r = await wt($.getConnection(e), { title: i, description: n, depends_on: o });
        this.bus.emit("tasks:task-mutated", { task: { ...r, cocoonId: e } }, "tasks");
      } catch (r) {
        console.error("[TasksPlugin] tasks:create error:", r);
      }
    }, "tasks"), this.bus.on("tasks:update", async ({ cocoonId: e, task_id: i, title: n, description: o, status: r }) => {
      try {
        const l = await Ct($.getConnection(e), { task_id: i, title: n, description: o, status: r });
        this.bus.emit("tasks:task-mutated", { task: { ...l, cocoonId: e } }, "tasks");
      } catch (l) {
        console.error("[TasksPlugin] tasks:update error:", l);
      }
    }, "tasks"), this.bus.on("tasks:delete", async ({ cocoonId: e, task_id: i }) => {
      try {
        await Et($.getConnection(e), i), this.bus.emit("tasks:task-deleted", { task_id: i, cocoonId: e }, "tasks");
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
const H = globalThis, F = H.ShadowRoot && (H.ShadyCSS === void 0 || H.ShadyCSS.nativeShadow) && "adoptedStyleSheets" in Document.prototype && "replace" in CSSStyleSheet.prototype, $t = Symbol(), et = /* @__PURE__ */ new WeakMap();
let Pt = class {
  constructor(t, e, i) {
    if (this._$cssResult$ = !0, i !== $t) throw Error("CSSResult is not constructable. Use `unsafeCSS` or `css` instead.");
    this.cssText = t, this.t = e;
  }
  get styleSheet() {
    let t = this.o;
    const e = this.t;
    if (F && t === void 0) {
      const i = e !== void 0 && e.length === 1;
      i && (t = et.get(e)), t === void 0 && ((this.o = t = new CSSStyleSheet()).replaceSync(this.cssText), i && et.set(e, t));
    }
    return t;
  }
  toString() {
    return this.cssText;
  }
};
const Tt = (s) => new Pt(typeof s == "string" ? s : s + "", void 0, $t), Dt = (s, t) => {
  if (F) s.adoptedStyleSheets = t.map((e) => e instanceof CSSStyleSheet ? e : e.styleSheet);
  else for (const e of t) {
    const i = document.createElement("style"), n = H.litNonce;
    n !== void 0 && i.setAttribute("nonce", n), i.textContent = e.cssText, s.appendChild(i);
  }
}, st = F ? (s) => s : (s) => s instanceof CSSStyleSheet ? ((t) => {
  let e = "";
  for (const i of t.cssRules) e += i.cssText;
  return Tt(e);
})(s) : s;
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const { is: Ut, defineProperty: Ot, getOwnPropertyDescriptor: Nt, getOwnPropertyNames: Mt, getOwnPropertySymbols: Rt, getPrototypeOf: It } = Object, z = globalThis, it = z.trustedTypes, Ht = it ? it.emptyScript : "", Lt = z.reactiveElementPolyfillSupport, T = (s, t) => s, j = { toAttribute(s, t) {
  switch (t) {
    case Boolean:
      s = s ? Ht : null;
      break;
    case Object:
    case Array:
      s = s == null ? s : JSON.stringify(s);
  }
  return s;
}, fromAttribute(s, t) {
  let e = s;
  switch (t) {
    case Boolean:
      e = s !== null;
      break;
    case Number:
      e = s === null ? null : Number(s);
      break;
    case Object:
    case Array:
      try {
        e = JSON.parse(s);
      } catch {
        e = null;
      }
  }
  return e;
} }, Q = (s, t) => !Ut(s, t), nt = { attribute: !0, type: String, converter: j, reflect: !1, useDefault: !1, hasChanged: Q };
Symbol.metadata ??= Symbol("metadata"), z.litPropertyMetadata ??= /* @__PURE__ */ new WeakMap();
let C = class extends HTMLElement {
  static addInitializer(t) {
    this._$Ei(), (this.l ??= []).push(t);
  }
  static get observedAttributes() {
    return this.finalize(), this._$Eh && [...this._$Eh.keys()];
  }
  static createProperty(t, e = nt) {
    if (e.state && (e.attribute = !1), this._$Ei(), this.prototype.hasOwnProperty(t) && ((e = Object.create(e)).wrapped = !0), this.elementProperties.set(t, e), !e.noAccessor) {
      const i = Symbol(), n = this.getPropertyDescriptor(t, i, e);
      n !== void 0 && Ot(this.prototype, t, n);
    }
  }
  static getPropertyDescriptor(t, e, i) {
    const { get: n, set: o } = Nt(this.prototype, t) ?? { get() {
      return this[e];
    }, set(r) {
      this[e] = r;
    } };
    return { get: n, set(r) {
      const l = n?.call(this);
      o?.call(this, r), this.requestUpdate(t, l, i);
    }, configurable: !0, enumerable: !0 };
  }
  static getPropertyOptions(t) {
    return this.elementProperties.get(t) ?? nt;
  }
  static _$Ei() {
    if (this.hasOwnProperty(T("elementProperties"))) return;
    const t = It(this);
    t.finalize(), t.l !== void 0 && (this.l = [...t.l]), this.elementProperties = new Map(t.elementProperties);
  }
  static finalize() {
    if (this.hasOwnProperty(T("finalized"))) return;
    if (this.finalized = !0, this._$Ei(), this.hasOwnProperty(T("properties"))) {
      const e = this.properties, i = [...Mt(e), ...Rt(e)];
      for (const n of i) this.createProperty(n, e[n]);
    }
    const t = this[Symbol.metadata];
    if (t !== null) {
      const e = litPropertyMetadata.get(t);
      if (e !== void 0) for (const [i, n] of e) this.elementProperties.set(i, n);
    }
    this._$Eh = /* @__PURE__ */ new Map();
    for (const [e, i] of this.elementProperties) {
      const n = this._$Eu(e, i);
      n !== void 0 && this._$Eh.set(n, e);
    }
    this.elementStyles = this.finalizeStyles(this.styles);
  }
  static finalizeStyles(t) {
    const e = [];
    if (Array.isArray(t)) {
      const i = new Set(t.flat(1 / 0).reverse());
      for (const n of i) e.unshift(st(n));
    } else t !== void 0 && e.push(st(t));
    return e;
  }
  static _$Eu(t, e) {
    const i = e.attribute;
    return i === !1 ? void 0 : typeof i == "string" ? i : typeof t == "string" ? t.toLowerCase() : void 0;
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
    for (const i of e.keys()) this.hasOwnProperty(i) && (t.set(i, this[i]), delete this[i]);
    t.size > 0 && (this._$Ep = t);
  }
  createRenderRoot() {
    const t = this.shadowRoot ?? this.attachShadow(this.constructor.shadowRootOptions);
    return Dt(t, this.constructor.elementStyles), t;
  }
  connectedCallback() {
    this.renderRoot ??= this.createRenderRoot(), this.enableUpdating(!0), this._$EO?.forEach((t) => t.hostConnected?.());
  }
  enableUpdating(t) {
  }
  disconnectedCallback() {
    this._$EO?.forEach((t) => t.hostDisconnected?.());
  }
  attributeChangedCallback(t, e, i) {
    this._$AK(t, i);
  }
  _$ET(t, e) {
    const i = this.constructor.elementProperties.get(t), n = this.constructor._$Eu(t, i);
    if (n !== void 0 && i.reflect === !0) {
      const o = (i.converter?.toAttribute !== void 0 ? i.converter : j).toAttribute(e, i.type);
      this._$Em = t, o == null ? this.removeAttribute(n) : this.setAttribute(n, o), this._$Em = null;
    }
  }
  _$AK(t, e) {
    const i = this.constructor, n = i._$Eh.get(t);
    if (n !== void 0 && this._$Em !== n) {
      const o = i.getPropertyOptions(n), r = typeof o.converter == "function" ? { fromAttribute: o.converter } : o.converter?.fromAttribute !== void 0 ? o.converter : j;
      this._$Em = n;
      const l = r.fromAttribute(e, o.type);
      this[n] = l ?? this._$Ej?.get(n) ?? l, this._$Em = null;
    }
  }
  requestUpdate(t, e, i, n = !1, o) {
    if (t !== void 0) {
      const r = this.constructor;
      if (n === !1 && (o = this[t]), i ??= r.getPropertyOptions(t), !((i.hasChanged ?? Q)(o, e) || i.useDefault && i.reflect && o === this._$Ej?.get(t) && !this.hasAttribute(r._$Eu(t, i)))) return;
      this.C(t, e, i);
    }
    this.isUpdatePending === !1 && (this._$ES = this._$EP());
  }
  C(t, e, { useDefault: i, reflect: n, wrapped: o }, r) {
    i && !(this._$Ej ??= /* @__PURE__ */ new Map()).has(t) && (this._$Ej.set(t, r ?? e ?? this[t]), o !== !0 || r !== void 0) || (this._$AL.has(t) || (this.hasUpdated || i || (e = void 0), this._$AL.set(t, e)), n === !0 && this._$Em !== t && (this._$Eq ??= /* @__PURE__ */ new Set()).add(t));
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
        for (const [n, o] of this._$Ep) this[n] = o;
        this._$Ep = void 0;
      }
      const i = this.constructor.elementProperties;
      if (i.size > 0) for (const [n, o] of i) {
        const { wrapped: r } = o, l = this[n];
        r !== !0 || this._$AL.has(n) || l === void 0 || this.C(n, void 0, o, l);
      }
    }
    let t = !1;
    const e = this._$AL;
    try {
      t = this.shouldUpdate(e), t ? (this.willUpdate(e), this._$EO?.forEach((i) => i.hostUpdate?.()), this.update(e)) : this._$EM();
    } catch (i) {
      throw t = !1, this._$EM(), i;
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
C.elementStyles = [], C.shadowRootOptions = { mode: "open" }, C[T("elementProperties")] = /* @__PURE__ */ new Map(), C[T("finalized")] = /* @__PURE__ */ new Map(), Lt?.({ ReactiveElement: C }), (z.reactiveElementVersions ??= []).push("2.1.2");
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const G = globalThis, ot = (s) => s, q = G.trustedTypes, rt = q ? q.createPolicy("lit-html", { createHTML: (s) => s }) : void 0, bt = "$lit$", v = `lit$${Math.random().toFixed(9).slice(2)}$`, mt = "?" + v, jt = `<${mt}>`, x = document, U = () => x.createComment(""), O = (s) => s === null || typeof s != "object" && typeof s != "function", Z = Array.isArray, qt = (s) => Z(s) || typeof s?.[Symbol.iterator] == "function", B = `[ 	
\f\r]`, P = /<(?:(!--|\/[^a-zA-Z])|(\/?[a-zA-Z][^>\s]*)|(\/?$))/g, at = /-->/g, lt = />/g, y = RegExp(`>|${B}(?:([^\\s"'>=/]+)(${B}*=${B}*(?:[^ 	
\f\r"'\`<>=]|("|')|))|$)`, "g"), ct = /'/g, dt = /"/g, _t = /^(?:script|style|textarea|title)$/i, zt = (s) => (t, ...e) => ({ _$litType$: s, strings: t, values: e }), p = zt(1), E = Symbol.for("lit-noChange"), u = Symbol.for("lit-nothing"), ht = /* @__PURE__ */ new WeakMap(), k = x.createTreeWalker(x, 129);
function vt(s, t) {
  if (!Z(s) || !s.hasOwnProperty("raw")) throw Error("invalid template strings array");
  return rt !== void 0 ? rt.createHTML(t) : t;
}
const Wt = (s, t) => {
  const e = s.length - 1, i = [];
  let n, o = t === 2 ? "<svg>" : t === 3 ? "<math>" : "", r = P;
  for (let l = 0; l < e; l++) {
    const a = s[l];
    let c, h, d = -1, g = 0;
    for (; g < a.length && (r.lastIndex = g, h = r.exec(a), h !== null); ) g = r.lastIndex, r === P ? h[1] === "!--" ? r = at : h[1] !== void 0 ? r = lt : h[2] !== void 0 ? (_t.test(h[2]) && (n = RegExp("</" + h[2], "g")), r = y) : h[3] !== void 0 && (r = y) : r === y ? h[0] === ">" ? (r = n ?? P, d = -1) : h[1] === void 0 ? d = -2 : (d = r.lastIndex - h[2].length, c = h[1], r = h[3] === void 0 ? y : h[3] === '"' ? dt : ct) : r === dt || r === ct ? r = y : r === at || r === lt ? r = P : (r = y, n = void 0);
    const _ = r === y && s[l + 1].startsWith("/>") ? " " : "";
    o += r === P ? a + jt : d >= 0 ? (i.push(c), a.slice(0, d) + bt + a.slice(d) + v + _) : a + v + (d === -2 ? l : _);
  }
  return [vt(s, o + (s[e] || "<?>") + (t === 2 ? "</svg>" : t === 3 ? "</math>" : "")), i];
};
class N {
  constructor({ strings: t, _$litType$: e }, i) {
    let n;
    this.parts = [];
    let o = 0, r = 0;
    const l = t.length - 1, a = this.parts, [c, h] = Wt(t, e);
    if (this.el = N.createElement(c, i), k.currentNode = this.el.content, e === 2 || e === 3) {
      const d = this.el.content.firstChild;
      d.replaceWith(...d.childNodes);
    }
    for (; (n = k.nextNode()) !== null && a.length < l; ) {
      if (n.nodeType === 1) {
        if (n.hasAttributes()) for (const d of n.getAttributeNames()) if (d.endsWith(bt)) {
          const g = h[r++], _ = n.getAttribute(d).split(v), R = /([.?@])?(.*)/.exec(g);
          a.push({ type: 1, index: o, name: R[2], strings: _, ctor: R[1] === "." ? Vt : R[1] === "?" ? Ft : R[1] === "@" ? Qt : W }), n.removeAttribute(d);
        } else d.startsWith(v) && (a.push({ type: 6, index: o }), n.removeAttribute(d));
        if (_t.test(n.tagName)) {
          const d = n.textContent.split(v), g = d.length - 1;
          if (g > 0) {
            n.textContent = q ? q.emptyScript : "";
            for (let _ = 0; _ < g; _++) n.append(d[_], U()), k.nextNode(), a.push({ type: 2, index: ++o });
            n.append(d[g], U());
          }
        }
      } else if (n.nodeType === 8) if (n.data === mt) a.push({ type: 2, index: o });
      else {
        let d = -1;
        for (; (d = n.data.indexOf(v, d + 1)) !== -1; ) a.push({ type: 7, index: o }), d += v.length - 1;
      }
      o++;
    }
  }
  static createElement(t, e) {
    const i = x.createElement("template");
    return i.innerHTML = t, i;
  }
}
function S(s, t, e = s, i) {
  if (t === E) return t;
  let n = i !== void 0 ? e._$Co?.[i] : e._$Cl;
  const o = O(t) ? void 0 : t._$litDirective$;
  return n?.constructor !== o && (n?._$AO?.(!1), o === void 0 ? n = void 0 : (n = new o(s), n._$AT(s, e, i)), i !== void 0 ? (e._$Co ??= [])[i] = n : e._$Cl = n), n !== void 0 && (t = S(s, n._$AS(s, t.values), n, i)), t;
}
class Bt {
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
    const { el: { content: e }, parts: i } = this._$AD, n = (t?.creationScope ?? x).importNode(e, !0);
    k.currentNode = n;
    let o = k.nextNode(), r = 0, l = 0, a = i[0];
    for (; a !== void 0; ) {
      if (r === a.index) {
        let c;
        a.type === 2 ? c = new M(o, o.nextSibling, this, t) : a.type === 1 ? c = new a.ctor(o, a.name, a.strings, this, t) : a.type === 6 && (c = new Gt(o, this, t)), this._$AV.push(c), a = i[++l];
      }
      r !== a?.index && (o = k.nextNode(), r++);
    }
    return k.currentNode = x, n;
  }
  p(t) {
    let e = 0;
    for (const i of this._$AV) i !== void 0 && (i.strings !== void 0 ? (i._$AI(t, i, e), e += i.strings.length - 2) : i._$AI(t[e])), e++;
  }
}
class M {
  get _$AU() {
    return this._$AM?._$AU ?? this._$Cv;
  }
  constructor(t, e, i, n) {
    this.type = 2, this._$AH = u, this._$AN = void 0, this._$AA = t, this._$AB = e, this._$AM = i, this.options = n, this._$Cv = n?.isConnected ?? !0;
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
    t = S(this, t, e), O(t) ? t === u || t == null || t === "" ? (this._$AH !== u && this._$AR(), this._$AH = u) : t !== this._$AH && t !== E && this._(t) : t._$litType$ !== void 0 ? this.$(t) : t.nodeType !== void 0 ? this.T(t) : qt(t) ? this.k(t) : this._(t);
  }
  O(t) {
    return this._$AA.parentNode.insertBefore(t, this._$AB);
  }
  T(t) {
    this._$AH !== t && (this._$AR(), this._$AH = this.O(t));
  }
  _(t) {
    this._$AH !== u && O(this._$AH) ? this._$AA.nextSibling.data = t : this.T(x.createTextNode(t)), this._$AH = t;
  }
  $(t) {
    const { values: e, _$litType$: i } = t, n = typeof i == "number" ? this._$AC(t) : (i.el === void 0 && (i.el = N.createElement(vt(i.h, i.h[0]), this.options)), i);
    if (this._$AH?._$AD === n) this._$AH.p(e);
    else {
      const o = new Bt(n, this), r = o.u(this.options);
      o.p(e), this.T(r), this._$AH = o;
    }
  }
  _$AC(t) {
    let e = ht.get(t.strings);
    return e === void 0 && ht.set(t.strings, e = new N(t)), e;
  }
  k(t) {
    Z(this._$AH) || (this._$AH = [], this._$AR());
    const e = this._$AH;
    let i, n = 0;
    for (const o of t) n === e.length ? e.push(i = new M(this.O(U()), this.O(U()), this, this.options)) : i = e[n], i._$AI(o), n++;
    n < e.length && (this._$AR(i && i._$AB.nextSibling, n), e.length = n);
  }
  _$AR(t = this._$AA.nextSibling, e) {
    for (this._$AP?.(!1, !0, e); t !== this._$AB; ) {
      const i = ot(t).nextSibling;
      ot(t).remove(), t = i;
    }
  }
  setConnected(t) {
    this._$AM === void 0 && (this._$Cv = t, this._$AP?.(t));
  }
}
class W {
  get tagName() {
    return this.element.tagName;
  }
  get _$AU() {
    return this._$AM._$AU;
  }
  constructor(t, e, i, n, o) {
    this.type = 1, this._$AH = u, this._$AN = void 0, this.element = t, this.name = e, this._$AM = n, this.options = o, i.length > 2 || i[0] !== "" || i[1] !== "" ? (this._$AH = Array(i.length - 1).fill(new String()), this.strings = i) : this._$AH = u;
  }
  _$AI(t, e = this, i, n) {
    const o = this.strings;
    let r = !1;
    if (o === void 0) t = S(this, t, e, 0), r = !O(t) || t !== this._$AH && t !== E, r && (this._$AH = t);
    else {
      const l = t;
      let a, c;
      for (t = o[0], a = 0; a < o.length - 1; a++) c = S(this, l[i + a], e, a), c === E && (c = this._$AH[a]), r ||= !O(c) || c !== this._$AH[a], c === u ? t = u : t !== u && (t += (c ?? "") + o[a + 1]), this._$AH[a] = c;
    }
    r && !n && this.j(t);
  }
  j(t) {
    t === u ? this.element.removeAttribute(this.name) : this.element.setAttribute(this.name, t ?? "");
  }
}
class Vt extends W {
  constructor() {
    super(...arguments), this.type = 3;
  }
  j(t) {
    this.element[this.name] = t === u ? void 0 : t;
  }
}
class Ft extends W {
  constructor() {
    super(...arguments), this.type = 4;
  }
  j(t) {
    this.element.toggleAttribute(this.name, !!t && t !== u);
  }
}
class Qt extends W {
  constructor(t, e, i, n, o) {
    super(t, e, i, n, o), this.type = 5;
  }
  _$AI(t, e = this) {
    if ((t = S(this, t, e, 0) ?? u) === E) return;
    const i = this._$AH, n = t === u && i !== u || t.capture !== i.capture || t.once !== i.once || t.passive !== i.passive, o = t !== u && (i === u || n);
    n && this.element.removeEventListener(this.name, this, i), o && this.element.addEventListener(this.name, this, t), this._$AH = t;
  }
  handleEvent(t) {
    typeof this._$AH == "function" ? this._$AH.call(this.options?.host ?? this.element, t) : this._$AH.handleEvent(t);
  }
}
class Gt {
  constructor(t, e, i) {
    this.element = t, this.type = 6, this._$AN = void 0, this._$AM = e, this.options = i;
  }
  get _$AU() {
    return this._$AM._$AU;
  }
  _$AI(t) {
    S(this, t);
  }
}
const Zt = G.litHtmlPolyfillSupport;
Zt?.(N, M), (G.litHtmlVersions ??= []).push("3.3.2");
const Jt = (s, t, e) => {
  const i = e?.renderBefore ?? t;
  let n = i._$litPart$;
  if (n === void 0) {
    const o = e?.renderBefore ?? null;
    i._$litPart$ = n = new M(t.insertBefore(U(), o), o, void 0, e ?? {});
  }
  return n._$AI(s), n;
};
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const J = globalThis;
class D extends C {
  constructor() {
    super(...arguments), this.renderOptions = { host: this }, this._$Do = void 0;
  }
  createRenderRoot() {
    const t = super.createRenderRoot();
    return this.renderOptions.renderBefore ??= t.firstChild, t;
  }
  update(t) {
    const e = this.render();
    this.hasUpdated || (this.renderOptions.isConnected = this.isConnected), super.update(t), this._$Do = Jt(e, this.renderRoot, this.renderOptions);
  }
  connectedCallback() {
    super.connectedCallback(), this._$Do?.setConnected(!0);
  }
  disconnectedCallback() {
    super.disconnectedCallback(), this._$Do?.setConnected(!1);
  }
  render() {
    return E;
  }
}
D._$litElement$ = !0, D.finalized = !0, J.litElementHydrateSupport?.({ LitElement: D });
const Yt = J.litElementPolyfillSupport;
Yt?.({ LitElement: D });
(J.litElementVersions ??= []).push("4.2.2");
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
const Xt = { attribute: !0, type: String, converter: j, reflect: !1, hasChanged: Q }, Kt = (s = Xt, t, e) => {
  const { kind: i, metadata: n } = e;
  let o = globalThis.litPropertyMetadata.get(n);
  if (o === void 0 && globalThis.litPropertyMetadata.set(n, o = /* @__PURE__ */ new Map()), i === "setter" && ((s = Object.create(s)).wrapped = !0), o.set(e.name, s), i === "accessor") {
    const { name: r } = e;
    return { set(l) {
      const a = t.get.call(this);
      t.set.call(this, l), this.requestUpdate(r, a, s, !0, l);
    }, init(l) {
      return l !== void 0 && this.C(r, void 0, s, l), l;
    } };
  }
  if (i === "setter") {
    const { name: r } = e;
    return function(l) {
      const a = this[r];
      t.call(this, l), this.requestUpdate(r, a, s, !0, l);
    };
  }
  throw Error("Unsupported decorator location: " + i);
};
function te(s) {
  return (t, e) => typeof e == "object" ? Kt(s, t, e) : ((i, n, o) => {
    const r = n.hasOwnProperty(o);
    return n.constructor.createProperty(o, i), r ? Object.getOwnPropertyDescriptor(n, o) : void 0;
  })(s, t, e);
}
/**
 * @license
 * Copyright 2017 Google LLC
 * SPDX-License-Identifier: BSD-3-Clause
 */
function b(s) {
  return te({ ...s, state: !0, attribute: !1 });
}
const Y = {
  todo: "bg-gray-500/20 text-gray-300",
  in_progress: "bg-blue-500/20 text-blue-300",
  done: "bg-green-500/20 text-green-300",
  blocked: "bg-red-500/20 text-red-300",
  cancelled: "bg-gray-600/20 text-gray-400"
}, X = {
  todo: "Todo",
  in_progress: "In Progress",
  done: "Done",
  blocked: "Blocked",
  cancelled: "Cancelled"
};
function ee(s) {
  const t = Math.floor((Date.now() - s * 1e3) / 1e3);
  if (t < 60) return `${t}s ago`;
  const e = Math.floor(t / 60);
  if (e < 60) return `${e}m ago`;
  const i = Math.floor(e / 60);
  return i < 24 ? `${i}h ago` : `${Math.floor(i / 24)}d ago`;
}
const A = (s, t, e) => p`
  <div class="flex flex-col items-center px-3 py-1.5 rounded-lg ${e}">
    <span class="text-lg font-semibold">${t}</span>
    <span class="text-xs opacity-70">${s}</span>
  </div>
`, se = (s) => s ? p`
    <div class="flex gap-2 flex-wrap mb-4">
      ${A("Total", s.total_tasks, "bg-white/5 text-gray-300")}
      ${A("Todo", s.todo_count, "bg-gray-500/10 text-gray-300")}
      ${A("In Progress", s.in_progress_count, "bg-blue-500/10 text-blue-300")}
      ${A("Done", s.done_count, "bg-green-500/10 text-green-300")}
      ${A("Blocked", s.blocked_count, "bg-red-500/10 text-red-300")}
      ${A("Cancelled", s.cancelled_count, "bg-gray-600/10 text-gray-400")}
    </div>
  ` : u, ie = (s, t) => p`
    <div class="flex gap-1 mb-4 flex-wrap">
      ${[
  { label: "All", value: void 0 },
  { label: "Todo", value: "todo" },
  { label: "In Progress", value: "in_progress" },
  { label: "Done", value: "done" },
  { label: "Blocked", value: "blocked" },
  { label: "Cancelled", value: "cancelled" }
].map(
  (i) => p`
          <button
            class="px-3 py-1 rounded-full text-sm transition-colors ${s === i.value ? "bg-purple-500/30 text-purple-200 font-medium" : "bg-white/5 text-gray-400 hover:bg-white/10 hover:text-gray-200"}"
            @click=${() => t(i.value)}
          >
            ${i.label}
          </button>
        `
)}
    </div>
  `, ne = (s, t) => p`
  <button
    class="w-full text-left p-3 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-start gap-3 group"
    @click=${() => t(s)}
  >
    <span class="inline-flex px-2 py-0.5 rounded text-xs font-medium shrink-0 mt-0.5 ${Y[s.status]}">
      ${X[s.status]}
    </span>
    <div class="flex-1 min-w-0">
      <div class="text-sm text-gray-200 group-hover:text-white truncate">${s.title}</div>
      ${s.description ? p`<div class="text-xs text-gray-500 mt-0.5 truncate">${s.description}</div>` : u}
    </div>
    <span class="text-xs text-gray-600 shrink-0 mt-0.5">${ee(s.updated_at)}</span>
  </button>
`;
function oe(s) {
  return p`
    <div class="space-y-3">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-lg font-semibold text-gray-200">Tasks</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-purple-500/20 text-purple-200 hover:bg-purple-500/30 transition-colors text-sm font-medium"
          @click=${s.onNewTask}
        >
          + New Task
        </button>
      </div>

      ${se(s.stats)}
      ${ie(s.filter, s.onFilterChange)}

      <div class="relative mb-3">
        <input
          type="text"
          placeholder="Search tasks..."
          .value=${s.searchQuery}
          @input=${(t) => s.onSearch(t.target.value)}
          class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:border-purple-500/50"
        />
      </div>

      ${s.error ? p`<div class="p-3 rounded-lg bg-red-500/10 border border-red-500/20 text-sm text-red-300">${s.error}</div>` : s.loading ? p`<div class="text-center py-8 text-gray-500 text-sm">Loading...</div>` : s.tasks.length === 0 ? p`<div class="text-center py-8 text-gray-500 text-sm">No tasks found</div>` : p`
                <div class="space-y-2">
                  ${s.tasks.map((t) => ne(t, s.onSelectTask))}
                </div>
              `}
    </div>
  `;
}
const re = ["todo", "in_progress", "done", "blocked", "cancelled"], ut = (s) => new Date(s * 1e3).toLocaleString(), pt = (s, t, e) => t.length === 0 ? u : p`
    <div class="mt-4">
      <h4 class="text-xs font-medium text-gray-400 uppercase tracking-wider mb-2">${s}</h4>
      <div class="space-y-1">
        ${t.map(
  (i) => p`
            <button
              class="w-full text-left px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 transition-colors flex items-center gap-2 text-sm"
              @click=${() => e(i)}
            >
              <span class="inline-flex px-1.5 py-0.5 rounded text-xs ${Y[i.status]}">
                ${X[i.status]}
              </span>
              <span class="text-gray-300 truncate">${i.title}</span>
            </button>
          `
)}
      </div>
    </div>
  `;
function ae(s) {
  const { task: t, submitting: e, confirmingDelete: i, onBack: n, onCancelDelete: o, onStatusChange: r, onDelete: l, onNavigate: a } = s, { task: c, depends_on: h, dependents: d } = t;
  return p`
    <div class="space-y-4">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${n}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4 space-y-4">
        <h2 class="text-lg font-semibold text-gray-100">${c.title}</h2>

        ${c.description ? p`<p class="text-sm text-gray-400 whitespace-pre-wrap">${c.description}</p>` : u}

        <div class="flex items-center gap-3 flex-wrap">
          <label class="text-xs text-gray-500 uppercase tracking-wider">Status</label>
          <div class="flex gap-1 flex-wrap">
            ${re.map(
    (g) => p`
                <button
                  class="px-2.5 py-1 rounded text-xs transition-colors ${c.status === g ? Y[g] + " font-medium ring-1 ring-white/20" : "bg-white/5 text-gray-500 hover:bg-white/10 hover:text-gray-300"}"
                  ?disabled=${e}
                  @click=${() => {
      c.status !== g && r(g);
    }}
                >
                  ${X[g]}
                </button>
              `
  )}
          </div>
        </div>

        <div class="flex gap-4 text-xs text-gray-500">
          <span>Created: ${ut(c.created_at)}</span>
          <span>Updated: ${ut(c.updated_at)}</span>
        </div>

        ${pt("Depends on", h, a)}
        ${pt("Blocked by this", d, a)}

        <div class="pt-3 border-t border-white/10">
          ${i ? p`
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
function le(s) {
  const { connections: t, submitting: e, onBack: i, onCreate: n } = s;
  return p`
    <div class="space-y-3">
      <button class="text-sm text-gray-400 hover:text-gray-200 transition-colors" @click=${i}>
        &larr; Back to list
      </button>

      <div class="bg-white/5 rounded-xl p-4">
        <h2 class="text-lg font-semibold text-gray-200 mb-4">New Task</h2>

        <form @submit=${(r) => {
    r.preventDefault();
    const l = r.target, a = new FormData(l), c = (a.get("title") ?? "").trim(), h = (a.get("description") ?? "").trim(), d = a.get("cocoonId");
    c && d && n({ title: c, description: h || void 0, cocoonId: d });
  }} class="space-y-4">
          <div>
            <label class="block text-xs text-gray-400 uppercase tracking-wider mb-1">Connection</label>
            <select
              name="cocoonId"
              required
              ?disabled=${e}
              class="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-gray-200 focus:outline-none focus:border-purple-500/50 disabled:opacity-50"
            >
              ${t.map((r) => p`<option value=${r.id}>${r.id}</option>`)}
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
              @click=${i}
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
var ce = Object.defineProperty, m = (s, t, e, i) => {
  for (var n = void 0, o = s.length - 1, r; o >= 0; o--)
    (r = s[o]) && (n = r(t, e, n) || n);
  return n && ce(t, e, n), n;
};
class f extends D {
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
        this.tasks = this.tasks.filter((i) => !(i.id === t && i.cocoonId === e)), this.view = "list", this.selectedTask = null, this.confirmingDelete = !1, this.submitting = !1;
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
    return $.bus;
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
    const t = $.allConnections();
    return this.view === "detail" && this.selectedTask ? ae({
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
    }) : this.view === "create" ? le({
      connections: t,
      submitting: this.submitting,
      onBack: () => {
        this.view = "list";
      },
      onCreate: (e) => this.handleCreate(e)
    }) : oe({
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
  b()
], f.prototype, "tasks");
m([
  b()
], f.prototype, "stats");
m([
  b()
], f.prototype, "selectedTask");
m([
  b()
], f.prototype, "filter");
m([
  b()
], f.prototype, "searchQuery");
m([
  b()
], f.prototype, "view");
m([
  b()
], f.prototype, "loading");
m([
  b()
], f.prototype, "submitting");
m([
  b()
], f.prototype, "confirmingDelete");
m([
  b()
], f.prototype, "error");
const de = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.defineProperty({
  __proto__: null,
  AdiTasksElement: f
}, Symbol.toStringTag, { value: "Module" }));
export {
  ue as PLUGIN_ID,
  pe as PLUGIN_NAME,
  fe as PLUGIN_TYPE,
  ge as PLUGIN_VERSION,
  $e as PluginShell,
  kt as TaskStatus,
  $e as TasksPlugin
};
