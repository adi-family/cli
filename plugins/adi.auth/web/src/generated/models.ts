/**
 * Auto-generated models from TypeSpec.
 * DO NOT EDIT.
 */


export interface RequestCodeInput {
  email: string;
}

export interface VerifyCodeInput {
  email: string;
  code: string;
}

export interface VerifyTotpInput {
  email: string;
  code: string;
}

export interface EnableTotpInput {
  secret: string;
  code: string;
}

export interface AuthToken {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
}

export interface TotpSetup {
  secret: string;
  otpauthUrl: string;
  qrCodeBase64: string;
}

export interface UserInfo {
  id: string;
  email: string;
  createdAt: string;
  lastLoginAt?: string;
  isAdmin: boolean;
}

export interface SubtokenInput {
  ttlSeconds?: number;
}

export interface MessageResponse {
  message: string;
}

export interface ErrorResponse {
  error: string;
}
