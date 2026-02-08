/**
 * Credentials API - Secure credentials storage service
 *
 * Provides encrypted storage for various credential types using ChaCha20-Poly1305 AEAD encryption.
 */

import "@typespec/http";
import "@typespec/rest";

using TypeSpec.Http;
using TypeSpec.Rest;

namespace Credentials;

// === Scalars ===

@format("uuid")
scalar uuid extends string;

// === Enums ===

/** Type of credential being stored */
enum CredentialType {
  githubToken: "github_token",
  gitlabToken: "gitlab_token",
  apiKey: "api_key",
  oauth2: "oauth2",
  sshKey: "ssh_key",
  password: "password",
  certificate: "certificate",
  custom: "custom",
}

// === Models ===

/** Credential metadata (without sensitive data) */
model Credential {
  /** Unique identifier */
  id: uuid;

  /** User-defined name for the credential */
  name: string;

  /** Optional description */
  description?: string;

  /** Type of credential */
  credentialType: CredentialType;

  /** Non-sensitive metadata (key-value pairs) */
  metadata: Record<unknown>;

  /** Provider identifier (e.g., "github.com", "gitlab.com") */
  provider?: string;

  /** When the credential expires (null if no expiration) */
  expiresAt?: utcDateTime;

  /** When the credential was created */
  createdAt: utcDateTime;

  /** When the credential was last updated */
  updatedAt: utcDateTime;

  /** When the credential was last accessed */
  lastUsedAt?: utcDateTime;
}

/** Credential with decrypted sensitive data */
model CredentialWithData {
  ...Credential;

  /** Decrypted credential data */
  data: Record<unknown>;
}

/** Request to create a new credential */
model CreateCredential {
  /** User-defined name for the credential */
  name: string;

  /** Optional description */
  description?: string;

  /** Type of credential */
  credentialType: CredentialType;

  /** The actual credential data (will be encrypted) */
  data: Record<unknown>;

  /** Optional non-sensitive metadata */
  metadata?: Record<unknown>;

  /** Provider identifier */
  provider?: string;

  /** When the credential expires */
  expiresAt?: utcDateTime;
}

/** Request to update an existing credential */
model UpdateCredential {
  /** Updated name */
  name?: string;

  /** Updated description */
  description?: string;

  /** Updated credential data (will be re-encrypted) */
  data?: Record<unknown>;

  /** Updated metadata */
  metadata?: Record<unknown>;

  /** Updated provider */
  provider?: string;

  /** Updated expiration */
  expiresAt?: utcDateTime;
}

/** Access log entry for audit trail */
model CredentialAccessLog {
  /** Log entry ID */
  id: uuid;

  /** Credential that was accessed */
  credentialId: uuid;

  /** User who performed the action */
  userId: uuid;

  /** Action performed: 'read', 'create', 'update', 'delete' */
  action: string;

  /** IP address of the request */
  ipAddress?: string;

  /** User agent of the request */
  userAgent?: string;

  /** Additional details about the action */
  details?: Record<unknown>;

  /** When the action occurred */
  createdAt: utcDateTime;
}

/** Result of credential verification */
model VerifyResult {
  /** Whether the credential is valid (not expired) */
  valid: boolean;

  /** Whether the credential has expired */
  isExpired: boolean;

  /** When the credential expires (null if no expiration) */
  expiresAt?: utcDateTime;
}

/** Result of credential deletion */
model DeleteResult {
  /** Whether the credential was deleted */
  deleted: boolean;
}

// === API Interface ===

@route("/credentials")
interface CredentialsService {
  /** List all credentials for the authenticated user */
  @get
  list(
    @query credentialType?: CredentialType,
    @query provider?: string
  ): Credential[];

  /** Create a new credential */
  @post
  create(@body body: CreateCredential): Credential;

  /** Get a credential by ID (without decrypted data) */
  @get
  @route("/{id}")
  get(@path id: uuid): Credential;

  /** Update a credential */
  @put
  @route("/{id}")
  update(@path id: uuid, @body body: UpdateCredential): Credential;

  /** Delete a credential */
  @delete
  @route("/{id}")
  delete(@path id: uuid): DeleteResult;

  /** Get a credential with decrypted data */
  @get
  @route("/{id}/data")
  getWithData(@path id: uuid): CredentialWithData;

  /** Get access logs for a credential */
  @get
  @route("/{id}/logs")
  getAccessLogs(@path id: uuid): CredentialAccessLog[];

  /** Verify a credential is still valid (check expiration) */
  @get
  @route("/{id}/verify")
  verify(@path id: uuid): VerifyResult;
}
