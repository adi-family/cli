/**
 * Auto-generated eventbus registry from TypeSpec.
 * DO NOT EDIT.
 */

import type { AdiAuthGetTokenEvent, AdiAuthSessionSaveEvent, AdiAuthStateChangedEvent, AdiAuthTokenResolvedEvent, AdiSignalingAuthAnonymousEvent, AdiSignalingAuthErrorEvent, AdiSignalingAuthOkEvent, AdiSignalingConnectionInfoEvent, AdiSignalingDeviceDeregisteredEvent, AdiSignalingDeviceRegisteredEvent, AdiSignalingDeviceUpdatedEvent, AdiSignalingDevicesEvent, AdiSignalingPairingCodeEvent, AdiSignalingPairingConnectedEvent, AdiSignalingPairingFailedEvent, AdiSignalingPeerConnectedEvent, AdiSignalingPeerDisconnectedEvent, AdiSignalingRoomActorJoinedEvent, AdiSignalingRoomActorLeftEvent, AdiSignalingRoomMessageEvent, AdiSignalingRoomUpdatedEvent, AdiSignalingStateEvent, AdiSignalingSyncDataEvent, AdiSignalingTagsUpdatedEvent } from './bus-types';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    // ── adi.signaling ──
    'adi.signaling:state': AdiSignalingStateEvent;
    'adi.signaling:auth-ok': AdiSignalingAuthOkEvent;
    'adi.signaling:auth-error': AdiSignalingAuthErrorEvent;
    'adi.signaling:connection-info': AdiSignalingConnectionInfoEvent;
    'adi.signaling:devices': AdiSignalingDevicesEvent;
    'adi.signaling:peer-connected': AdiSignalingPeerConnectedEvent;
    'adi.signaling:peer-disconnected': AdiSignalingPeerDisconnectedEvent;
    'adi.signaling:auth-anonymous': AdiSignalingAuthAnonymousEvent;
    'adi.signaling:device-registered': AdiSignalingDeviceRegisteredEvent;
    'adi.signaling:device-deregistered': AdiSignalingDeviceDeregisteredEvent;
    'adi.signaling:tags-updated': AdiSignalingTagsUpdatedEvent;
    'adi.signaling:device-updated': AdiSignalingDeviceUpdatedEvent;
    'adi.signaling:pairing-code': AdiSignalingPairingCodeEvent;
    'adi.signaling:pairing-connected': AdiSignalingPairingConnectedEvent;
    'adi.signaling:pairing-failed': AdiSignalingPairingFailedEvent;
    'adi.signaling:sync-data': AdiSignalingSyncDataEvent;
    'adi.signaling:room-updated': AdiSignalingRoomUpdatedEvent;
    'adi.signaling:room-actor-joined': AdiSignalingRoomActorJoinedEvent;
    'adi.signaling:room-actor-left': AdiSignalingRoomActorLeftEvent;
    'adi.signaling:room-message': AdiSignalingRoomMessageEvent;

    // ── adi.auth ──
    'adi.auth:state-changed': AdiAuthStateChangedEvent;
    'adi.auth:get-token': AdiAuthGetTokenEvent;
    'adi.auth:token-resolved': AdiAuthTokenResolvedEvent;
    'adi.auth:session-save': AdiAuthSessionSaveEvent;
  }
}
