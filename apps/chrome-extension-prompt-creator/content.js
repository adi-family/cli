// ADI Prompt Creator - Content Script (ISOLATED world)
// Sidebar UI, element picker, HTML path extraction, prompt generation via Anthropic API.
// React data is fetched from pageworld.js running in the MAIN world via CustomEvents.

(() => {
  const SIDEBAR_ID = "__epi-sidebar-host";
  const SIDEBAR_WIDTH = 400;

  // Toggle
  const existing = document.getElementById(SIDEBAR_ID);
  if (existing) {
    existing.remove();
    document.body.style.marginRight = "";
    return;
  }

  // ========== ADI Design Tokens (indigo dark) ==========
  const T = {
    bg: "#0a0a0a",
    surface: "#0e0e0e",
    surfaceAlt: "#141414",
    text: "#e0e0e0",
    textMuted: "#a0a0a0",
    border: "rgba(255,255,255,0.07)",
    accent: "#875fd7",
    accentSoft: "rgba(135,95,215,0.12)",
    success: "#22cc00",
    successSoft: "rgba(34,204,0,0.12)",
    error: "#ff0000",
    errorSoft: "rgba(255,0,0,0.12)",
    warning: "#ffaa00",
    warningSoft: "rgba(255,170,0,0.12)",
    fontHeading: "'Space Grotesk', sans-serif",
    fontBody: "'Inter', sans-serif",
    fontMono: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
    radius: "8px",
    radiusSm: "4px",
    radiusLg: "12px",
  };

  // ========== State ==========
  let isPicking = false;
  let overlay = null;
  let hoverTooltip = null;
  let copyData = {};
  let requestCounter = 0;
  let lastResult = null;

  // ========== Page World Bridge ==========
  function queryPageWorld(type, element) {
    return new Promise((resolve) => {
      const requestId = "__epi_" + (++requestCounter);
      element.setAttribute("data-epi-target", "1");
      const onResponse = (e) => {
        if (e.detail && e.detail.requestId === requestId) {
          document.removeEventListener("__epi_response", onResponse);
          resolve(e.detail.result);
        }
      };
      document.addEventListener("__epi_response", onResponse);
      document.dispatchEvent(new CustomEvent("__epi_query", { detail: { requestId, type } }));
      setTimeout(() => {
        document.removeEventListener("__epi_response", onResponse);
        element.removeAttribute("data-epi-target");
        resolve(null);
      }, 500);
    });
  }

  // ========== HTML Path Extraction ==========
  function getHtmlPath(element) {
    const path = [];
    let current = element;
    while (current && current !== document.documentElement.parentNode) {
      let selector = current.tagName?.toLowerCase();
      if (!selector) break;
      if (current.id) {
        selector += `#${current.id}`;
      } else {
        const classes = Array.from(current.classList || [])
          .filter((c) => !c.match(/^[a-zA-Z]+_[a-zA-Z0-9]{5,}/) && !c.match(/^css-/))
          .slice(0, 2);
        if (classes.length > 0) selector += "." + classes.join(".");
        const parent = current.parentElement;
        if (parent) {
          const siblings = Array.from(parent.children).filter((s) => s.tagName === current.tagName);
          if (siblings.length > 1) {
            selector += `:nth-child(${siblings.indexOf(current) + 1})`;
          }
        }
      }
      path.unshift(selector);
      if (current.tagName?.toLowerCase() === "body") break;
      current = current.parentElement;
    }
    return path;
  }

  // ========== Picker Overlay ==========
  function createPickerOverlay() {
    overlay = document.createElement("div");
    overlay.id = "__epi-overlay";
    overlay.style.cssText =
      `position:fixed;pointer-events:none;z-index:2147483645;border:2px solid ${T.accent};background:${T.accentSoft};transition:all 0.05s ease-out;display:none;`;
    hoverTooltip = document.createElement("div");
    hoverTooltip.id = "__epi-tooltip";
    hoverTooltip.style.cssText =
      `position:fixed;z-index:2147483645;pointer-events:none;background:${T.bg};color:${T.text};font-family:${T.fontMono};font-size:11px;padding:4px 8px;border-radius:${T.radiusSm};border:1px solid ${T.border};max-width:500px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;display:none;`;
    document.documentElement.appendChild(overlay);
    document.documentElement.appendChild(hoverTooltip);
  }

  async function highlightElement(el) {
    if (!overlay) return;
    const rect = el.getBoundingClientRect();
    overlay.style.display = "block";
    overlay.style.top = rect.top + "px";
    overlay.style.left = rect.left + "px";
    overlay.style.width = rect.width + "px";
    overlay.style.height = rect.height + "px";
    const tag = el.tagName.toLowerCase();
    const reactName = await queryPageWorld("getReactName", el);
    hoverTooltip.textContent = reactName ? `<${tag}> in <${reactName}>` : `<${tag}>`;
    hoverTooltip.style.display = "block";
    let tooltipTop = rect.top - 24;
    if (tooltipTop < 4) tooltipTop = rect.bottom + 4;
    hoverTooltip.style.top = tooltipTop + "px";
    hoverTooltip.style.left = Math.max(4, rect.left) + "px";
  }

  function removePickerOverlay() {
    overlay?.remove();
    hoverTooltip?.remove();
    overlay = null;
    hoverTooltip = null;
  }

  // ========== Picker Events ==========
  function isInsideSidebar(el) {
    let node = el;
    while (node) {
      if (node.id === SIDEBAR_ID) return true;
      node = node.parentNode || node.host;
    }
    return false;
  }

  let hoverTimer = null;
  function onMouseMove(e) {
    if (!isPicking || isInsideSidebar(e.target)) return;
    if (overlay) {
      const rect = e.target.getBoundingClientRect();
      overlay.style.display = "block";
      overlay.style.top = rect.top + "px";
      overlay.style.left = rect.left + "px";
      overlay.style.width = rect.width + "px";
      overlay.style.height = rect.height + "px";
    }
    clearTimeout(hoverTimer);
    hoverTimer = setTimeout(() => highlightElement(e.target), 30);
  }

  async function onClick(e) {
    if (!isPicking || isInsideSidebar(e.target)) return;
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();
    const el = e.target;
    const htmlPath = getHtmlPath(el);
    const reactComponents = await queryPageWorld("getReactData", el);
    let snippet = el.outerHTML;
    if (snippet.length > 300) snippet = snippet.substring(0, 300) + "...";

    lastResult = {
      htmlPath: htmlPath.join(" > "),
      htmlTag: el.tagName.toLowerCase(),
      snippet,
      reactComponents,
      reactComponentChain: reactComponents ? reactComponents.map((c) => c.name).join(" > ") : null,
      textContent: (el.textContent || "").trim().substring(0, 100),
      pageUrl: window.location.href,
      pageTitle: document.title,
    };

    stopPicking();
    renderResult(lastResult);
  }

  function onKeyDown(e) {
    if (e.key === "Escape" && isPicking) stopPicking();
  }

  function startPicking() {
    if (isPicking) return;
    isPicking = true;
    createPickerOverlay();
    document.addEventListener("mousemove", onMouseMove, true);
    document.addEventListener("click", onClick, true);
    document.addEventListener("keydown", onKeyDown, true);
    document.body.style.cursor = "crosshair";
    updatePickBtn(true);
  }

  function stopPicking() {
    isPicking = false;
    clearTimeout(hoverTimer);
    document.removeEventListener("mousemove", onMouseMove, true);
    document.removeEventListener("click", onClick, true);
    document.removeEventListener("keydown", onKeyDown, true);
    document.body.style.cursor = "";
    removePickerOverlay();
    updatePickBtn(false);
  }

  // ========== Sidebar (Shadow DOM) ==========
  const host = document.createElement("div");
  host.id = SIDEBAR_ID;
  host.style.cssText = `position:fixed;top:0;right:0;width:${SIDEBAR_WIDTH}px;height:100vh;z-index:2147483647;`;

  const shadow = host.attachShadow({ mode: "closed" });

  shadow.innerHTML = `
    <style>
      :host { all: initial; }
      * { box-sizing: border-box; margin: 0; padding: 0; }

      .sidebar {
        width: ${SIDEBAR_WIDTH}px; height: 100vh;
        background: ${T.bg};
        font-family: ${T.fontBody};
        font-size: 13px; color: ${T.text};
        display: flex; flex-direction: column;
        border-left: 1px solid ${T.border};
        box-shadow: -2px 0 12px rgba(0,0,0,0.3);
        overflow: hidden;
      }

      /* Header */
      .header {
        padding: 10px 14px;
        background: ${T.surface};
        border-bottom: 1px solid ${T.border};
        display: flex; align-items: center; justify-content: space-between;
        flex-shrink: 0;
      }
      .header h1 {
        font-family: ${T.fontHeading}; font-size: 13px; font-weight: 600;
        color: ${T.accent};
      }
      .header-actions { display: flex; gap: 6px; align-items: center; }

      .pick-btn {
        padding: 5px 12px;
        background: ${T.accent}; color: #fff;
        border: none; border-radius: ${T.radiusSm};
        font-family: ${T.fontBody}; font-size: 11px; font-weight: 600;
        cursor: pointer;
      }
      .pick-btn:hover { filter: brightness(1.15); }
      .pick-btn.active { background: ${T.warning}; color: ${T.bg}; }

      .close-btn {
        width: 24px; height: 24px;
        background: ${T.surfaceAlt}; color: ${T.textMuted};
        border: 1px solid ${T.border}; border-radius: ${T.radiusSm};
        font-size: 14px; cursor: pointer;
        display: flex; align-items: center; justify-content: center;
      }
      .close-btn:hover { color: ${T.text}; background: rgba(255,255,255,0.07); }

      /* Body */
      .body { flex: 1; overflow-y: auto; padding: 12px 14px; }
      .body::-webkit-scrollbar { width: 4px; }
      .body::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.07); border-radius: 2px; }
      .body::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.14); }

      /* Empty */
      .empty-state { padding: 32px 8px; text-align: center; color: ${T.textMuted}; }
      .empty-state p { font-size: 12px; line-height: 1.5; }
      .empty-state .hint { margin-top: 6px; font-size: 11px; color: ${T.textMuted}; opacity: 0.6; }

      /* Sections */
      .section { margin-bottom: 14px; }

      .section-title {
        font-family: ${T.fontHeading}; font-size: 10px;
        text-transform: uppercase; letter-spacing: 0.5px;
        color: ${T.textMuted}; margin-bottom: 6px;
        display: flex; align-items: center; justify-content: space-between;
      }

      .copy-btn {
        font-size: 10px; color: ${T.accent}; background: none; border: none;
        cursor: pointer; padding: 2px 4px; font-family: ${T.fontBody};
      }
      .copy-btn:hover { text-decoration: underline; }
      .copy-btn.copied { color: ${T.success}; }

      /* Details / collapsible */
      details {
        background: ${T.surface};
        border: 1px solid ${T.border};
        border-radius: ${T.radius};
        overflow: hidden;
        margin-bottom: 8px;
      }
      details > summary {
        padding: 8px 12px;
        cursor: pointer;
        font-family: ${T.fontHeading}; font-size: 11px; font-weight: 600;
        color: ${T.text};
        list-style: none;
        display: flex; align-items: center; justify-content: space-between;
        user-select: none;
      }
      details > summary::-webkit-details-marker { display: none; }
      details > summary::before {
        content: "\\25B6"; font-size: 8px; color: ${T.textMuted};
        margin-right: 8px; transition: transform 0.15s;
        display: inline-block;
      }
      details[open] > summary::before { transform: rotate(90deg); }
      details > summary:hover { background: ${T.surfaceAlt}; }
      details > .details-body { padding: 8px 12px; border-top: 1px solid ${T.border}; }

      /* Code boxes */
      .code-box {
        background: ${T.surfaceAlt}; color: ${T.text};
        font-family: ${T.fontMono}; font-size: 11px;
        padding: 8px 10px; border-radius: ${T.radiusSm};
        line-height: 1.6; word-break: break-all; white-space: pre-wrap;
        max-height: 120px; overflow-y: auto;
        border: 1px solid ${T.border};
      }
      .code-box .sep { color: ${T.accent}; }
      .code-box .comp { color: #a78bfa; }
      .code-box .src { color: ${T.textMuted}; font-size: 10px; }
      .code-box .tag { color: ${T.success}; }
      .code-box .cls { color: ${T.warning}; }
      .code-box .eid { color: #f472b6; }

      .snippet-box {
        background: ${T.surfaceAlt}; color: ${T.text};
        font-family: ${T.fontMono}; font-size: 11px;
        padding: 8px 10px; border-radius: ${T.radiusSm};
        line-height: 1.4; word-break: break-all; white-space: pre-wrap;
        max-height: 80px; overflow-y: auto;
        border: 1px solid ${T.border};
      }

      /* Element tag pill */
      .element-tag {
        display: inline-block; background: ${T.accentSoft}; color: ${T.accent};
        padding: 2px 8px; border-radius: ${T.radiusSm};
        font-family: ${T.fontMono}; font-size: 12px; font-weight: 600;
        margin-bottom: 4px; border: 1px solid ${T.border};
      }
      .text-preview {
        font-size: 11px; color: ${T.textMuted}; padding: 2px 0; font-style: italic;
      }

      /* React tree cards */
      .comp-tree { display: flex; flex-direction: column; gap: 2px; }
      .comp-card { background: ${T.surfaceAlt}; border: 1px solid ${T.border}; border-radius: 6px; overflow: hidden; }
      .comp-card-header {
        display: flex; align-items: center; gap: 6px; padding: 6px 10px;
        cursor: pointer; user-select: none;
        border-bottom: 1px solid transparent;
      }
      .comp-card-header:hover { background: rgba(255,255,255,0.03); }
      .comp-card.open .comp-card-header { border-bottom-color: ${T.border}; }
      .comp-arrow { font-size: 9px; color: ${T.textMuted}; transition: transform 0.15s; flex-shrink: 0; width: 10px; text-align: center; }
      .comp-card.open .comp-arrow { transform: rotate(90deg); }
      .comp-name { font-family: ${T.fontMono}; font-size: 12px; font-weight: 600; color: ${T.accent}; }
      .comp-source { font-family: ${T.fontMono}; font-size: 9px; color: ${T.textMuted}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; min-width: 0; text-align: right; }
      .comp-card-body { display: none; padding: 6px 10px 8px; font-size: 11px; }
      .comp-card.open .comp-card-body { display: block; }
      .comp-sub-title { font-size: 9px; text-transform: uppercase; letter-spacing: 0.4px; color: ${T.textMuted}; margin: 6px 0 3px; }
      .comp-sub-title:first-child { margin-top: 0; }
      .prop-list { font-family: ${T.fontMono}; font-size: 10px; line-height: 1.6; }
      .prop-key { color: #a78bfa; }
      .prop-val { color: ${T.text}; }
      .prop-eq { color: ${T.textMuted}; }
      .hook-item { font-family: ${T.fontMono}; font-size: 10px; line-height: 1.6; }
      .hook-type { color: #1ABC9C; }
      .hook-idx { color: ${T.textMuted}; }
      .hook-val { color: ${T.text}; }
      .comp-depth { display: inline-block; width: 16px; height: 16px; border-radius: 50%; background: ${T.accentSoft}; color: ${T.accent}; font-size: 9px; font-weight: 600; text-align: center; line-height: 16px; flex-shrink: 0; }
      .no-data { color: ${T.textMuted}; font-style: italic; font-size: 11px; padding: 6px 0; }

      /* ===== Prompt Section ===== */
      .prompt-section {
        margin-bottom: 14px;
        background: ${T.surface};
        border: 1px solid ${T.border};
        border-radius: ${T.radius};
        padding: 12px;
      }

      .prompt-section label {
        display: block;
        font-family: ${T.fontHeading}; font-size: 10px;
        text-transform: uppercase; letter-spacing: 0.5px;
        color: ${T.textMuted}; margin-bottom: 6px;
      }

      .prompt-section textarea, .prompt-section input[type="password"] {
        width: 100%; padding: 8px 10px;
        background: ${T.surfaceAlt}; color: ${T.text};
        font-family: ${T.fontBody}; font-size: 12px;
        border: 1px solid ${T.border}; border-radius: ${T.radiusSm};
        outline: none; resize: vertical;
      }
      .prompt-section textarea:focus, .prompt-section input[type="password"]:focus {
        border-color: ${T.accent};
        box-shadow: 0 0 0 3px rgba(135,95,215,0.15);
      }
      .prompt-section textarea { min-height: 80px; max-height: 200px; }
      .prompt-section input[type="password"] { height: 34px; }

      .prompt-section .field + .field { margin-top: 10px; }

      .generate-btn {
        width: 100%; padding: 8px 16px; margin-top: 12px;
        background: ${T.accent}; color: #fff;
        border: none; border-radius: ${T.radiusSm};
        font-family: ${T.fontHeading}; font-size: 12px; font-weight: 600;
        cursor: pointer;
        box-shadow: 0 2px 8px rgba(135,95,215,0.3);
      }
      .generate-btn:hover { filter: brightness(1.15); box-shadow: 0 4px 20px rgba(135,95,215,0.5); }
      .generate-btn:disabled { opacity: 0.5; cursor: default; filter: none; box-shadow: none; }

      .output-box {
        margin-top: 12px;
        background: ${T.surfaceAlt}; color: ${T.text};
        font-family: ${T.fontMono}; font-size: 11px;
        padding: 10px; border-radius: ${T.radiusSm};
        line-height: 1.6; white-space: pre-wrap; word-break: break-word;
        max-height: 300px; overflow-y: auto;
        border: 1px solid ${T.border};
        display: none;
      }
      .output-box.visible { display: block; }

      .output-header {
        display: none; align-items: center; justify-content: space-between;
        margin-top: 10px;
      }
      .output-header.visible { display: flex; }

      .error-msg {
        margin-top: 8px; padding: 8px 10px;
        background: ${T.errorSoft}; color: ${T.error};
        border-radius: ${T.radiusSm}; font-size: 11px;
        display: none; border: 1px solid rgba(255,0,0,0.15);
      }
      .error-msg.visible { display: block; }

      .result { display: none; }
      .result.visible { display: block; }

      .divider {
        height: 1px; background: ${T.border}; margin: 14px 0;
      }
    </style>

    <div class="sidebar">
      <div class="header">
        <h1>ADI Prompt Creator</h1>
        <div class="header-actions">
          <button class="pick-btn" id="pickBtn">Select</button>
          <button class="close-btn" id="closeBtn">&times;</button>
        </div>
      </div>
      <div class="body">

        <!-- Prompt Input Section (always visible) -->
        <div class="prompt-section">
          <div class="field">
            <label for="taskInput">What do you want to change?</label>
            <textarea id="taskInput" placeholder="Describe the change you want to make to this element..."></textarea>
          </div>
          <div class="field">
            <label for="apiKeyInput">Anthropic API Key</label>
            <input type="password" id="apiKeyInput" placeholder="sk-ant-..." />
          </div>
          <button class="generate-btn" id="generateBtn" disabled>Generate Prompt</button>
          <div id="errorMsg" class="error-msg"></div>
          <div id="outputHeader" class="output-header">
            <span class="section-title" style="margin:0">Generated Prompt</span>
            <button class="copy-btn" data-copy="generatedPrompt">copy</button>
          </div>
          <div id="outputBox" class="output-box"></div>
        </div>

        <!-- Empty state -->
        <div id="emptyState" class="empty-state">
          <p>Click "Select" to pick an element,<br>then describe your change above.</p>
          <p class="hint">Press Escape to cancel selection.</p>
        </div>

        <!-- Inspection results (in details) -->
        <div id="result" class="result">

          <div class="section">
            <span id="elementTag" class="element-tag"></span>
            <div id="textPreview" class="text-preview"></div>
          </div>

          <details id="reactDetails">
            <summary>React Component Path <button class="copy-btn" data-copy="reactPath" style="margin-left:auto">copy</button></summary>
            <div class="details-body">
              <div id="reactPath" class="code-box"></div>
            </div>
          </details>

          <details id="reactTreeDetails">
            <summary>React Components <button class="copy-btn" data-copy="reactTree" style="margin-left:auto">copy</button></summary>
            <div class="details-body">
              <div id="reactTree" class="comp-tree"></div>
            </div>
          </details>

          <details id="htmlDetails">
            <summary>HTML Path <button class="copy-btn" data-copy="htmlPath" style="margin-left:auto">copy</button></summary>
            <div class="details-body">
              <div id="htmlPath" class="code-box"></div>
            </div>
          </details>

          <details id="snippetDetails">
            <summary>HTML Snippet <button class="copy-btn" data-copy="snippet" style="margin-left:auto">copy</button></summary>
            <div class="details-body">
              <div id="snippet" class="snippet-box"></div>
            </div>
          </details>
        </div>
      </div>
    </div>
  `;

  // ========== DOM refs ==========
  const $ = (sel) => shadow.querySelector(sel);
  const pickBtn = $("#pickBtn");
  const closeBtn = $("#closeBtn");
  const emptyState = $("#emptyState");
  const resultEl = $("#result");
  const taskInput = $("#taskInput");
  const apiKeyInput = $("#apiKeyInput");
  const generateBtn = $("#generateBtn");
  const errorMsg = $("#errorMsg");
  const outputHeader = $("#outputHeader");
  const outputBox = $("#outputBox");

  // Load saved API key
  chrome.storage?.local?.get(["anthropicApiKey"], (stored) => {
    if (stored?.anthropicApiKey) apiKeyInput.value = stored.anthropicApiKey;
    updateGenerateBtn();
  });

  // Save API key on change
  apiKeyInput.addEventListener("input", () => {
    chrome.storage?.local?.set({ anthropicApiKey: apiKeyInput.value.trim() });
    updateGenerateBtn();
  });

  taskInput.addEventListener("input", updateGenerateBtn);

  function updateGenerateBtn() {
    generateBtn.disabled = !taskInput.value.trim() || !apiKeyInput.value.trim() || !lastResult;
  }

  function updatePickBtn(active) {
    if (!pickBtn) return;
    pickBtn.textContent = active ? "Picking..." : "Select";
    pickBtn.classList.toggle("active", active);
  }

  pickBtn.addEventListener("click", () => { isPicking ? stopPicking() : startPicking(); });
  closeBtn.addEventListener("click", () => { stopPicking(); host.remove(); document.body.style.marginRight = ""; });

  // Copy buttons
  function bindCopyBtn(btn) {
    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const key = btn.dataset.copy;
      const text = copyData[key];
      if (!text) return;
      navigator.clipboard.writeText(text).then(() => {
        btn.textContent = "copied!";
        btn.classList.add("copied");
        setTimeout(() => { btn.textContent = "copy"; btn.classList.remove("copied"); }, 1500);
      });
    });
  }
  shadow.querySelectorAll(".copy-btn").forEach(bindCopyBtn);

  // ========== Generate Prompt ==========
  generateBtn.addEventListener("click", async () => {
    if (!lastResult || !taskInput.value.trim() || !apiKeyInput.value.trim()) return;

    generateBtn.disabled = true;
    generateBtn.textContent = "Generating...";
    errorMsg.classList.remove("visible");
    outputHeader.classList.remove("visible");
    outputBox.classList.remove("visible");

    try {
      const contextParts = [];
      contextParts.push(`Page: ${lastResult.pageTitle} (${lastResult.pageUrl})`);
      contextParts.push(`Selected element: <${lastResult.htmlTag}>`);
      if (lastResult.textContent) contextParts.push(`Element text: "${lastResult.textContent}"`);
      contextParts.push(`HTML path: ${lastResult.htmlPath}`);
      if (lastResult.reactComponentChain) contextParts.push(`React component path: ${lastResult.reactComponentChain}`);
      if (lastResult.snippet) contextParts.push(`HTML snippet:\n${lastResult.snippet}`);

      if (lastResult.reactComponents && lastResult.reactComponents.length > 0) {
        const tree = lastResult.reactComponents.map((c, i) => {
          let line = `  ${i + 1}. ${c.name}`;
          if (c.source) line += ` (${c.source})`;
          if (c.props) line += "\n     Props: " + Object.entries(c.props).map(([k, v]) => `${k}=${v}`).join(", ");
          if (c.state) line += "\n     State: " + Object.entries(c.state).map(([k, v]) => `${k}=${v}`).join(", ");
          if (c.hooks) line += "\n     Hooks: " + c.hooks.map((h) => `${h.type}[${h.index}]=${h.value}`).join(", ");
          return line;
        }).join("\n");
        contextParts.push(`React component tree:\n${tree}`);
      }

      const systemPrompt = `You are a prompt engineer. You generate concise, actionable prompts for a coding agent (like Claude Code or Cursor) that needs to modify a specific UI element in a web application.

Your output must be a single prompt that the user can paste directly into a coding agent. The prompt should:
1. Clearly state what change to make
2. Include the exact file path / React component to find (based on the component path and source info)
3. Include the HTML structure context so the agent can quickly locate the element
4. Be direct and specific - no preamble, no explanation of what a prompt is
5. Use the React component hierarchy to identify which file(s) to edit
6. Mention the relevant props/state if they relate to the change

Output ONLY the prompt text, nothing else.`;

      const userMessage = `The user wants to make this change:
"${taskInput.value.trim()}"

Here is the context about the selected element:
${contextParts.join("\n\n")}

Generate a prompt for a coding agent to make this change.`;

      const response = await fetch("https://api.anthropic.com/v1/messages", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": apiKeyInput.value.trim(),
          "anthropic-version": "2023-06-01",
          "anthropic-dangerous-direct-browser-access": "true",
        },
        body: JSON.stringify({
          model: "claude-haiku-4-5",
          max_tokens: 1024,
          system: systemPrompt,
          messages: [{ role: "user", content: userMessage }],
        }),
      });

      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(err.error?.message || `API error ${response.status}`);
      }

      const data = await response.json();
      const prompt = data.content?.[0]?.text || "No response generated.";

      copyData.generatedPrompt = prompt;
      outputBox.textContent = prompt;
      outputBox.classList.add("visible");
      outputHeader.classList.add("visible");
    } catch (err) {
      errorMsg.textContent = err.message || "Failed to generate prompt.";
      errorMsg.classList.add("visible");
    } finally {
      generateBtn.disabled = false;
      generateBtn.textContent = "Generate Prompt";
      updateGenerateBtn();
    }
  });

  // ========== Render Result ==========
  function escapeHtml(text) {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  function renderPropsHtml(props) {
    return Object.entries(props).map(([k, v]) =>
      `<div class="prop-list"><span class="prop-key">${escapeHtml(k)}</span><span class="prop-eq"> = </span><span class="prop-val">${escapeHtml(v)}</span></div>`
    ).join("");
  }

  function renderHooksHtml(hooks) {
    return hooks.map((h) =>
      `<div class="hook-item"><span class="hook-type">${escapeHtml(h.type)}</span><span class="hook-idx">[${h.index}]</span> <span class="hook-val">${escapeHtml(h.value)}</span></div>`
    ).join("");
  }

  function renderStateHtml(state) {
    return Object.entries(state).map(([k, v]) =>
      `<div class="prop-list"><span class="prop-key">${escapeHtml(k)}</span><span class="prop-eq"> = </span><span class="prop-val">${escapeHtml(v)}</span></div>`
    ).join("");
  }

  function buildComponentCard(comp, index, total) {
    const hasDetails = comp.props || comp.state || comp.hooks;
    let bodyHtml = "";
    if (comp.props) bodyHtml += `<div class="comp-sub-title">Props</div>${renderPropsHtml(comp.props)}`;
    if (comp.state) bodyHtml += `<div class="comp-sub-title">State</div>${renderStateHtml(comp.state)}`;
    if (comp.hooks) bodyHtml += `<div class="comp-sub-title">Hooks</div>${renderHooksHtml(comp.hooks)}`;
    if (!hasDetails) bodyHtml = `<div class="no-data">No props, state, or hooks detected</div>`;

    const sourceHtml = comp.source ? `<span class="comp-source" title="${escapeHtml(comp.source)}">${escapeHtml(comp.source)}</span>` : "";
    const card = document.createElement("div");
    card.className = "comp-card";
    if (index === total - 1) card.classList.add("open");
    card.innerHTML = `
      <div class="comp-card-header">
        <span class="comp-arrow">&#9654;</span>
        <span class="comp-depth">${index + 1}</span>
        <span class="comp-name">${escapeHtml(comp.name)}</span>
        ${sourceHtml}
      </div>
      <div class="comp-card-body">${bodyHtml}</div>
    `;
    card.querySelector(".comp-card-header").addEventListener("click", () => { card.classList.toggle("open"); });
    return card;
  }

  function buildReactTreeText(components) {
    return components.map((c, i) => {
      let text = `${i + 1}. ${c.name}`;
      if (c.source) text += `  (${c.source})`;
      if (c.props) text += "\n   Props: " + Object.entries(c.props).map(([k, v]) => `${k}=${v}`).join(", ");
      if (c.state) text += "\n   State: " + Object.entries(c.state).map(([k, v]) => `${k}=${v}`).join(", ");
      if (c.hooks) text += "\n   Hooks: " + c.hooks.map((h) => `${h.type}[${h.index}]=${h.value}`).join(", ");
      return text;
    }).join("\n");
  }

  function renderResult(data) {
    emptyState.style.display = "none";
    resultEl.classList.add("visible");
    updateGenerateBtn();

    $("#elementTag").textContent = `<${data.htmlTag}>`;
    const tp = $("#textPreview");
    if (data.textContent) { tp.textContent = `"${data.textContent}"`; tp.style.display = "block"; }
    else tp.style.display = "none";

    const hasReact = data.reactComponents && data.reactComponents.length > 0;

    // React path
    const reactDetails = $("#reactDetails");
    if (hasReact) {
      reactDetails.style.display = "block";
      $("#reactPath").innerHTML = data.reactComponents.map((c, i) => {
        let h = `<span class="comp">${escapeHtml(c.name)}</span>`;
        if (c.source) h += ` <span class="src">(${escapeHtml(c.source)})</span>`;
        if (i < data.reactComponents.length - 1) h += ` <span class="sep"> &gt; </span>`;
        return h;
      }).join("");
      copyData.reactPath = data.reactComponentChain;
    } else {
      reactDetails.style.display = "none";
    }

    // React tree
    const reactTreeDetails = $("#reactTreeDetails");
    if (hasReact) {
      reactTreeDetails.style.display = "block";
      const treeEl = $("#reactTree");
      treeEl.innerHTML = "";
      data.reactComponents.forEach((comp, i) => {
        treeEl.appendChild(buildComponentCard(comp, i, data.reactComponents.length));
      });
      copyData.reactTree = buildReactTreeText(data.reactComponents);
    } else {
      reactTreeDetails.style.display = "none";
    }

    // HTML path
    const parts = data.htmlPath.split(" > ");
    $("#htmlPath").innerHTML = parts.map((part, i) => {
      const colored = part
        .replace(/#[\w-]+/g, '<span class="eid">$&</span>')
        .replace(/\.[\w-]+/g, '<span class="cls">$&</span>')
        .replace(/^[\w-]+/, '<span class="tag">$&</span>');
      return colored + (i < parts.length - 1 ? ' <span class="sep">&gt;</span> ' : "");
    }).join("");
    copyData.htmlPath = data.htmlPath;

    // Snippet
    $("#snippet").textContent = data.snippet || "";
    copyData.snippet = data.snippet || "";
  }

  // ========== Mount ==========
  document.documentElement.appendChild(host);
  document.body.style.marginRight = SIDEBAR_WIDTH + "px";
})();
