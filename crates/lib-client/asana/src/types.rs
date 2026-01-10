use serde::{Deserialize, Serialize};

/// Asana API response wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct AsanaResponse<T> {
    pub data: T,
}

/// Task.
#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub gid: String,
    pub name: String,
    pub notes: Option<String>,
    pub completed: bool,
    pub completed_at: Option<String>,
    pub due_on: Option<String>,
    pub due_at: Option<String>,
    pub assignee: Option<User>,
    pub projects: Option<Vec<Project>>,
    pub tags: Option<Vec<Tag>>,
    pub created_at: String,
    pub modified_at: String,
    pub permalink_url: Option<String>,
}

/// Project.
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub gid: String,
    pub name: String,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub archived: Option<bool>,
    pub workspace: Option<Workspace>,
    pub team: Option<Team>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub permalink_url: Option<String>,
}

/// Workspace.
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub gid: String,
    pub name: String,
}

/// Team.
#[derive(Debug, Clone, Deserialize)]
pub struct Team {
    pub gid: String,
    pub name: String,
}

/// Section.
#[derive(Debug, Clone, Deserialize)]
pub struct Section {
    pub gid: String,
    pub name: String,
    pub project: Option<Project>,
}

/// User.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub gid: String,
    pub name: String,
    pub email: Option<String>,
}

/// Tag.
#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    pub gid: String,
    pub name: String,
    pub color: Option<String>,
}

/// Task list response.
#[derive(Debug, Clone, Deserialize)]
pub struct TaskList {
    pub data: Vec<Task>,
    pub next_page: Option<NextPage>,
}

/// Next page info.
#[derive(Debug, Clone, Deserialize)]
pub struct NextPage {
    pub offset: String,
    pub uri: String,
}

/// Create task input.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateTaskInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl CreateTaskInput {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn workspace(mut self, gid: impl Into<String>) -> Self {
        self.workspace = Some(gid.into());
        self
    }

    pub fn project(mut self, gid: impl Into<String>) -> Self {
        self.projects = Some(vec![gid.into()]);
        self
    }

    pub fn assignee(mut self, gid: impl Into<String>) -> Self {
        self.assignee = Some(gid.into());
        self
    }

    pub fn due_on(mut self, date: impl Into<String>) -> Self {
        self.due_on = Some(date.into());
        self
    }
}

/// Update task input.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateTaskInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
}

impl UpdateTaskInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn completed(mut self, completed: bool) -> Self {
        self.completed = Some(completed);
        self
    }

    pub fn assignee(mut self, gid: impl Into<String>) -> Self {
        self.assignee = Some(gid.into());
        self
    }

    pub fn due_on(mut self, date: impl Into<String>) -> Self {
        self.due_on = Some(date.into());
        self
    }
}

/// Add to project input.
#[derive(Debug, Clone, Serialize)]
pub struct AddToProjectInput {
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

impl AddToProjectInput {
    pub fn new(project_gid: impl Into<String>) -> Self {
        Self {
            project: project_gid.into(),
            section: None,
        }
    }

    pub fn section(mut self, section_gid: impl Into<String>) -> Self {
        self.section = Some(section_gid.into());
        self
    }
}
