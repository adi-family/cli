import type { CredentialType } from '../types.js';

export const TYPE_COLORS: Record<CredentialType, string> = {
  github_token: 'bg-gray-700/30 text-gray-200',
  gitlab_token: 'bg-orange-500/20 text-orange-300',
  api_key: 'bg-blue-500/20 text-blue-300',
  oauth2: 'bg-purple-500/20 text-purple-300',
  ssh_key: 'bg-green-500/20 text-green-300',
  password: 'bg-yellow-500/20 text-yellow-300',
  certificate: 'bg-cyan-500/20 text-cyan-300',
  custom: 'bg-gray-500/20 text-gray-300',
};

export const TYPE_LABELS: Record<CredentialType, string> = {
  github_token: 'GitHub Token',
  gitlab_token: 'GitLab Token',
  api_key: 'API Key',
  oauth2: 'OAuth2',
  ssh_key: 'SSH Key',
  password: 'Password',
  certificate: 'Certificate',
  custom: 'Custom',
};

export const ALL_TYPES: CredentialType[] = [
  'github_token', 'gitlab_token', 'api_key', 'oauth2',
  'ssh_key', 'password', 'certificate', 'custom',
];

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
