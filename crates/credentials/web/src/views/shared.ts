import { CredentialType } from '../types.js';

export const TYPE_COLORS: Record<CredentialType, string> = {
  [CredentialType.GithubToken]: 'bg-gray-700/30 text-gray-200',
  [CredentialType.GitlabToken]: 'bg-orange-500/20 text-orange-300',
  [CredentialType.ApiKey]: 'bg-blue-500/20 text-blue-300',
  [CredentialType.Oauth2]: 'bg-purple-500/20 text-purple-300',
  [CredentialType.SshKey]: 'bg-green-500/20 text-green-300',
  [CredentialType.Password]: 'bg-yellow-500/20 text-yellow-300',
  [CredentialType.Certificate]: 'bg-cyan-500/20 text-cyan-300',
  [CredentialType.Custom]: 'bg-gray-500/20 text-gray-300',
};

export const TYPE_LABELS: Record<CredentialType, string> = {
  [CredentialType.GithubToken]: 'GitHub Token',
  [CredentialType.GitlabToken]: 'GitLab Token',
  [CredentialType.ApiKey]: 'API Key',
  [CredentialType.Oauth2]: 'OAuth2',
  [CredentialType.SshKey]: 'SSH Key',
  [CredentialType.Password]: 'Password',
  [CredentialType.Certificate]: 'Certificate',
  [CredentialType.Custom]: 'Custom',
};

export const ALL_TYPES: CredentialType[] = [
  CredentialType.GithubToken, CredentialType.GitlabToken, CredentialType.ApiKey, CredentialType.Oauth2,
  CredentialType.SshKey, CredentialType.Password, CredentialType.Certificate, CredentialType.Custom,
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
