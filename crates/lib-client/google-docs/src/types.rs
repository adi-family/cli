use serde::{Deserialize, Serialize};

/// Document.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub document_id: String,
    pub title: String,
    pub body: Option<Body>,
    pub revision_id: Option<String>,
}

/// Document body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub content: Option<Vec<StructuralElement>>,
}

/// Structural element.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralElement {
    pub start_index: Option<i32>,
    pub end_index: Option<i32>,
    pub paragraph: Option<Paragraph>,
    pub table: Option<Table>,
    pub section_break: Option<SectionBreak>,
}

/// Paragraph.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paragraph {
    pub elements: Vec<ParagraphElement>,
    pub paragraph_style: Option<ParagraphStyle>,
}

/// Paragraph element.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphElement {
    pub start_index: Option<i32>,
    pub end_index: Option<i32>,
    pub text_run: Option<TextRun>,
}

/// Text run.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRun {
    pub content: String,
    pub text_style: Option<TextStyle>,
}

/// Text style.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<Dimension>,
}

/// Dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    pub magnitude: f64,
    pub unit: String,
}

impl Dimension {
    pub fn pt(magnitude: f64) -> Self {
        Self {
            magnitude,
            unit: "PT".to_string(),
        }
    }
}

/// Paragraph style.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphStyle {
    pub named_style_type: Option<String>,
    pub alignment: Option<String>,
}

/// Table.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub rows: i32,
    pub columns: i32,
    pub table_rows: Vec<TableRow>,
}

/// Table row.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub table_cells: Vec<TableCell>,
}

/// Table cell.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub content: Vec<StructuralElement>,
}

/// Section break.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionBreak {
    pub section_style: Option<SectionStyle>,
}

/// Section style.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionStyle {
    pub section_type: Option<String>,
}

/// Batch update request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateRequest {
    pub requests: Vec<Request>,
}

/// Request for batch update.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<InsertTextRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_content_range: Option<DeleteContentRangeRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_text_style: Option<UpdateTextStyleRequest>,
}

impl Request {
    pub fn insert_text(text: impl Into<String>, index: i32) -> Self {
        Self {
            insert_text: Some(InsertTextRequest {
                text: text.into(),
                location: Location { index },
            }),
            delete_content_range: None,
            update_text_style: None,
        }
    }

    pub fn delete_range(start: i32, end: i32) -> Self {
        Self {
            insert_text: None,
            delete_content_range: Some(DeleteContentRangeRequest {
                range: Range {
                    start_index: start,
                    end_index: end,
                },
            }),
            update_text_style: None,
        }
    }
}

/// Insert text request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertTextRequest {
    pub text: String,
    pub location: Location,
}

/// Location.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub index: i32,
}

/// Delete content range request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteContentRangeRequest {
    pub range: Range,
}

/// Range.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start_index: i32,
    pub end_index: i32,
}

/// Update text style request.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTextStyleRequest {
    pub range: Range,
    pub text_style: TextStyle,
    pub fields: String,
}

/// Batch update response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateResponse {
    pub document_id: String,
    pub replies: Vec<serde_json::Value>,
}

/// Create document request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateDocumentRequest {
    pub title: String,
}

impl CreateDocumentRequest {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}
