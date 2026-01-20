// Popup script for ADI Browser Debugger

async function loadStatus() {
  const response = await chrome.runtime.sendMessage({ type: 'getStatus' });
  
  // Update connection status
  const statusEl = document.getElementById('status');
  const statusTextEl = document.getElementById('statusText');
  
  if (response.connected) {
    statusEl.className = 'status connected';
    statusTextEl.textContent = `Connected (${response.activeTabs} tabs)`;
  } else {
    statusEl.className = 'status disconnected';
    statusTextEl.textContent = 'Disconnected';
  }
  
  // Update tabs list
  const tabsListEl = document.getElementById('tabsList');
  
  if (response.tabs && response.tabs.length > 0) {
    tabsListEl.innerHTML = response.tabs.map(tab => `
      <div class="tab-item" data-tab-id="${tab.tabId}">
        <div class="tab-info">
          <div class="tab-title" title="${escapeHtml(tab.url)}">${escapeHtml(tab.title)}</div>
          <div class="tab-stats">${tab.requestCount} requests, ${tab.consoleCount} console entries</div>
        </div>
        <button class="tab-stop" data-tab-id="${tab.tabId}">Stop</button>
      </div>
    `).join('');
    
    // Add click handlers for stop buttons
    tabsListEl.querySelectorAll('.tab-stop').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const tabId = parseInt(e.target.dataset.tabId);
        await chrome.runtime.sendMessage({ type: 'stopDebugging', tabId });
        loadStatus();
      });
    });
  } else {
    tabsListEl.innerHTML = '<div class="no-tabs">No active debug sessions</div>';
  }
  
  // Update settings
  document.getElementById('signalingUrl').value = response.signalingUrl || '';
  document.getElementById('browserId').textContent = response.browserId || 'Unknown';
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Settings form
document.getElementById('settingsForm').addEventListener('submit', async (e) => {
  e.preventDefault();
  
  const signalingUrl = document.getElementById('signalingUrl').value.trim();
  const maxBodySize = parseInt(document.getElementById('maxBodySize').value);
  
  await chrome.runtime.sendMessage({
    type: 'updateSettings',
    signalingUrl: signalingUrl || undefined,
    responseBodyMaxSize: maxBodySize || undefined
  });
  
  // Reload status
  loadStatus();
});

// Load initial status
loadStatus();

// Refresh status periodically
setInterval(loadStatus, 2000);
