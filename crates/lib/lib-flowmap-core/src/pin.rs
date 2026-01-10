use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinKind {
    Exec,
    String,
    Number,
    Object,
    Boolean,
    Error,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: PinId,
    pub kind: PinKind,
    pub direction: PinDirection,
    pub label: String,
    pub connected: bool,
}

impl Pin {
    pub fn exec_in() -> Self {
        Self {
            id: PinId(0),
            kind: PinKind::Exec,
            direction: PinDirection::Input,
            label: String::new(),
            connected: false,
        }
    }

    pub fn exec_out() -> Self {
        Self {
            id: PinId(0),
            kind: PinKind::Exec,
            direction: PinDirection::Output,
            label: String::new(),
            connected: false,
        }
    }

    pub fn error_out() -> Self {
        Self {
            id: PinId(0),
            kind: PinKind::Error,
            direction: PinDirection::Output,
            label: "error".to_string(),
            connected: false,
        }
    }

    pub fn data_out(kind: PinKind, label: &str) -> Self {
        Self {
            id: PinId(0),
            kind,
            direction: PinDirection::Output,
            label: label.to_string(),
            connected: false,
        }
    }

    pub fn data_in(kind: PinKind, label: &str) -> Self {
        Self {
            id: PinId(0),
            kind,
            direction: PinDirection::Input,
            label: label.to_string(),
            connected: false,
        }
    }

    pub fn with_id(mut self, id: u64) -> Self {
        self.id = PinId(id);
        self
    }
}
