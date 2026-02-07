// ADI Browser Debugger - Background Service Worker
// Detects X-ADI-Debug-Token header and streams debug data to signaling server

const DEBUG_HEADER = "x-adi-debug-token";
const DEFAULT_SIGNALING_URL = "wss://adi.the-ihor.com/api/signaling/ws";
const RESPONSE_BODY_MAX_SIZE = 102400; // 100KB default

// State
let browserId = null;
let signalingUrl = DEFAULT_SIGNALING_URL;
let responseBodyMaxSize = RESPONSE_BODY_MAX_SIZE;
let ws = null;
let wsReconnectTimer = null;
let isConnecting = false;

// Active debug tabs: tabId -> { token, requestMap, consoleEntries }
const debugTabs = new Map();
// Pending requests for response body: requestId -> { tabId, timestamp }
const pendingRequests = new Map();

// Initialize on install
chrome.runtime.onInstalled.addListener(async (details) => {
  console.log("[ADI Debug] Extension installed:", details.reason);
  await initializeExtension();
});

// Initialize on startup
chrome.runtime.onStartup.addListener(async () => {
  console.log("[ADI Debug] Browser started");
  await initializeExtension();
});

// Initialize immediately
initializeExtension();

async function initializeExtension() {
  // Generate or load browser_id
  const stored = await chrome.storage.local.get([
    "browserId",
    "signalingUrl",
    "responseBodyMaxSize",
  ]);

  if (stored.browserId) {
    browserId = stored.browserId;
  } else {
    browserId = crypto.randomUUID();
    await chrome.storage.local.set({ browserId });
  }

  if (stored.signalingUrl) {
    signalingUrl = stored.signalingUrl;
  }

  if (stored.responseBodyMaxSize) {
    responseBodyMaxSize = stored.responseBodyMaxSize;
  }

  console.log("[ADI Debug] Initialized with browser_id:", browserId);
}

// ========== WebSocket Connection ==========

function connectWebSocket() {
  console.log("[ADI Debug] Attempting to connect WebSocket...");

  if (
    ws &&
    (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)
  ) {
    return;
  }

  if (isConnecting) {
    return;
  }

  isConnecting = true;
  console.log("[ADI Debug] Connecting to signaling server:", signalingUrl);

  try {
    ws = new WebSocket(signalingUrl);

    ws.onopen = () => {
      console.log("[ADI Debug] WebSocket connected");
      isConnecting = false;

      // Re-register all active tabs
      for (const [tabId, tabData] of debugTabs) {
        sendTabAvailable(tabData);
      }
    };

    ws.onclose = (event) => {
      console.log("[ADI Debug] WebSocket closed:", event.code, event.reason);
      isConnecting = false;
      ws = null;

      // Reconnect after delay if we have active tabs
      if (debugTabs.size > 0) {
        scheduleReconnect();
      }
    };

    ws.onerror = (error) => {
      console.error("[ADI Debug] WebSocket error:", error);
      isConnecting = false;
    };

    ws.onmessage = (event) => {
      handleSignalingMessage(event.data);
    };
  } catch (error) {
    console.error("[ADI Debug] Failed to connect WebSocket:", error);
    isConnecting = false;
    scheduleReconnect();
  }
}

function scheduleReconnect() {
  if (wsReconnectTimer) {
    clearTimeout(wsReconnectTimer);
  }

  wsReconnectTimer = setTimeout(() => {
    wsReconnectTimer = null;
    if (debugTabs.size > 0) {
      connectWebSocket();
    }
  }, 5000);
}

function sendMessage(message) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(message));
    return true;
  }
  return false;
}

function handleSignalingMessage(data) {
  try {
    const message = JSON.parse(data);

    switch (message.type) {
      case "browser_debug_get_network":
        handleGetNetwork(message);
        break;

      case "browser_debug_get_console":
        handleGetConsole(message);
        break;

      case "error":
        console.error("[ADI Debug] Server error:", message.message);
        break;

      default:
        // Ignore other message types
        break;
    }
  } catch (error) {
    console.error("[ADI Debug] Failed to parse message:", error);
  }
}

