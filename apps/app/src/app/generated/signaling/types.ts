/**
 * Auto-generated protocol types from TypeSpec.
 * DO NOT EDIT.
 */

export enum WsState {
  Disconnected = "disconnected",
  Connecting = "connecting",
  Connected = "connected",
  Error = "error",
}

export enum AuthRequirement {
  Required = "required",
  Optional = "optional",
}

export enum AuthOption {
  Verified = "verified",
  Anonymous = "anonymous",
}

export interface IceServer {
  urls: string[];
  username?: string;
  credential?: string;
}

export interface ConnectionInfo {
  manual_allowed: boolean;
  ice_servers?: IceServer[];
}

export interface DeviceInfo {
  device_id: string;
  tags: Record<string, string>;
  online: boolean;
  device_type?: string;
  device_config?: unknown;
}

export interface CocoonKind {
  id: string;
  runner_type: string;
  runner_config: unknown;
  image: string;
}

export interface RoomInfo {
  room_id: string;
  owner_user_id: string;
  granted_users: string[];
  actors: DeviceInfo[];
}
