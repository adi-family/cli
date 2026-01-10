use serde::{Deserialize, Serialize};

/// Issue in Jira.
#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: String,
    pub fields: IssueFields,
}

/// Issue fields.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueFields {
    pub summary: String,
    pub description: Option<serde_json::Value>,
    pub status: Status,
    pub priority: Option<Priority>,
    pub issuetype: IssueType,
    pub project: Project,
    pub assignee: Option<User>,
    pub reporter: Option<User>,
    pub labels: Vec<String>,
    pub created: String,
    pub updated: String,
    pub resolutiondate: Option<String>,
}

/// Status.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub id: String,
    pub name: String,
    pub status_category: StatusCategory,
}

/// Status category.
#[derive(Debug, Clone, Deserialize)]
pub struct StatusCategory {
    pub id: u32,
    pub key: String,
    pub name: String,
}

/// Priority.
#[derive(Debug, Clone, Deserialize)]
pub struct Priority {
    pub id: String,
    pub name: String,
}

/// Issue type.
#[derive(Debug, Clone, Deserialize)]
pub struct IssueType {
    pub id: String,
    pub name: String,
    pub subtask: bool,
}

/// Project.
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(rename = "self")]
    pub self_url: String,
}

/// User.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub account_id: String,
    pub display_name: String,
    pub email_address: Option<String>,
    pub avatar_urls: Option<AvatarUrls>,
    pub active: bool,
}

/// Avatar URLs.
#[derive(Debug, Clone, Deserialize)]
pub struct AvatarUrls {
    #[serde(rename = "48x48")]
    pub large: Option<String>,
    #[serde(rename = "24x24")]
    pub small: Option<String>,
    #[serde(rename = "16x16")]
    pub xsmall: Option<String>,
    #[serde(rename = "32x32")]
    pub medium: Option<String>,
}

/// Sprint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprint {
    pub id: u64,
    pub name: String,
    pub state: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub complete_date: Option<String>,
    pub origin_board_id: u64,
}

/// Board.
#[derive(Debug, Clone, Deserialize)]
pub struct Board {
    pub id: u64,
    pub name: String,
    #[serde(rename = "type")]
    pub board_type: String,
}

/// Comment.
#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: User,
    pub body: serde_json::Value,
    pub created: String,
    pub updated: String,
}

/// Transition.
#[derive(Debug, Clone, Deserialize)]
pub struct Transition {
    pub id: String,
    pub name: String,
    pub to: Status,
}

/// Search result.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub start_at: u32,
    pub max_results: u32,
    pub total: u32,
    pub issues: Vec<Issue>,
}

/// Input for creating an issue.
#[derive(Debug, Clone, Serialize)]
pub struct CreateIssueInput {
    pub fields: CreateIssueFields,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateIssueFields {
    pub project: ProjectRef,
    pub summary: String,
    pub issuetype: IssueTypeRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<PriorityRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<UserRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectRef {
    pub key: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IssueTypeRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriorityRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRef {
    pub account_id: String,
}

impl CreateIssueInput {
    pub fn new(project_key: impl Into<String>, summary: impl Into<String>, issue_type: impl Into<String>) -> Self {
        Self {
            fields: CreateIssueFields {
                project: ProjectRef { key: project_key.into() },
                summary: summary.into(),
                issuetype: IssueTypeRef { name: issue_type.into() },
                ..Default::default()
            },
        }
    }

    pub fn description(mut self, desc: serde_json::Value) -> Self {
        self.fields.description = Some(desc);
        self
    }

    pub fn priority(mut self, priority: impl Into<String>) -> Self {
        self.fields.priority = Some(PriorityRef { name: priority.into() });
        self
    }

    pub fn assignee(mut self, account_id: impl Into<String>) -> Self {
        self.fields.assignee = Some(UserRef { account_id: account_id.into() });
        self
    }

    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.fields.labels = Some(labels);
        self
    }
}

/// Input for updating an issue.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateIssueInput {
    pub fields: UpdateIssueFields,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateIssueFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<PriorityRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<UserRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

impl UpdateIssueInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.fields.summary = Some(summary.into());
        self
    }

    pub fn description(mut self, desc: serde_json::Value) -> Self {
        self.fields.description = Some(desc);
        self
    }

    pub fn priority(mut self, priority: impl Into<String>) -> Self {
        self.fields.priority = Some(PriorityRef { name: priority.into() });
        self
    }

    pub fn assignee(mut self, account_id: impl Into<String>) -> Self {
        self.fields.assignee = Some(UserRef { account_id: account_id.into() });
        self
    }
}

/// Add comment input.
#[derive(Debug, Clone, Serialize)]
pub struct AddCommentInput {
    pub body: serde_json::Value,
}

impl AddCommentInput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            body: serde_json::json!({
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": text.into()
                    }]
                }]
            }),
        }
    }
}

/// Transition input.
#[derive(Debug, Clone, Serialize)]
pub struct TransitionInput {
    pub transition: TransitionRef,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransitionRef {
    pub id: String,
}

impl TransitionInput {
    pub fn new(transition_id: impl Into<String>) -> Self {
        Self {
            transition: TransitionRef {
                id: transition_id.into(),
            },
        }
    }
}

/// Created issue response.
#[derive(Debug, Clone, Deserialize)]
pub struct CreatedIssue {
    pub id: String,
    pub key: String,
    #[serde(rename = "self")]
    pub self_url: String,
}
