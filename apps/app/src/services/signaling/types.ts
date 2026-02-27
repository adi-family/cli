// Signaling protocol types extracted from web-app reference.
// All wire types use snake_case to match server JSON serialization.

import type {} from '@adi-family/sdk-plugin';

// WebSocket connection state
export type WsState = 'disconnected' | 'connecting' | 'connected' | 'error';

// Per-cocoon WebRTC state
export type RtcState = 'idle' | 'signaling' | 'connecting' | 'connected' | 'failed';

// ---------------------------------------------------------------------------
// Signaling server entities
// ---------------------------------------------------------------------------

export interface ServiceInfo {
  name: string;
  service_type: 'http' | 'grpc' | 'custom';
  local_port: number;
  health_endpoint?: string;
}

export interface Capability {
  protocol: string;
  version: string;
}

export interface CocoonInfo {
  device_id: string;
  status: 'online' | 'offline';
  claimed_at: string;
  name?: string;
  services?: ServiceInfo[];
  capabilities?: Capability[];
  location?: string;
}

export interface CocoonKind {
  id: string;
  image: string;
}

export interface HiveInfo {
  hive_id: string;
  version: string;
  status: string;
  connected_at: string;
  cocoon_kinds: CocoonKind[];
}

// ---------------------------------------------------------------------------
// Signaling messages (WebSocket JSON frames)
// ---------------------------------------------------------------------------

export type SignalingMessage =
  // Authentication handshake
  | { type: 'hello'; auth_kind: string; auth_domain: string; auth_requirement: 'required' | 'optional'; auth_options: Array<'verified' | 'anonymous'> }
  | { type: 'authenticate'; access_token: string }
  | { type: 'authenticated'; user_id: string }
  // Cocoon management
  | { type: 'list_my_cocoons' }
  | { type: 'my_cocoons'; cocoons: CocoonInfo[] }
  | { type: 'remove_cocoon'; device_id: string }
  | { type: 'cocoon_removed'; device_id: string }
  | { type: 'connect_to_cocoon'; device_id: string }
  | { type: 'connected'; device_id: string }
  | { type: 'access_denied'; reason: string; auth_kind?: string; auth_domain?: string; plugin?: string }
  | { type: 'error'; message: string }
  | { type: 'sync_data'; payload: unknown }
  | { type: 'peer_connected'; peer_id: string }
  | { type: 'peer_disconnected'; peer_id: string }
  // Hive orchestration
  | { type: 'list_hives' }
  | { type: 'hives_list'; hives: HiveInfo[] }
  | { type: 'spawn_cocoon'; request_id: string; setup_token: string; name?: string; kind: string }
  | { type: 'spawn_cocoon_result'; request_id: string; success: boolean; device_id?: string; container_id?: string; error?: string }
  // WebRTC signaling
  | { type: 'web_rtc_start_session'; session_id: string; device_id: string }
  | { type: 'web_rtc_session_started'; session_id: string; device_id: string }
  | { type: 'web_rtc_offer'; session_id: string; sdp: string }
  | { type: 'web_rtc_answer'; session_id: string; sdp: string }
  | { type: 'web_rtc_ice_candidate'; session_id: string; candidate: string; sdp_mid?: string; sdp_mline_index?: number }
  | { type: 'web_rtc_session_ended'; session_id: string; reason?: string }
  | { type: 'web_rtc_error'; session_id: string; code: string; message: string }
  | { type: 'web_rtc_data'; session_id: string; channel: string; data: string; binary?: boolean };

// ---------------------------------------------------------------------------
// Data channel message types
// ---------------------------------------------------------------------------

export interface SilkHtmlSpan {
  text: string;
  classes?: string[];
  styles?: Record<string, string>;
}

export type PtyMessage =
  | { type: 'attach_pty'; command: string; cols: number; rows: number; env?: Record<string, string> }
  | { type: 'pty_created'; session_id: string }
  | { type: 'pty_input'; session_id: string; data: string }
  | { type: 'pty_output'; session_id: string; data: string }
  | { type: 'pty_resize'; session_id: string; cols: number; rows: number }
  | { type: 'pty_close'; session_id: string }
  | { type: 'pty_exited'; session_id: string; exit_code: number };

