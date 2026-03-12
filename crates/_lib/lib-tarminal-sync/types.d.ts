/**
 * Tarminal Sync Protocol - TypeScript Type Definitions
 *
 * Client-agnostic synchronization protocol for Tarminal terminal emulator.
 * These types match the Rust implementation for seamless JSON-based communication.
 */

export type DeviceId = string; // UUID v4 string
export type Uuid = string; // UUID v4 string

/**
 * Version Vector for CRDT-based causality tracking
 */
export interface VersionVector {
  clocks: Record<DeviceId, number>;
}

/**
 * Sync metadata for conflict resolution
 */
export interface SyncMetadata {
  created_at: string; // ISO 8601 datetime
  modified_at: string; // ISO 8601 datetime
  version: VersionVector;
  origin_device_id: DeviceId;
  tombstone: boolean;
}

/**
 * Sync protocol messages
 */
export type SyncMessage =
  | { type: 'hello'; device_id: DeviceId; display_name: string; app_version: string; protocol_version: number }
  | { type: 'request_full_sync' }
  | { type: 'full_state'; state: AppState }
  | { type: 'workspace_update'; workspace: SyncableWorkspace }
  | { type: 'session_update'; session: SyncableSession }
  | { type: 'command_block_update'; block: SyncableCommandBlock }
  | { type: 'delete'; entity_type: EntityType; entity_id: Uuid; deleted_by: DeviceId; deleted_at: string }
  | { type: 'ack'; message_id: Uuid }
  | { type: 'ping' }
  | { type: 'pong' };

export type EntityType = 'workspace' | 'session' | 'command_block';

/**
 * Complete application state for full sync
 */
export interface AppState {
  workspaces: SyncableWorkspace[];
  sessions: SyncableSession[];
  command_blocks: SyncableCommandBlock[];
}

/**
 * Syncable workspace entity
 */
export interface SyncableWorkspace {
  id: Uuid;
  name: string;
  icon: string | null;
  session_ids: Uuid[];
  active_session_id: Uuid | null;
  sync_metadata: SyncMetadata;
}

/**
 * Syncable session entity
 */
export interface SyncableSession {
  id: Uuid;
  workspace_id: Uuid;
  title: string;
  command_block_ids: Uuid[];
  current_directory: string;
  session_type: SessionType;
  sync_metadata: SyncMetadata;
}

export type SessionType = 'block_based' | 'interactive';

/**
 * Syncable command block entity
 */
export interface SyncableCommandBlock {
  id: Uuid;
  session_id: Uuid;
  command: string;
  output: string;
  exit_code: number | null;
  started_at: string; // ISO 8601 datetime
  finished_at: string | null; // ISO 8601 datetime
  sync_metadata: SyncMetadata;
}

/**
 * Signaling server messages
 */
export type SignalingMessage =
  | { type: 'register'; device_id: string }
  | { type: 'registered'; device_id: string }
  | { type: 'create_pairing_code' }
  | { type: 'pairing_code'; code: string }
  | { type: 'use_pairing_code'; code: string }
  | { type: 'paired'; peer_id: string }
  | { type: 'pairing_failed'; reason: string }
  | { type: 'sync_data'; payload: any }
  | { type: 'peer_connected'; peer_id: string }
  | { type: 'peer_disconnected'; peer_id: string }
  | { type: 'error'; message: string };

/**
 * Terminal grid synchronization
 */
export interface Cell {
  char: string;
  fg: TerminalColor;
  bg: TerminalColor;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
  inverse: boolean;
  hidden: boolean;
  strikethrough: boolean;
}

export type TerminalColor =
  | { type: 'default' }
  | { type: 'named'; color: NamedColor }
  | { type: 'indexed'; index: number }
  | { type: 'rgb'; r: number; g: number; b: number };

export enum NamedColor {
  Black = 0,
  Red = 1,
  Green = 2,
  Yellow = 3,
  Blue = 4,
  Magenta = 5,
  Cyan = 6,
  White = 7,
  BrightBlack = 8,
  BrightRed = 9,
  BrightGreen = 10,
  BrightYellow = 11,
  BrightBlue = 12,
  BrightMagenta = 13,
  BrightCyan = 14,
  BrightWhite = 15,
}

export interface GridDelta {
  operations: GridOperation[];
  base_version: number;
  new_version: number;
}

export type GridOperation =
  | { op: 'set_cells'; row: number; start_col: number; cells: Cell[] }
  | { op: 'scroll_up'; lines: number }
  | { op: 'scroll_down'; lines: number }
  | { op: 'clear_region'; x: number; y: number; width: number; height: number }
  | { op: 'resize'; cols: number; rows: number }
  | { op: 'cursor_move'; x: number; y: number }
  | { op: 'cursor_visibility'; visible: boolean }
  | { op: 'set_title'; title: string }
  | { op: 'full_snapshot'; snapshot: GridSnapshot };

export interface GridSnapshot {
  cols: number;
  rows: number;
  cells: Cell[][];
  cursor_x: number;
  cursor_y: number;
  cursor_visible: boolean;
  scroll_top: number;
  scroll_bottom: number;
  version: number;
  title: string;
}

/**
 * Helper functions for working with version vectors
 */
export namespace VersionVectorUtils {
  export function increment(vv: VersionVector, deviceId: DeviceId): VersionVector {
    return {
      clocks: {
        ...vv.clocks,
        [deviceId]: (vv.clocks[deviceId] || 0) + 1,
      },
    };
  }

  export function happensBefore(a: VersionVector, b: VersionVector): boolean {
    let atLeastOneSmaller = false;

    for (const [device, clock] of Object.entries(a.clocks)) {
      const otherClock = b.clocks[device] || 0;
      if (clock > otherClock) return false;
      if (clock < otherClock) atLeastOneSmaller = true;
    }

    for (const [device, clock] of Object.entries(b.clocks)) {
      if (!(device in a.clocks) && clock > 0) {
        atLeastOneSmaller = true;
      }
    }

    return atLeastOneSmaller;
  }

  export function concurrent(a: VersionVector, b: VersionVector): boolean {
    return !happensBefore(a, b) && !happensBefore(b, a) && !equal(a, b);
  }

  export function equal(a: VersionVector, b: VersionVector): boolean {
    return JSON.stringify(a.clocks) === JSON.stringify(b.clocks);
  }

  export function merge(a: VersionVector, b: VersionVector): VersionVector {
    const result: Record<DeviceId, number> = {};
    const allDevices = new Set([...Object.keys(a.clocks), ...Object.keys(b.clocks)]);

    for (const device of allDevices) {
      result[device] = Math.max(a.clocks[device] || 0, b.clocks[device] || 0);
    }

    return { clocks: result };
  }
}
