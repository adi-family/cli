/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

import type { ConnectionInfo, DeviceInfo, RoomInfo } from './models';

import { WsState } from './enums';

export interface AdiSignalingStateEvent {
  url: string;
  state: WsState;
}

export interface AdiSignalingAuthOkEvent {
  url: string;
  userId: string;
}

export interface AdiSignalingAuthErrorEvent {
  url: string;
  reason: string;
}

export interface AdiSignalingConnectionInfoEvent {
  url: string;
  connectionInfo: ConnectionInfo;
}

export interface AdiSignalingDevicesEvent {
  url: string;
  devices: DeviceInfo[];
}

export interface AdiSignalingPeerConnectedEvent {
  url: string;
  peerId: string;
}

export interface AdiSignalingPeerDisconnectedEvent {
  url: string;
  peerId: string;
}

export interface AdiSignalingAuthAnonymousEvent {
  signalingUrl: string;
  authDomain: string;
}

export interface AdiSignalingDeviceRegisteredEvent {
  url: string;
  deviceId: string;
  tags?: Record<string, string>;
}

export interface AdiSignalingDeviceDeregisteredEvent {
  url: string;
  deviceId: string;
}

export interface AdiSignalingTagsUpdatedEvent {
  url: string;
  deviceId: string;
  tags: Record<string, string>;
}

export interface AdiSignalingDeviceUpdatedEvent {
  url: string;
  deviceId: string;
  tags: Record<string, string>;
  deviceConfig?: unknown;
}

export interface AdiSignalingPairingCodeEvent {
  url: string;
  code: string;
}

export interface AdiSignalingPairingConnectedEvent {
  url: string;
  peerId: string;
}

export interface AdiSignalingPairingFailedEvent {
  url: string;
  reason: string;
}

export interface AdiSignalingSyncDataEvent {
  url: string;
  payload: unknown;
}

export interface AdiSignalingRoomUpdatedEvent {
  url: string;
  room: RoomInfo;
}

export interface AdiSignalingRoomActorJoinedEvent {
  url: string;
  roomId: string;
  deviceId: string;
}

export interface AdiSignalingRoomActorLeftEvent {
  url: string;
  roomId: string;
  deviceId: string;
}

export interface AdiSignalingRoomMessageEvent {
  url: string;
  roomId: string;
  from: string;
  payload: unknown;
}

export interface AdiAuthStateChangedEvent {
  user: unknown;
}

export interface AdiAuthGetTokenEvent {
  authDomain: string;
  sourceUrl?: string;
}

export interface AdiAuthTokenResolvedEvent {
  authDomain: string;
  token: string | null;
}

export interface AdiAuthSessionSaveEvent {
  accessToken: string;
  email: string;
  expiresAt: number;
  authUrl: string;
}

export enum AdiAuthBusKey {
  StateChanged = 'adi.auth:state-changed',
  GetToken = 'adi.auth:get-token',
  TokenResolved = 'adi.auth:token-resolved',
  SessionSave = 'adi.auth:session-save',
}

export enum AdiSignalingBusKey {
  State = 'adi.signaling:state',
  AuthOk = 'adi.signaling:auth-ok',
  AuthError = 'adi.signaling:auth-error',
  ConnectionInfo = 'adi.signaling:connection-info',
  Devices = 'adi.signaling:devices',
  PeerConnected = 'adi.signaling:peer-connected',
  PeerDisconnected = 'adi.signaling:peer-disconnected',
  AuthAnonymous = 'adi.signaling:auth-anonymous',
  DeviceRegistered = 'adi.signaling:device-registered',
  DeviceDeregistered = 'adi.signaling:device-deregistered',
  TagsUpdated = 'adi.signaling:tags-updated',
  DeviceUpdated = 'adi.signaling:device-updated',
  PairingCode = 'adi.signaling:pairing-code',
  PairingConnected = 'adi.signaling:pairing-connected',
  PairingFailed = 'adi.signaling:pairing-failed',
  SyncData = 'adi.signaling:sync-data',
  RoomUpdated = 'adi.signaling:room-updated',
  RoomActorJoined = 'adi.signaling:room-actor-joined',
  RoomActorLeft = 'adi.signaling:room-actor-left',
  RoomMessage = 'adi.signaling:room-message',
}
