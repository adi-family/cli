// Credential type labels and icons (not generated, manually maintained)

import { CredentialType } from './generated/credentials/typescript';

export const CREDENTIAL_TYPE_LABELS: Record<CredentialType, string> = {
  [CredentialType.GithubToken]: "GitHub Token",
  [CredentialType.GitlabToken]: "GitLab Token",
  [CredentialType.ApiKey]: "API Key",
  [CredentialType.Oauth2]: "OAuth 2.0",
  [CredentialType.SshKey]: "SSH Key",
  [CredentialType.Password]: "Password",
  [CredentialType.Certificate]: "Certificate",
  [CredentialType.Custom]: "Custom",
};

export const CREDENTIAL_TYPE_ICONS: Record<CredentialType, string> = {
  [CredentialType.GithubToken]: "Github",
  [CredentialType.GitlabToken]: "Gitlab",
  [CredentialType.ApiKey]: "Key",
  [CredentialType.Oauth2]: "Shield",
  [CredentialType.SshKey]: "Terminal",
  [CredentialType.Password]: "Lock",
  [CredentialType.Certificate]: "FileKey",
  [CredentialType.Custom]: "FileQuestion",
};
