@format("uuid")
scalar uuid extends string;

enum CredentialType {
  apiKey: "api_key",
  oauth2: "oauth2",
  sshKey: "ssh_key",
  password: "password",
  certificate: "certificate",
  custom: "custom",
}

model Credential {
  id: uuid;
  name: string;
  description?: string;
  credential_type: CredentialType;
  metadata: Record<unknown>;
  provider?: string;
  expires_at?: utcDateTime;
  created_at: utcDateTime;
  updated_at: utcDateTime;
  last_used_at?: utcDateTime;
}

model CredentialWithData {
  ...Credential;
  data: Record<unknown>;
}

model CredentialAccessLog {
  id: uuid;
  credential_id: uuid;
  user_id: uuid;
  action: string;
  ip_address?: string;
  user_agent?: string;
  details?: Record<unknown>;
  created_at: utcDateTime;
}

model VerifyResult {
  valid: boolean;
  is_expired: boolean;
  expires_at?: utcDateTime;
}

model DeleteResult {
  deleted: boolean;
}

// ── EventBus: Credentials ─────────────────────────────────

model CredentialRef {
  id: uuid;
  cocoonId: string;
}

model CredentialWithCocoon {
  ...Credential;
  cocoonId: string;
}

model CredentialWithDataAndCocoon {
  ...CredentialWithData;
  cocoonId: string;
}

@bus("adi.credentials")
interface CredentialsBus {
  // Commands (UI → plugin)
  @event list(credential_type?: CredentialType, provider?: string): void;
  @event get(id: uuid, cocoonId: string): void;
  @event reveal(id: uuid, cocoonId: string): void;
  @event create(cocoonId: string, name: string, credential_type: CredentialType, data: Record<unknown>, description?: string, provider?: string, expires_at?: utcDateTime): void;
  @event update(cocoonId: string, id: uuid, name?: string, description?: string, data?: Record<unknown>, provider?: string, expires_at?: utcDateTime): void;
  @event delete(id: uuid, cocoonId: string): void;
  @event verify(id: uuid, cocoonId: string): void;
  @event logs(id: uuid, cocoonId: string): void;

  // Responses (plugin → UI)
  @event listChanged(credentials: CredentialWithCocoon[]): void;
  @event detailChanged(credential: CredentialWithCocoon): void;
  @event dataRevealed(credential: CredentialWithDataAndCocoon): void;
  @event mutated(credential: CredentialWithCocoon): void;
  @event deleted(id: uuid, cocoonId: string): void;
  @event verified(id: uuid, result: VerifyResult): void;
  @event logsChanged(id: uuid, logs: CredentialAccessLog[]): void;
  @event error(message: string, event: string): void;
}

// ── AdiService: Credentials ───────────────────────────────

@channel("adi.credentials")
interface CredentialsService {
  @request
  list(credential_type?: CredentialType, provider?: string): Credential[];

  @request
  get(id: uuid): Credential;

  @request
  getWithData(id: uuid): CredentialWithData;

  @request
  create(
    name: string,
    credential_type: CredentialType,
    data: Record<unknown>,
    description?: string,
    metadata?: Record<unknown>,
    provider?: string,
    expires_at?: utcDateTime,
  ): Credential;

  @request
  update(
    id: uuid,
    name?: string,
    description?: string,
    data?: Record<unknown>,
    metadata?: Record<unknown>,
    provider?: string,
    expires_at?: utcDateTime,
  ): Credential;

  @request
  delete(id: uuid): DeleteResult;

  @request
  verify(id: uuid): VerifyResult;

  @request
  accessLogs(id: uuid): CredentialAccessLog[];
}