function handleGetNetwork(message) {
  const { request_id, token, filters } = message;

  // Find tab with this token
  let tabData = null;
  for (const [, data] of debugTabs) {
    if (data.token === token) {
      tabData = data;
      break;
    }
  }

  if (!tabData) {
    console.warn("[ADI Debug] Tab not found for token:", token);
    return;
  }

  // Convert request map to array
  let requests = Array.from(tabData.requestMap.values());

  // Apply filters
  if (filters) {
    if (filters.url_pattern) {
      try {
        const regex = new RegExp(filters.url_pattern);
        requests = requests.filter((r) => regex.test(r.url));
      } catch (e) {
        console.warn("[ADI Debug] Invalid URL pattern regex:", e);
      }
    }

    if (filters.method && filters.method.length > 0) {
      requests = requests.filter((r) => filters.method.includes(r.method));
    }

    if (filters.status_min !== undefined) {
      requests = requests.filter(
        (r) => r.status !== undefined && r.status >= filters.status_min,
      );
    }

    if (filters.status_max !== undefined) {
      requests = requests.filter(
        (r) => r.status !== undefined && r.status <= filters.status_max,
      );
    }

    if (filters.since !== undefined) {
      requests = requests.filter((r) => r.timestamp >= filters.since);
    }

    if (filters.limit !== undefined) {
      requests = requests.slice(0, filters.limit);
    }
  }

  sendMessage({
    type: "browser_debug_network_data",
    request_id,
    requests,
  });
}

function handleGetConsole(message) {
  const { request_id, token, filters } = message;

  // Find tab with this token
  let tabData = null;
  for (const [, data] of debugTabs) {
    if (data.token === token) {
      tabData = data;
      break;
    }
  }

  if (!tabData) {
    console.warn("[ADI Debug] Tab not found for token:", token);
    return;
  }

  let entries = [...tabData.consoleEntries];

  // Apply filters
  if (filters) {
    if (filters.level && filters.level.length > 0) {
      entries = entries.filter((e) => filters.level.includes(e.level));
    }

    if (filters.message_pattern) {
      try {
        const regex = new RegExp(filters.message_pattern);
        entries = entries.filter((e) => regex.test(e.message));
      } catch (e) {
        console.warn("[ADI Debug] Invalid message pattern regex:", e);
      }
    }

    if (filters.since !== undefined) {
      entries = entries.filter((e) => e.timestamp >= filters.since);
    }

    if (filters.limit !== undefined) {
      entries = entries.slice(0, filters.limit);
    }
  }

  sendMessage({
    type: "browser_debug_console_data",
    request_id,
    entries,
  });
}

// ========== Tab Available/Closed ==========

function sendTabAvailable(tabData) {
  sendMessage({
    type: "browser_debug_tab_available",
    token: tabData.token,
    browser_id: browserId,
    url: tabData.url,
    title: tabData.title,
    favicon: tabData.favicon,
  });
}

function sendTabClosed(token) {
  sendMessage({
    type: "browser_debug_tab_closed",
    token,
  });
}

function sendTabUpdated(token, url, title) {
  sendMessage({
    type: "browser_debug_tab_updated",
    token,
    url,
    title,
  });
}

// ========== Network Event Streaming ==========

function sendNetworkEvent(token, eventType, data) {
  sendMessage({
    type: "browser_debug_network_event",
    token,
    event: eventType,
    data,
  });
}

function sendConsoleEvent(token, entry) {
  sendMessage({
    type: "browser_debug_console_event",
    token,
    entry,
  });
}

// ========== Debug Token Detection ==========

// Listen for response headers to detect debug token
chrome.webRequest.onHeadersReceived.addListener(
  (details) => {
    // Only check main_frame (document) requests
    if (details.type !== "main_frame") {
      return;
    }

    // Check for debug token header
    const debugTokenHeader = details.responseHeaders?.find(
      (h) => h.name.toLowerCase() === DEBUG_HEADER,
    );

    if (debugTokenHeader && debugTokenHeader.value) {
      const token = debugTokenHeader.value;
      console.log(
        "[ADI Debug] Found debug token on tab",
        details.tabId,
        "URL:",
        details.url,
      );

      // Start debugging this tab
      startDebugging(details.tabId, token, details.url);
    }
  },
  { urls: ["<all_urls>"] },
  ["responseHeaders"],
);

// ========== Debugger Attachment ==========

