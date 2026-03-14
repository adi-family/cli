/**
 * Auto-generated eventbus types from TypeSpec.
 * DO NOT EDIT.
 */

import type { CredentialAccessLog, CredentialWithCocoon, CredentialWithDataAndCocoon, VerifyResult } from './models';

import { CredentialType } from './enums';

export interface AdiCredentialsListEvent {
  credential_type?: CredentialType;
  provider?: string;
}

export interface AdiCredentialsGetEvent {
  id: string;
  cocoonId: string;
}

export interface AdiCredentialsRevealEvent {
  id: string;
  cocoonId: string;
}

export interface AdiCredentialsCreateEvent {
  cocoonId: string;
  name: string;
  credential_type: CredentialType;
  data: Record<string, unknown>;
  description?: string;
  provider?: string;
  expires_at?: string;
}

export interface AdiCredentialsUpdateEvent {
  cocoonId: string;
  id: string;
  name?: string;
  description?: string;
  data?: Record<string, unknown>;
  provider?: string;
  expires_at?: string;
}

export interface AdiCredentialsDeleteEvent {
  id: string;
  cocoonId: string;
}

export interface AdiCredentialsVerifyEvent {
  id: string;
  cocoonId: string;
}

export interface AdiCredentialsLogsEvent {
  id: string;
  cocoonId: string;
}

export interface AdiCredentialsListChangedEvent {
  credentials: CredentialWithCocoon[];
}

export interface AdiCredentialsDetailChangedEvent {
  credential: CredentialWithCocoon;
}

export interface AdiCredentialsDataRevealedEvent {
  credential: CredentialWithDataAndCocoon;
}

export interface AdiCredentialsMutatedEvent {
  credential: CredentialWithCocoon;
}

export interface AdiCredentialsDeletedEvent {
  id: string;
  cocoonId: string;
}

export interface AdiCredentialsVerifiedEvent {
  id: string;
  result: VerifyResult;
}

export interface AdiCredentialsLogsChangedEvent {
  id: string;
  logs: CredentialAccessLog[];
}

export interface AdiCredentialsErrorEvent {
  message: string;
  event: string;
}

export enum AdiCredentialsBusKey {
  List = 'adi.credentials:list',
  Get = 'adi.credentials:get',
  Reveal = 'adi.credentials:reveal',
  Create = 'adi.credentials:create',
  Update = 'adi.credentials:update',
  Delete = 'adi.credentials:delete',
  Verify = 'adi.credentials:verify',
  Logs = 'adi.credentials:logs',
  ListChanged = 'adi.credentials:list-changed',
  DetailChanged = 'adi.credentials:detail-changed',
  DataRevealed = 'adi.credentials:data-revealed',
  Mutated = 'adi.credentials:mutated',
  Deleted = 'adi.credentials:deleted',
  Verified = 'adi.credentials:verified',
  LogsChanged = 'adi.credentials:logs-changed',
  Error = 'adi.credentials:error',
}
