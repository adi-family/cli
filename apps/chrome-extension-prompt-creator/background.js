// Element Path Inspector - Background Service Worker
// Clicking the extension icon injects scripts and toggles the sidebar.
// Two scripts are injected:
//   1. pageworld.js in MAIN world - accesses React fiber internals
//   2. content.js in ISOLATED world - sidebar UI, picker, DOM path extraction

chrome.action.onClicked.addListener(async (tab) => {
  if (!tab.id) return;

  try {
    // Inject page-world script first (React fiber access).
    // This needs to run in the page's JS context to see __reactFiber$ keys.
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