async function startDebugging(tabId, token, url) {
  console.log(
    "[ADI Debug] Starting debugging for tab",
    tabId,
    "with token",
    token,
  );

  // If already debugging this tab with same token, skip
  if (debugTabs.has(tabId) && debugTabs.get(tabId).token === token) {
    return;
  }

  // If debugging with different token, clean up first
  if (debugTabs.has(tabId)) {
    await stopDebugging(tabId);
  }

  try {
    // Get tab info
    const tab = await chrome.tabs.get(tabId);

    // Attach debugger
    await chrome.debugger.attach({ tabId }, "1.3");
    console.log("[ADI Debug] Debugger attached to tab", tabId);

    // Enable Network domain
    await chrome.debugger.sendCommand({ tabId }, "Network.enable", {});

    // Enable Runtime domain for console
    await chrome.debugger.sendCommand({ tabId }, "Runtime.enable", {});

    // Enable Log domain for additional console messages
    await chrome.debugger.sendCommand({ tabId }, "Log.enable", {});

    // Store tab data
    const tabData = {
      token,
      tabId,
      url: tab.url || url,
      title: tab.title || "Unknown",
      favicon: tab.favIconUrl || null,
      requestMap: new Map(), // requestId -> NetworkRequest
      consoleEntries: [],
    };

    debugTabs.set(tabId, tabData);

    // Connect to signaling server if not connected
    connectWebSocket();

    // Send tab available message
    sendTabAvailable(tabData);

    console.log("[ADI Debug] Started debugging tab", tabId, "with token");
  } catch (error) {
    console.error(
      "[ADI Debug] Failed to attach debugger to tab",
      tabId,
      ":",
      error,
    );
  }
}

async function stopDebugging(tabId) {
  const tabData = debugTabs.get(tabId);
  if (!tabData) {
    return;
  }

  // Send tab closed message
  sendTabClosed(tabData.token);

  // Remove from tracking
  debugTabs.delete(tabId);

  // Detach debugger
  try {
    await chrome.debugger.detach({ tabId });
    console.log("[ADI Debug] Debugger detached from tab", tabId);
  } catch (error) {
    // Tab might already be closed
    console.log(
      "[ADI Debug] Could not detach debugger (tab may be closed):",
      error.message,
    );
  }

  // Close WebSocket if no more tabs
  if (debugTabs.size === 0 && ws) {
    ws.close();
    ws = null;
  }
}

// ========== Debugger Event Handling ==========

chrome.debugger.onEvent.addListener((source, method, params) => {
  const { tabId } = source;
  const tabData = debugTabs.get(tabId);

  if (!tabData) {
    return;
  }

  switch (method) {
    case "Network.requestWillBeSent":
      handleNetworkRequest(tabData, params);
      break;

    case "Network.responseReceived":
      handleNetworkResponse(tabData, params);
      break;

    case "Network.loadingFinished":
      handleNetworkFinished(tabData, tabId, params);
      break;

    case "Network.loadingFailed":
      handleNetworkFailed(tabData, params);
      break;

    case "Runtime.consoleAPICalled":
      handleConsoleCall(tabData, params);
      break;

    case "Runtime.exceptionThrown":
      handleException(tabData, params);
      break;

    case "Log.entryAdded":
      handleLogEntry(tabData, params);
      break;
  }
});

function handleNetworkRequest(tabData, params) {
  const { requestId, request, timestamp, frameId } = params;

  const networkRequest = {
    request_id: requestId,
    timestamp: Math.floor(timestamp * 1000),
    method: request.method,
    url: request.url,
    request_headers: request.headers || {},
    request_body: request.postData || null,
  };

  tabData.requestMap.set(requestId, networkRequest);

  // Stream event
  sendNetworkEvent(tabData.token, "request", {
    request_id: requestId,
    timestamp: networkRequest.timestamp,
    method: request.method,
    url: request.url,
    request_headers: request.headers,
    request_body: request.postData,
  });
}

function handleNetworkResponse(tabData, params) {
  const { requestId, response, timestamp } = params;

  const networkRequest = tabData.requestMap.get(requestId);
  if (networkRequest) {
    networkRequest.status = response.status;
    networkRequest.status_text = response.statusText;
    networkRequest.response_headers = response.headers || {};
    networkRequest.mime_type = response.mimeType;
  }

  // Stream event
  sendNetworkEvent(tabData.token, "response", {
    request_id: requestId,
    timestamp: Math.floor(timestamp * 1000),
    status: response.status,
    status_text: response.statusText,
    response_headers: response.headers,
    mime_type: response.mimeType,
  });
}

