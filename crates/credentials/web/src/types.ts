export type {
  CredentialAccessLog,
  DeleteResult,
  VerifyResult,
} from './generated/types.js';

export { CredentialType } from './generated/types.js';

import type {
  Credential as GeneratedCredential,
  CredentialWithData as GeneratedCredentialWithData,
} from './generated/types.js';

/** Credential with the cocoonId injected by the plugin layer. */
export interface Credential extends GeneratedCredential {
  cocoonId: string;
}

/** CredentialWithData with the cocoonId injected by the plugin layer. */
export interface CredentialWithData extends GeneratedCredentialWithData {
  cocoonId: string;
}