export type SilkMessage =
  // Requests (web -> cocoon)
  | { type: 'create_session'; cwd?: string; env?: Record<string, string>; shell?: string }
  | { type: 'execute'; session_id: string; command: string; command_id: string }
  | { type: 'input'; session_id: string; command_id: string; data: string }
  | { type: 'resize'; session_id: string; command_id: string; cols: number; rows: number }
  | { type: 'signal'; session_id: string; command_id: string; signal: 'interrupt' | 'terminate' | 'kill' }
  | { type: 'close_session'; session_id: string }
  // Responses (cocoon -> web)
  | { type: 'session_created'; session_id: string; cwd: string; shell: string }
  | { type: 'command_started'; session_id: string; command_id: string; interactive: boolean }
  | { type: 'output'; session_id: string; command_id: string; stream: 'stdout' | 'stderr'; data: string; html?: SilkHtmlSpan[] }
  | { type: 'interactive_required'; session_id: string; command_id: string; reason: string; pty_session_id: string }
  | { type: 'pty_output'; session_id: string; command_id: string; pty_session_id: string; data: string }
  | { type: 'command_completed'; session_id: string; command_id: string; exit_code: number; cwd: string }
  | { type: 'session_closed'; session_id: string }
  | { type: 'error'; session_id?: string; command_id?: string; code: string; message: string };

export interface FileEntry {
  name: string;
  is_dir: boolean;
  is_file: boolean;
  is_symlink: boolean;
  size?: number;
  modified?: string;
}

export interface FileStat {
  is_dir: boolean;
  is_file: boolean;
  is_symlink: boolean;
  size: number;
  modified?: string;
  created?: string;
  permissions?: number;
}

export interface WalkEntry {
  path: string;
  is_dir: boolean;
  size?: number;
}

export type FileSystemMessage =
  // Requests
  | { type: 'fs_list_dir'; request_id: string; path: string }
  | { type: 'fs_read_file'; request_id: string; path: string; offset?: number; limit?: number }
  | { type: 'fs_stat'; request_id: string; path: string }
  | { type: 'fs_walk'; request_id: string; path: string; max_depth?: number; pattern?: string }
  // Responses
  | { type: 'fs_dir_listing'; request_id: string; path: string; entries: FileEntry[] }
  | { type: 'fs_file_content'; request_id: string; path: string; content: string; encoding: 'utf8' | 'base64'; total_size: number }
  | { type: 'fs_file_stat'; request_id: string; path: string; stat: FileStat }
  | { type: 'fs_walk_result'; request_id: string; path: string; entries: WalkEntry[]; truncated: boolean }
  | { type: 'fs_error'; request_id: string; code: string; message: string };

// ---------------------------------------------------------------------------
// ADI service protocol
// ---------------------------------------------------------------------------

export interface AdiRequest {
  request_id: string;
  service: string;
  method: string;
  params: Record<string, unknown>;
}

export type AdiResponse =
  | { type: 'success'; request_id: string; data: unknown }
  | { type: 'stream'; request_id: string; data: unknown; done: boolean }
  | { type: 'error'; request_id: string; code: string; message: string }
  | { type: 'service_not_found'; request_id: string; service: string }
  | { type: 'method_not_found'; request_id: string; service: string; method: string };

export type AdiDiscovery =
  | { type: 'list_services' }
  | { type: 'services_list'; services: AdiServiceInfo[] };

export interface AdiServiceInfo {
  id: string;
  name: string;
  version: string;
  methods: AdiMethodInfo[];
}

export interface AdiMethodInfo {
  name: string;
  description: string;
  streaming: boolean;
  params_schema?: Record<string, unknown>;
}

export type AdiMessage = AdiRequest | AdiResponse | AdiDiscovery;

export type DataChannelName = 'terminal' | 'silk' | 'file' | 'pty' | 'adi';

// ---------------------------------------------------------------------------
// Event bus declaration merging
// ---------------------------------------------------------------------------

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'signaling:state': { url: string; state: WsState };
    'signaling:cocoons': { url: string; cocoons: CocoonInfo[] };
    'signaling:hives': { url: string; hives: HiveInfo[] };
    'signaling:session-state': { url: string; deviceId: string; state: RtcState; sessionId: string };
    'signaling:spawn-result': { url: string; requestId: string; success: boolean; deviceId?: string; error?: string };
    'signaling:auth-error': { url: string; reason: string; authKind?: string; authDomain?: string };
    'signaling:auth-anonymous': { signalingUrl: string; authDomain: string };
    'connection:added': { id: string; services: string[] };
    'connection:removed': { id: string };
  }
}
