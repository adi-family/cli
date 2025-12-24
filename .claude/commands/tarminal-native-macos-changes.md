---
allowed-tools: Bash(ls:*), Bash(cat:*), Bash(xcodebuild:*), Bash(xcodegen:*), Read, Grep, Glob, Edit, Write, Task
description: Load tarminal-native-macos context and wait for task
---

## Project: tarminal-native-macos

**Path**: `apps/tarminal-native-macos/`
**Tech**: Swift 5.9, SwiftUI, macOS 14.0+, iOS 17.0+
**Bundle**: family.adi.tarminal-native (macOS), family.adi.tarminal-native-ios (iOS)

## Targets

### TarminalNative (macOS)
- Full terminal emulator with PTY support
- Workspace/session management
- P2P sync via MultipeerConnectivity + WebSocket
- Dependencies: TarminalCore, TarminalTerminal, TarminalPTY, TarminalSync, TarminalWebSync, TarminalSSH, TarminalUI, TarminalUIKit

### TarminalNativeiOS (iOS)
- Companion app for iPhone/iPad
- Read-only view of synced workspaces/sessions
- P2P sync with macOS app
- Dependencies: TarminalCore, TarminalSync, TarminalWebSync, TarminalTerminal, TarminalUIKitiOS

## macOS Source Files (TarminalNative/Sources/)

```
ContentView.swift          - Main NavigationSplitView layout
EmptyStateView.swift       - Empty state when no workspaces
InteractiveTerminalView.swift - Full PTY terminal with mouse support
PairingView.swift          - Device pairing UI
PtyTerminalView.swift      - PTY-based terminal view
SettingsView.swift         - App settings including sync toggle
SidebarView.swift          - Workspace/session navigation
StateCoordinator.swift     - Central state management
Styles.swift               - UI style definitions
SyncService.swift          - P2P sync via MultipeerConnectivity
TarminalNativeApp.swift    - App entry, menu bar, window management
TerminalView.swift         - Terminal display and input handling
WebSyncService.swift       - WebSocket-based sync service
Terminal/
  FullTerminal.swift       - Full terminal emulation view
  TerminalNSView.swift     - NSView wrapper for terminal
ViewModels/
  CommandBlockViewModel.swift  - Observable command execution state
  SessionViewModel.swift       - Observable SyncableSession + runtime PTY
  WorkspaceViewModel.swift     - Observable SyncableWorkspace wrapper
```

## iOS Source Files (TarminalNativeiOS/Sources/)

```
ContentView.swift          - NavigationSplitView layout
PairingView.swift          - Device pairing UI
SessionListView.swift      - Session list and detail views
SessionViewModeliOS.swift  - iOS session view model
StateCoordinatoriOS.swift  - Read-only state coordinator
SyncServiceiOS.swift       - P2P sync receiver
SyncStatusView.swift       - Sync status indicator
TarminalNativeiOSApp.swift - iOS app entry point
WebSyncServiceiOS.swift    - WebSocket sync for iOS
WorkspaceListView.swift    - Workspace sidebar
WorkspaceViewModeliOS.swift - iOS workspace view model
```

## Shared Packages (in Packages/)

### TarminalCore
Sync-ready models and persistence:
- Models: SyncableWorkspace, SyncableSession, SyncableCommandBlock, CommandStatus, ConnectionType, TerminalMode
- Sync: DeviceID, SyncableModel, SyncMetadata, VersionVector
- Persistence: Local storage management

### TarminalTerminal
Terminal emulation engine:
- Grid: TerminalGrid, Cell, CellAttributes, TerminalColor, MouseMode
- GridSync: Grid state synchronization
- Parser: ANSI escape sequence parsing

### TarminalPTY (macOS only)
PTY handling:
- PtyController.swift - PTY process control
- TerminalConnection.swift - Terminal connection management

### TarminalSync
P2P sync via MultipeerConnectivity:
- Transport: MultipeerTransport, TransportLayer
- Protocol: Sync message definitions
- Merge: CRDT-based merge logic
- Security: Sync encryption
- SyncManager.swift - Sync orchestration

### TarminalWebSync
WebSocket-based sync:
- WebSocketTransport.swift - WebSocket connection
- WebSyncManager.swift - WebSocket sync orchestration
- SignalingMessage.swift - Signaling protocol

### TarminalSSH
SSH connection support:
- SSHClient.swift - SSH client implementation
- SSHKeyManager.swift - SSH key management

### TarminalUI
Shared UI components:
- Styles: Common style definitions
- Views: Reusable view components

### TarminalUIKit (macOS)
macOS-specific UI:
- TerminalNSView.swift - NSView terminal wrapper

### TarminalUIKitiOS (iOS)
iOS-specific UI:
- TerminalUIView.swift - UIView terminal wrapper
- TerminalKeyboardView.swift - Custom keyboard accessory

## Build Commands

```bash
# Regenerate Xcode project from project.yml
cd apps/tarminal-native-macos && xcodegen generate

# Build macOS
xcodebuild -project TarminalNative.xcodeproj -scheme TarminalNative -configuration Debug build

# Build iOS (simulator)
xcodebuild -project TarminalNative.xcodeproj -scheme TarminalNativeiOS -destination 'generic/platform=iOS Simulator' -configuration Debug build

# Build iOS (device) - requires signing
xcodebuild -project TarminalNative.xcodeproj -scheme TarminalNativeiOS -destination 'generic/platform=iOS' -configuration Debug build
```

## Sync Architecture

### P2P Sync (MultipeerConnectivity)
1. macOS StateCoordinator manages state via TarminalCore models
2. Changes trigger SyncService via MultipeerTransport
3. SyncManager broadcasts updates with VersionVector CRDT
4. iOS receives updates, merges via StateCoordinatoriOS
5. Conflict resolution via VersionVector comparison

### WebSocket Sync
1. WebSyncService connects to signaling server
2. WebSyncManager handles message routing
3. Used for remote sync when devices not on same network

## Instructions

Context loaded. Respond with "ok" and wait for user's task.
