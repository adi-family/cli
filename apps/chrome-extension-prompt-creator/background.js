// Element Path Inspector - Background Service Worker
// Clicking the extension icon injects scripts and toggles the sidebar.
// Debugger is attached to every tab as early as possible to capture all console output.

// ========== Helpers ==========

const SKIP_PROTOCOLS = ["chrome:", "chrome-extension:", "about:", "data:", "javascript:", "blob:"];

function shouldAttach(url) {
  if (!url) return true; // attach even before URL is known (onCreated)
  try {
    return !SKIP_PROTOCOLS.includes(new URL(url).protocol);
  } catch {
    return false;
  }
}

// target can be { tabId } or { tabId, sessionId } for child sessions
function sendDebugCommand(target, method, params = {}) {
  return new Promise((resolve, reject) => {
    chrome.debugger.sendCommand(target, method, params, (result) => {
      if (chrome.runtime.lastError) { reject(chrome.runtime.lastError); return; }
      resolve(result);
    });
  });
}

async function attachDebugger(tabId) {
  try {
    await new Promise((resolve, reject) => {
      chrome.debugger.attach({ tabId }, "1.3", () => {
        if (chrome.runtime.lastError) { reject(chrome.runtime.lastError); return; }
        resolve();
      });
    });
  } catch (err) {
    // Already attached — fall through to enable domains
    if (!err.message?.includes("already")) return;
  }
  // Auto-attach to child targets (iframes, workers) via flat sessions.
  // onEvent handles Target.attachedToTarget and enables Runtime/Log there.
  await sendDebugCommand({ tabId }, "Target.setAutoAttach", {
    autoAttach: true,
    waitForDebuggerOnStart: false,
    flatten: true,
  }).catch(() => {});
  await sendDebugCommand({ tabId }, "Runtime.enable").catch(() => {});
  await sendDebugCommand({ tabId }, "Log.enable").catch(() => {});
}

// ========== Format CDP RemoteObject ==========

function formatArg(arg) {
  if (arg.type === "string") return arg.value;
  if (arg.type === "number" || arg.type === "boolean") return String(arg.value);
  if (arg.type === "undefined") return "undefined";
  if (arg.type === "object") {
    if (arg.subtype === "null") return "null";
    return arg.description || arg.className || "[object]";
  }
  if (arg.type === "function") return arg.description || "[function]";
  return arg.value != null ? String(arg.value) : (arg.description || arg.type);
}

// ========== Per-tab log buffer ==========

const MAX_BUFFER = 500;
const tabLogs = new Map(); // tabId -> entry[]

function bufferEntry(tabId, entry) {
  if (!tabLogs.has(tabId)) tabLogs.set(tabId, []);
  const buf = tabLogs.get(tabId);
  buf.push(entry);
  if (buf.length > MAX_BUFFER) buf.shift();
}

// ========== CDP Event Listener ==========

chrome.debugger.onEvent.addListener((source, method, params) => {
  // New target attached (navigation, iframe, worker) — enable domains on the new session
  if (method === "Target.attachedToTarget") {
    const session = { ...source, sessionId: params.sessionId };
    // Top-level page navigation = fresh history
    if (params.targetInfo?.type === "page" && source.tabId) {
      tabLogs.delete(source.tabId);
    }
    sendDebugCommand(session, "Runtime.enable").catch(() => {});
    sendDebugCommand(session, "Log.enable").catch(() => {});
    return;
  }

  const tabId = source.tabId;
  if (!tabId) return;

  let entry = null;
  if (method === "Runtime.consoleAPICalled") {
    const text = (params.args || []).map(formatArg).join(" ");
    entry = { type: params.type, text, source: "console" };
  } else if (method === "Runtime.exceptionThrown") {
    const details = params.exceptionDetails;
    const text = details.exception?.description || details.text || "Uncaught exception";
    entry = { type: "error", text, source: "exception" };
  } else if (method === "Log.entryAdded") {
    const e = params.entry;
    entry = { type: e.level, text: e.text, source: e.source, url: e.url, line: e.lineNumber };
  }

  if (!entry) return;
  bufferEntry(tabId, entry);
  chrome.tabs.sendMessage(tabId, { action: "consoleEntry", entry }).catch(() => {});
});

// Content script asks for buffered history when sidebar opens
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.action === "getHistory" && sender.tab?.id) {
    sendResponse({ entries: tabLogs.get(sender.tab.id) || [] });
  }
  if (msg.action === "clearHistory" && sender.tab?.id) {
    tabLogs.delete(sender.tab.id);
  }
});

// ========== Auto-attach to all tabs ==========

// Attach to all tabs already open when the service worker starts
chrome.tabs.query({}).then((tabs) => {
  for (const tab of tabs) {
    if (tab.id && shouldAttach(tab.url)) attachDebugger(tab.id);
  }
}).catch(() => {});

// Attach to newly created tabs immediately
chrome.tabs.onCreated.addListener((tab) => {
  if (tab.id) attachDebugger(tab.id);
});

// Re-attach after unexpected detach (not from tab close or user opening DevTools)
// Navigation is handled by Target.attachedToTarget via Target.setAutoAttach.
chrome.debugger.onDetach.addListener((source, reason) => {
  if (reason === "canceled_by_user" || !source.tabId) return;
  if (reason === "target_closed") return;
  chrome.tabs.get(source.tabId, (tab) => {
    if (chrome.runtime.lastError || !tab) return;
    if (shouldAttach(tab.url)) attachDebugger(source.tabId);
  });
});

// Clean up on tab close
chrome.tabs.onRemoved.addListener((tabId) => {
  tabLogs.delete(tabId);
  chrome.debugger.detach({ tabId }).catch(() => {});
});

// ========== Sidebar injection ==========

chrome.action.onClicked.addListener(async (tab) => {
  if (!tab.id) return;

  try {
    // Inject page-world script first (React fiber access).
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["pageworld.js"],
      world: "MAIN",
    });

    // Then inject the content script (sidebar UI, picker).
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["content.js"],
    });
  } catch (err) {
    console.error("[EPI] Failed to inject scripts:", err);
  }
});
