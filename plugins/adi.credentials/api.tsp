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