async function handleNetworkFinished(tabData, tabId, params) {
  const { requestId, timestamp, encodedDataLength } = params;

  const networkRequest = tabData.requestMap.get(requestId);
  if (!networkRequest) {
    return;
  }

  const startTimestamp = networkRequest.timestamp;
  const endTimestamp = Math.floor(timestamp * 1000);
  networkRequest.duration_ms = endTimestamp - startTimestamp;

  // Try to get response body (if small enough)
  let responseBody = null;
  let bodyTruncated = false;

  try {
    const result = await chrome.debugger.sendCommand(
      { tabId },
      "Network.getResponseBody",
      { requestId },
    );

    if (result) {
      let body = result.body;

      // Decode base64 if needed
      if (result.base64Encoded) {
        try {
          body = atob(result.body);
        } catch (e) {
          // Keep base64 if decode fails (binary data)
          body = `[base64] ${result.body.substring(0, 1000)}...`;
          bodyTruncated = true;
        }
      }

      // Truncate if too large
      if (body.length > responseBodyMaxSize) {
        body = body.substring(0, responseBodyMaxSize);
        bodyTruncated = true;
      }

      responseBody = body;
    }
  } catch (error) {
    // Body may not be available (streaming, cancelled, etc.)
  }

  networkRequest.response_body = responseBody;
  networkRequest.response_body_truncated = bodyTruncated;

  // Stream event
  sendNetworkEvent(tabData.token, "finished", {
    request_id: requestId,
    timestamp: endTimestamp,
    response_body: responseBody,
    response_body_truncated: bodyTruncated,
    duration_ms: networkRequest.duration_ms,
  });

  // Limit stored requests
  if (tabData.requestMap.size > 1000) {
    // Remove oldest entries
    const keysToRemove = Array.from(tabData.requestMap.keys()).slice(0, 100);
    keysToRemove.forEach((k) => tabData.requestMap.delete(k));
  }
}

function handleNetworkFailed(tabData, params) {
  const { requestId, timestamp, errorText } = params;

  const networkRequest = tabData.requestMap.get(requestId);
  if (networkRequest) {
    networkRequest.error = errorText;
  }

  // Stream event
  sendNetworkEvent(tabData.token, "failed", {
    request_id: requestId,
    timestamp: Math.floor(timestamp * 1000),
    error: errorText,
  });
}

function handleConsoleCall(tabData, params) {
  const { type, args, timestamp, executionContextId, stackTrace } = params;

  // Map console API type to level
  const levelMap = {
    log: "log",
    debug: "debug",
    info: "info",
    warning: "warn",
    error: "error",
    assert: "error",
    dir: "log",
    dirxml: "log",
    table: "log",
    trace: "log",
    group: "log",
    groupCollapsed: "log",
    groupEnd: "log",
    clear: "log",
    count: "log",
    countReset: "log",
    time: "log",
    timeEnd: "log",
    timeLog: "log",
    profile: "log",
    profileEnd: "log",
  };

  const level = levelMap[type] || "log";

  // Format arguments
  const formattedArgs = args.map((arg) => {
    if (arg.value !== undefined) return arg.value;
    if (arg.description) return arg.description;
    if (arg.type === "undefined") return undefined;
    if (arg.type === "null") return null;
    return `[${arg.type}]`;
  });

  const message = formattedArgs.map((a) => String(a)).join(" ");

  const entry = {
    timestamp: Math.floor(timestamp * 1000),
    level,
    message,
    args: formattedArgs,
    stack_trace: stackTrace ? formatStackTrace(stackTrace) : null,
  };

  tabData.consoleEntries.push(entry);

  // Limit stored entries
  if (tabData.consoleEntries.length > 1000) {
    tabData.consoleEntries.splice(0, 100);
  }

  // Stream event
  sendConsoleEvent(tabData.token, entry);
}

