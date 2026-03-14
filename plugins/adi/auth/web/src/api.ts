import type { AuthToken, UserInfo } from './types.js';

const json = (res: Response) =>
  res.ok ? res.json() : res.json().then(e => Promise.reject(e.error ?? res.statusText));

export const requestCode = (authUrl: string, email: string): Promise<void> =>
  fetch(`${authUrl}/request-code`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email }),
  }).then(r => { if (!r.ok) return r.json().then(e => Promise.reject(e.error ?? r.statusText)); });

export const verifyCode = async (authUrl: string, email: string, code: string): Promise<AuthToken> => {
  const raw = await fetch(`${authUrl}/verify`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, code }),
  }).then(json) as { accessToken: string; tokenType: string; expiresIn: number };
  return { accessToken: raw.accessToken, tokenType: raw.tokenType, expiresIn: raw.expiresIn };
};

export interface AnonymousResult {
  login: string;
  password: string;
  accessToken: string;
  tokenType: string;
  expiresIn: number;
}

export const loginAnonymous = async (authUrl: string): Promise<AnonymousResult> => {
  const raw = await fetch(`${authUrl}/anonymous`, { method: 'POST' }).then(json) as {
    login: string; password: string; accessToken: string; tokenType: string; expiresIn: number;
  };
  return {
    login: raw.login,
    password: raw.password,
    accessToken: raw.accessToken,
    tokenType: raw.tokenType,
    expiresIn: raw.expiresIn,
  };
};

export const getMe = async (authUrl: string, token: string): Promise<UserInfo> => {
  const raw = await fetch(`${authUrl}/me`, {
    headers: { Authorization: `Bearer ${token}` },
  }).then(json) as { id: string; email: string; createdAt: string; lastLoginAt?: string; isAdmin: boolean };
  return {
    id: raw.id,
    email: raw.email,
    createdAt: raw.createdAt,
    lastLoginAt: raw.lastLoginAt,
    isAdmin: raw.isAdmin,
  };
};
