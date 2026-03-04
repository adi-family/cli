use serde::{Deserialize, Serialize};

/// Issue in Linear.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i32,
    pub state: Option<IssueState>,
    pub assignee: Option<User>,
    pub project: Option<Project>,
    pub team: Option<Team>,
    pub labels: Option<LabelConnection>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub url: String,
}

/// Issue state.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueState {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(rename = "type")]
    pub state_type: String,
}

/// User in Linear.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Project in Linear.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub state: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Team in Linear.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub key: String,
    pub description: Option<String>,
}

/// Label.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
}

/// Label connection.
#[derive(Debug, Clone, Deserialize)]
pub struct LabelConnection {
    pub nodes: Vec<Label>,
}

/// Cycle (sprint).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cycle {
    pub id: String,
    pub number: i32,
    pub name: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub completed_at: Option<String>,
}

/// Issue connection for pagination.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueConnection {
    pub nodes: Vec<Issue>,
    pub page_info: PageInfo,
}

/// Page info for pagination.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

/// Input for creating an issue.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueCreateInput {
    pub title: String,
    pub team_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_ids: Option<Vec<String>>,
}

impl IssueCreateInput {
    pub fn new(title: impl Into<String>, team_id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            team_id: team_id.into(),
            ..Default::default()
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn assignee(mut self, id: impl Into<String>) -> Self {
        self.assignee_id = Some(id.into());
        self
    }

    pub fn project(mut self, id: impl Into<String>) -> Self {
        self.project_id = Some(id.into());
        self
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }
}

/// Input for updating an issue.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueUpdateInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_ids: Option<Vec<String>>,
}

impl IssueUpdateInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn state(mut self, id: impl Into<String>) -> Self {
        self.state_id = Some(id.into());
        self
    }

    pub fn assignee(mut self, id: impl Into<String>) -> Self {
        self.assignee_id = Some(id.into());
        self
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }
}

/// Issue filter for queries.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<StateFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<UserFilter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamFilter {
    pub id: IdComparator,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectFilter {
    pub id: IdComparator,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateFilter {
    pub name: StringComparator,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserFilter {
    pub id: IdComparator,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdComparator {
    pub eq: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StringComparator {
    pub eq: String,
}

/// Payload returned from mutations.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuePayload {
    pub success: bool,
    pub issue: Option<Issue>,
}
