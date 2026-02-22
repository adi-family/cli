// Element Path Inspector - Page World Script
// Runs in the MAIN world (same JS context as the page) to access React fiber internals.
// Communicates with the content script via CustomEvents on document.

(() => {
  // ========== Value Serialization ==========

  function serializeValue(val, depth) {
    try {
      if (depth > 2) return "...";
      if (val === null) return "null";
      if (val === undefined) return "undefined";
      if (typeof val === "string") return val.length > 80 ? `"${val.slice(0, 80)}..."` : `"${val}"`;
      if (typeof val === "number" || typeof val === "boolean") return String(val);
      if (typeof val === "function") return `fn ${val.name || "anonymous"}()`;
      if (val && val.$$typeof) return "<React Element>";
      if (Array.isArray(val)) {
        if (val.length === 0) return "[]";
        if (depth > 1) return `[...${val.length}]`;
        const items = val.slice(0, 5).map((v) => serializeValue(v, depth + 1));
        return val.length > 5 ? `[${items.join(", ")}, ...+${val.length - 5}]` : `[${items.join(", ")}]`;
      }
      if (typeof val === "object") {
        const keys = Object.keys(val);
        if (keys.length === 0) return "{}";
        if (depth > 1) return `{...${keys.length}}`;
        const entries = keys.slice(0, 5).map((k) => `${k}: ${serializeValue(val[k], depth + 1)}`);
        return keys.length > 5 ? `{ ${entries.join(", ")}, ...+${keys.length - 5} }` : `{ ${entries.join(", ")} }`;
      }
    } catch (e) {
      return "[error]";
    }
    return String(val);
  }

  // ========== Fiber Access ==========

  function getReactFiber(element) {
    const keys = Object.keys(element);
    const fiberKey = keys.find(
      (k) =>
        k.startsWith("__reactFiber$") ||
        k.startsWith("__reactInternalInstance$"),
    );
    return fiberKey ? element[fiberKey] : null;
  }

  function extractProps(fiber) {
    const props = fiber.memoizedProps;
    if (!props || typeof props !== "object") return null;
    const result = {};
    for (const [key, val] of Object.entries(props)) {
      if (key === "children") continue;
      result[key] = serializeValue(val, 0);
    }
    return Object.keys(result).length > 0 ? result : null;
  }

  function extractState(fiber) {
    if (fiber.stateNode && fiber.stateNode.state && typeof fiber.stateNode.state === "object") {
      const result = {};
      for (const [key, val] of Object.entries(fiber.stateNode.state)) {
        result[key] = serializeValue(val, 0);
      }
      return Object.keys(result).length > 0 ? result : null;
    }
    return null;
  }

  function extractHooks(fiber) {
    let hook = fiber.memoizedState;
    if (!hook || typeof hook !== "object" || !("next" in hook)) return null;

    const hooks = [];
    let index = 0;

    while (hook) {
      const val = hook.memoizedState;
      let entry = null;

      if (hook.queue !== null && hook.queue !== undefined) {
        entry = { type: "useState", index, value: serializeValue(val, 0) };
      } else if (val && typeof val === "object" && "current" in val) {
        entry = { type: "useRef", index, value: serializeValue(val.current, 0) };
      } else if (typeof val === "function") {
        entry = { type: "useCallback", index, value: `fn ${val.name || "anonymous"}()` };
      } else if (val !== null && val !== undefined && hook.queue === null) {
        entry = { type: "useMemo", index, value: serializeValue(val, 0) };
      }

      if (entry) hooks.push(entry);
      hook = hook.next;
      index++;
      if (index > 30) break;
    }

    return hooks.length > 0 ? hooks : null;
  }

  // ========== Main Query Handler ==========

  function getReactDataForElement(element) {
    const fiber = getReactFiber(element);
    if (!fiber) return null;

    const components = [];
    let current = fiber;

    while (current) {
      if (current.type && typeof current.type === "function") {
        const name = current.type.displayName || current.type.name || null;
        if (name && !name.startsWith("_")) {
          let source = null;
          if (current._debugSource) {
            const ds = current._debugSource;
            source = ds.fileName + (ds.lineNumber ? `:${ds.lineNumber}` : "");
          }
          components.push({
            name,
            source,
            props: extractProps(current),
            state: extractState(current),
            hooks: extractHooks(current),
          });
        }
      } else if (
        current.type &&
        typeof current.type === "object" &&
        current.type.$$typeof
      ) {
        const inner = current.type.render || current.type.type || current.type;
        const name =
          current.type.displayName ||
          (typeof inner === "function" ? inner.displayName || inner.name : null);
        if (name) {
          components.push({
            name,
            source: null,
            props: extractProps(current),
            state: null,
            hooks: null,
          });
        }
      }
      current = current.return;
    }

    return components.length > 0 ? components.reverse() : null;
  }

  function getReactNameForElement(element) {
    const fiber = getReactFiber(element);
    if (!fiber) return "";
    let cur = fiber;
    while (cur) {
      if (cur.type && typeof cur.type === "function") {
        const n = cur.type.displayName || cur.type.name;
        if (n && !n.startsWith("_")) return n;
      }
      cur = cur.return;
    }
    return "";
  }

  // Listen for requests from content script
  document.addEventListener("__epi_query", (e) => {
    const { requestId, type } = e.detail;
    let result = null;

    if (type === "getReactData") {
      // Element is identified by a temporary marker attribute
      const el = document.querySelector("[data-epi-target]");
      if (el) {
        result = getReactDataForElement(el);
        el.removeAttribute("data-epi-target");
      }
    } else if (type === "getReactName") {
      const el = document.querySelector("[data-epi-target]");
      if (el) {
        result = getReactNameForElement(el);
        el.removeAttribute("data-epi-target");
      }
    }

    document.dispatchEvent(
      new CustomEvent("__epi_response", {
        detail: { requestId, result },
      }),
    );
  });

  // Signal that the page world script is ready
  document.dispatchEvent(new CustomEvent("__epi_ready"));
})();
