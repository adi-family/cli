use serde::Serialize;

/// Block Kit block.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Section(SectionBlock),
    Divider,
    Header(HeaderBlock),
    Context(ContextBlock),
    Actions(ActionsBlock),
    Image(ImageBlock),
}

/// Section block.
#[derive(Debug, Clone, Serialize)]
pub struct SectionBlock {
    pub text: TextObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessory: Option<BlockElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<TextObject>>,
}

/// Header block.
#[derive(Debug, Clone, Serialize)]
pub struct HeaderBlock {
    pub text: TextObject,
}

/// Context block.
#[derive(Debug, Clone, Serialize)]
pub struct ContextBlock {
    pub elements: Vec<ContextElement>,
}

/// Actions block.
#[derive(Debug, Clone, Serialize)]
pub struct ActionsBlock {
    pub elements: Vec<BlockElement>,
}

/// Image block.
#[derive(Debug, Clone, Serialize)]
pub struct ImageBlock {
    pub image_url: String,
    pub alt_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<TextObject>,
}

/// Text object.
#[derive(Debug, Clone, Serialize)]
pub struct TextObject {
    #[serde(rename = "type")]
    pub text_type: TextType,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextType {
    PlainText,
    Mrkdwn,
}

impl TextObject {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text_type: TextType::PlainText,
            text: text.into(),
            emoji: Some(true),
        }
    }

    pub fn mrkdwn(text: impl Into<String>) -> Self {
        Self {
            text_type: TextType::Mrkdwn,
            text: text.into(),
            emoji: None,
        }
    }
}

/// Context element.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ContextElement {
    Text(TextObject),
    Image { type_: String, image_url: String, alt_text: String },
}

/// Block element.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockElement {
    Button(ButtonElement),
    StaticSelect(StaticSelectElement),
    Overflow(OverflowElement),
}

/// Button element.
#[derive(Debug, Clone, Serialize)]
pub struct ButtonElement {
    pub text: TextObject,
    pub action_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ButtonStyle>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonStyle {
    Primary,
    Danger,
}

/// Static select element.
#[derive(Debug, Clone, Serialize)]
pub struct StaticSelectElement {
    pub placeholder: TextObject,
    pub action_id: String,
    pub options: Vec<SelectOption>,
}

/// Select option.
#[derive(Debug, Clone, Serialize)]
pub struct SelectOption {
    pub text: TextObject,
    pub value: String,
}

/// Overflow element.
#[derive(Debug, Clone, Serialize)]
pub struct OverflowElement {
    pub action_id: String,
    pub options: Vec<SelectOption>,
}

// Builder helpers
impl Block {
    pub fn section(text: TextObject) -> Self {
        Self::Section(SectionBlock {
            text,
            accessory: None,
            fields: None,
        })
    }

    pub fn divider() -> Self {
        Self::Divider
    }

    pub fn header(text: impl Into<String>) -> Self {
        Self::Header(HeaderBlock {
            text: TextObject::plain(text),
        })
    }

    pub fn context(elements: Vec<ContextElement>) -> Self {
        Self::Context(ContextBlock { elements })
    }
}
