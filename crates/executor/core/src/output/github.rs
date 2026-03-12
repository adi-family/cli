use base64::{engine::general_purpose::STANDARD, Engine};
use lib_client_github::{token as token_auth, Client, CreateTreeEntry, GitHubError};
use tracing::info;

use crate::error::{ExecutorError, Result};
use crate::types::OutputFile;

pub struct GitHubOutputHandler {
    client: Client,
}

impl GitHubOutputHandler {
    pub fn new(token: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("adi-executor/0.1")
            .auth(token_auth(&token))
            .build()
            .map_err(|e: GitHubError| ExecutorError::GitPushFailed(e.to_string()))?;

        Ok(Self { client })
    }

    pub async fn push_to_branch(
        &self,
        repo: &str,
        branch: &str,
        files: &[OutputFile],
        commit_message: &str,
    ) -> Result<()> {
        let (owner, repo_name) = parse_repo(repo)?;
        info!(repo = %repo, branch = %branch, files = files.len(), "Pushing to GitHub");

        // Get or create branch
        let base_sha = self
            .get_or_create_branch(&owner, &repo_name, branch)
            .await?;

        // Create blobs for each file and build tree entries
        let mut tree_entries = Vec::new();
        for file in files {
            let (content, encoding) = if file.binary {
                (STANDARD.encode(&file.content), "base64")
            } else {
                (file.content.clone(), "utf-8")
            };

            let blob = self
                .client
                .create_blob(&owner, &repo_name, &content, encoding)
                .await
                .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

            tree_entries.push(CreateTreeEntry {
                path: file.path.clone(),
                mode: "100644".to_string(),
                content: None,
                sha: Some(blob.sha),
            });
        }

        // Create tree
        let tree = self
            .client
            .create_tree(&owner, &repo_name, Some(&base_sha), tree_entries)
            .await
            .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

        // Create commit
        let commit = self
            .client
            .create_commit(
                &owner,
                &repo_name,
                commit_message,
                &tree.sha,
                vec![&base_sha],
            )
            .await
            .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

        let commit_sha = commit["sha"]
            .as_str()
            .ok_or_else(|| ExecutorError::GitPushFailed("Missing commit sha".to_string()))?;

        // Update branch reference
        self.client
            .update_ref(
                &owner,
                &repo_name,
                &format!("heads/{}", branch),
                commit_sha,
                false,
            )
            .await
            .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

        info!(commit = %commit_sha, "Successfully pushed to GitHub");

        Ok(())
    }

    async fn get_or_create_branch(&self, owner: &str, repo: &str, branch: &str) -> Result<String> {
        // Try to get existing branch
        match self
            .client
            .get_ref(owner, repo, &format!("heads/{}", branch))
            .await
        {
            Ok(reference) => Ok(reference.object.sha),
            Err(_) => {
                // Branch doesn't exist, create from default branch
                let repository = self
                    .client
                    .get_repo(owner, repo)
                    .await
                    .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

                let default_ref = self
                    .client
                    .get_ref(owner, repo, &format!("heads/{}", repository.default_branch))
                    .await
                    .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

                let default_sha = default_ref.object.sha.clone();

                self.client
                    .create_ref(owner, repo, &format!("refs/heads/{}", branch), &default_sha)
                    .await
                    .map_err(|e| ExecutorError::GitPushFailed(e.to_string()))?;

                Ok(default_sha)
            }
        }
    }
}

fn parse_repo(repo: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 {
        return Err(ExecutorError::GitPushFailed(format!(
            "Invalid repo format: {}. Expected 'owner/repo'",
            repo
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}
