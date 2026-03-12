mod github;
mod webhook;

pub use github::GitHubOutputHandler;

use crate::error::Result;
use crate::types::{OutputConfig, OutputFile};

pub async fn handle_output(config: &OutputConfig, files: &[OutputFile]) -> Result<()> {
    match config {
        OutputConfig::GithubBranch {
            repo,
            branch,
            token,
            commit_message,
        } => {
            let handler = GitHubOutputHandler::new(token.clone())?;
            let message = commit_message
                .clone()
                .unwrap_or_else(|| "Update from adi-executor".to_string());
            handler.push_to_branch(repo, branch, files, &message).await
        }
        OutputConfig::Webhook { url, headers } => {
            webhook::send_webhook(url, headers.as_ref(), files).await
        }
    }
}
