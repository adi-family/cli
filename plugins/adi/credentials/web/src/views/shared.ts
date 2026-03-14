import { CredentialType } from '../types.js';

const TYPE_META: Record<CredentialType, { label: string; color: string }> = {
  [CredentialType.ApiKey]:      { label: 'API Key',      color: 'bg-blue-500/20 text-blue-300' },
  [CredentialType.Oauth2]:      { label: 'OAuth2',       color: 'bg-purple-500/20 text-purple-300' },
  [CredentialType.SshKey]:      { label: 'SSH Key',      color: 'bg-green-500/20 text-green-300' },
  [CredentialType.Password]:    { label: 'Password',     color: 'bg-yellow-500/20 text-yellow-300' },
  [CredentialType.Certificate]: { label: 'Certificate',  color: 'bg-cyan-500/20 text-cyan-300' },
  [CredentialType.Custom]:      { label: 'Custom',       color: 'bg-gray-500/20 text-gray-300' },
};

export const ALL_TYPES = Object.keys(TYPE_META) as CredentialType[];
export const TYPE_LABELS: Record<CredentialType, string> = Object.fromEntries(
  ALL_TYPES.map(t => [t, TYPE_META[t].label]),
) as Record<CredentialType, string>;
export const TYPE_COLORS: Record<CredentialType, string> = Object.fromEntries(
  ALL_TYPES.map(t => [t, TYPE_META[t].color]),
) as Record<CredentialType, string>;

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

export function timeAgo(iso: string): string {
  const seconds = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function isExpired(expiresAt?: string): boolean {
  if (!expiresAt) return false;
  return new Date(expiresAt).getTime() < Date.now();
}
