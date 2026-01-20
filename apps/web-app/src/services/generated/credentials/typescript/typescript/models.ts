/**
 * Auto-generated models from TypeSpec.
 * DO NOT EDIT.
 */

import { CredentialType } from './enums';

export interface Credential {
  id: string;
  name: string;
  description?: string;
  credentialType: CredentialType;
  metadata: Record<string, unknown>;
  provider?: string;
  expiresAt?: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string;
}

export interface CredentialWithData {
  id: string;
  name: string;
  description?: string;
  credentialType: CredentialType;
  metadata: Record<string, unknown>;
  provider?: string;
  expiresAt?: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string;
  data: Record<string, unknown>;
}

export interface CreateCredential {
  name: string;
  description?: string;
  credentialType: CredentialType;
  data: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  provider?: string;
  expiresAt?: string;
}

export interface UpdateCredential {
  name?: string;
  description?: string;
  data?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  provider?: string;
  expiresAt?: string;
}

export interface CredentialAccessLog {
  id: string;
  credentialId: string;
  userId: string;
  action: string;
  ipAddress?: string;
  userAgent?: string;
  details?: Record<string, unknown>;
  createdAt: string;
}

export interface VerifyResult {
  valid: boolean;
  isExpired: boolean;
  expiresAt?: string;
}

export interface DeleteResult {
  deleted: boolean;
}