function handleException(tabData, params) {
  const { exceptionDetails, timestamp } = params;

  const { text, exception, lineNumber, columnNumber, url, stackTrace } =
    exceptionDetails;

  const message = exception?.description || text || "Unknown exception";

  const entry = {
    timestamp: Math.floor(timestamp * 1000),
    level: "error",
    message,
    args: [message],
    source: url,
    line: lineNumber,
    column: columnNumber,
    stack_trace: stackTrace ? formatStackTrace(stackTrace) : null,
  };

  tabData.consoleEntries.push(entry);

  // Stream event
  sendConsoleEvent(tabData.token, entry);
}

function handleLogEntry(tabData, params) {
  const { entry } = params;

  const levelMap = {
    verbose: "debug",
    info: "info",
    warning: "warn",
    error: "error",
  };

  const logEntry = {
    timestamp: Math.floor(entry.timestamp),
    level: levelMap[entry.level] || "log",
    message: entry.text,
    args: [entry.text],
    source: entry.url,
    line: entry.lineNumber,
  };

  tabData.consoleEntries.push(logEntry);

  // Stream event
  sendConsoleEvent(tabData.token, logEntry);
}

function formatStackTrace(stackTrace) {
  if (!stackTrace || !stackTrace.callFrames) {
    return null;
  }

  return stackTrace.callFrames
    .map((frame) => {
      const fn = frame.functionName || "(anonymous)";
      const url = frame.url || "";
      const line = frame.lineNumber || 0;
      const col = frame.columnNumber || 0;
      return `    at ${fn} (${url}:${line}:${col})`;
    })
    .join("\n");
}

// ========== Tab Lifecycle ==========

// Handle tab updates (navigation)
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  const tabData = debugTabs.get(tabId);

  if (tabData) {
    // Update stored info
    if (changeInfo.url) {
      tabData.url = changeInfo.url;
    }
    if (changeInfo.title) {
      tabData.title = changeInfo.title;
    }
    if (changeInfo.favIconUrl) {
      tabData.favicon = changeInfo.favIconUrl;
    }

    // Send update if URL or title changed
    if (changeInfo.url || changeInfo.title) {
      sendTabUpdated(tabData.token, tabData.url, tabData.title);
    }
  }
});

// Handle tab close
chrome.tabs.onRemoved.addListener((tabId) => {
  if (debugTabs.has(tabId)) {
    stopDebugging(tabId);
  }
});

// Handle debugger detach (user clicked "Cancel" on debugger bar)
chrome.debugger.onDetach.addListener((source, reason) => {
  const { tabId } = source;

  if (debugTabs.has(tabId)) {
    const tabData = debugTabs.get(tabId);
    console.log(
      "[ADI Debug] Debugger detached from tab",
      tabId,
      "- reason:",
      reason,
    );

    // Send tab closed message
    sendTabClosed(tabData.token);

    // Remove from tracking
    debugTabs.delete(tabId);

    // Close WebSocket if no more tabs
    if (debugTabs.size === 0 && ws) {
      ws.close();
      ws = null;
    }
  }
});

// ========== Popup Communication ==========

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  console.log("[ADI Debug] Received message from popup:", message);

  switch (message.type) {
    case "getStatus":
      sendResponse({
        browserId,
        connected: ws && ws.readyState === WebSocket.OPEN,
        signalingUrl,
        activeTabs: debugTabs.size,
        tabs: Array.from(debugTabs.values()).map((t) => ({
          tabId: t.tabId,
          url: t.url,
          title: t.title,
          requestCount: t.requestMap.size,
          consoleCount: t.consoleEntries.length,
        })),
      });
      return true;

    case "updateSettings":
      if (message.signalingUrl) {
        signalingUrl = message.signalingUrl;
        chrome.storage.local.set({ signalingUrl });

        // Reconnect with new URL
        if (ws) {
          ws.close();
        }
        if (debugTabs.size > 0) {
          connectWebSocket();
        }
      }
      if (message.responseBodyMaxSize) {
        responseBodyMaxSize = message.responseBodyMaxSize;
        chrome.storage.local.set({ responseBodyMaxSize });
      }
      sendResponse({ success: true });
      return true;

    case "stopDebugging":
      if (message.tabId && debugTabs.has(message.tabId)) {
        stopDebugging(message.tabId);
        sendResponse({ success: true });
      } else {
        sendResponse({ success: false, error: "Tab not found" });
      }
      return true;
  }
});

console.log("[ADI Debug] Background service worker initialized");
