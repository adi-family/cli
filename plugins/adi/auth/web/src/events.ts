import type { UserInfo } from './types.js';

declare module '@adi-family/sdk-plugin/types' {
  interface EventRegistry {
    'auth:login':           { email: string; authUrl: string };
    'auth:login-anonymous': { authUrl: string };
    'auth:verify':          { email: string; code: string; authUrl: string };
    'auth:logout':          { authUrl?: string };
    'auth:me':              Record<string, never>;
    'auth:session-save':    { accessToken: string; email: string; expiresAt: number; authUrl: string };

    'auth:state-changed':   { user: UserInfo | null };
  }
}

export {};
