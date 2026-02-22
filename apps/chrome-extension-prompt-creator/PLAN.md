# Element Path Inspector - Chrome Extension

## Purpose

Select any element on a web page to see:
- **React component path** - walks React fiber tree to show the component hierarchy (e.g. `App > Layout > Sidebar > NavItem`)
- **HTML DOM path** - CSS-selector-style path from body to the element (e.g. `body > div#root > nav.sidebar > a.nav-item`)
- **HTML snippet** - the element's outer HTML (truncated)

## How It Works

1. Click the extension icon in the toolbar - a sidebar opens on the right side of the page
2. Click "Select" in the sidebar header to activate element picking
3. Hovering highlights elements with an overlay showing tag + nearest React component
4. Click an element to capture its paths - results render in the sidebar
5. Copy any path with the "copy" buttons
6. Press Escape to cancel selection, click X to close the sidebar
7. Click the extension icon again to toggle the sidebar off

## Files

| File | Purpose |
|------|---------|
| `manifest.json` | Extension metadata, permissions (activeTab, scripting) |
| `background.js` | Service worker - injects content script on icon click |
| `content.js` | Sidebar UI (shadow DOM), element picker, React fiber traversal, HTML path extraction |

## Architecture

- **No popup** - the sidebar is injected directly into the page DOM
- **Shadow DOM** - sidebar styles are fully isolated from the host page
- **Toggle** - re-injecting the script removes the sidebar (idempotent toggle)
- **Page offset** - `document.body.marginRight` is adjusted so the sidebar doesn't overlap content

## React Detection

Traverses `__reactFiber$` / `__reactInternalInstance$` keys on DOM nodes to walk up the fiber tree. Extracts component names from `displayName` or `name` properties. Also reads `_debugSource` for source file info (available in dev builds).

Handles: function components, class components, forwardRef, memo wrappers.

## Permissions

- `activeTab` - access to the current tab when user clicks the extension
- `scripting` - inject content script programmatically
