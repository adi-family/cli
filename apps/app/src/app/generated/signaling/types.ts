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

export interface ConnectionInfo {
  manual_allowed: boolean;
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
