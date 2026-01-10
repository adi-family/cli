use serde::{Deserialize, Serialize};

/// Board.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub closed: bool,
    pub url: String,
    pub short_url: String,
}

/// List (column).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub id: String,
    pub name: String,
    pub closed: bool,
    pub id_board: String,
    pub pos: f64,
}

/// Card.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub closed: bool,
    pub id_board: String,
    pub id_list: String,
    pub pos: f64,
    pub url: String,
    pub short_url: String,
    pub due: Option<String>,
    pub due_complete: bool,
    pub labels: Vec<Label>,
    pub id_members: Vec<String>,
}

/// Label.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

/// Member.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
}

/// Checklist.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checklist {
    pub id: String,
    pub name: String,
    pub id_card: String,
    pub check_items: Vec<CheckItem>,
}

/// Check item.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    pub id: String,
    pub name: String,
    pub state: String,
    pub pos: f64,
}

/// Create card input.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCardInput {
    pub name: String,
    pub id_list: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_labels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_members: Option<String>,
}

impl CreateCardInput {
    pub fn new(list_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id_list: list_id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn desc(mut self, desc: impl Into<String>) -> Self {
        self.desc = Some(desc.into());
        self
    }

    pub fn due(mut self, due: impl Into<String>) -> Self {
        self.due = Some(due.into());
        self
    }

    pub fn pos(mut self, pos: impl Into<String>) -> Self {
        self.pos = Some(pos.into());
        self
    }
}

/// Update card input.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCardInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_list: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_complete: Option<bool>,
}

impl UpdateCardInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn desc(mut self, desc: impl Into<String>) -> Self {
        self.desc = Some(desc.into());
        self
    }

    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = Some(closed);
        self
    }

    pub fn id_list(mut self, list_id: impl Into<String>) -> Self {
        self.id_list = Some(list_id.into());
        self
    }

    pub fn due(mut self, due: impl Into<String>) -> Self {
        self.due = Some(due.into());
        self
    }

    pub fn due_complete(mut self, complete: bool) -> Self {
        self.due_complete = Some(complete);
        self
    }
}

/// Create list input.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateListInput {
    pub name: String,
    pub id_board: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<String>,
}

impl CreateListInput {
    pub fn new(board_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id_board: board_id.into(),
            name: name.into(),
            pos: None,
        }
    }
}

/// Create board input.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_lists: Option<bool>,
}

impl CreateBoardInput {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            desc: None,
            default_lists: Some(false),
        }
    }

    pub fn desc(mut self, desc: impl Into<String>) -> Self {
        self.desc = Some(desc.into());
        self
    }

    pub fn with_default_lists(mut self) -> Self {
        self.default_lists = Some(true);
        self
    }
}
