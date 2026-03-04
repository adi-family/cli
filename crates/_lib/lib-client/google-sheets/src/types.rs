use serde::{Deserialize, Serialize};

/// Spreadsheet.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spreadsheet {
    pub spreadsheet_id: String,
    pub properties: SpreadsheetProperties,
    pub sheets: Option<Vec<Sheet>>,
    pub spreadsheet_url: String,
}

/// Spreadsheet properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpreadsheetProperties {
    pub title: String,
    pub locale: Option<String>,
    pub time_zone: Option<String>,
}

/// Sheet.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub properties: SheetProperties,
}

/// Sheet properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetProperties {
    pub sheet_id: u64,
    pub title: String,
    pub index: u32,
    pub sheet_type: Option<String>,
    pub grid_properties: Option<GridProperties>,
}

/// Grid properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridProperties {
    pub row_count: u32,
    pub column_count: u32,
    pub frozen_row_count: Option<u32>,
    pub frozen_column_count: Option<u32>,
}

/// Value range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueRange {
    pub range: String,
    pub major_dimension: Option<String>,
    pub values: Option<Vec<Vec<serde_json::Value>>>,
}

impl ValueRange {
    pub fn new(range: impl Into<String>, values: Vec<Vec<serde_json::Value>>) -> Self {
        Self {
            range: range.into(),
            major_dimension: Some("ROWS".to_string()),
            values: Some(values),
        }
    }
}

/// Update values response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateValuesResponse {
    pub spreadsheet_id: String,
    pub updated_range: String,
    pub updated_rows: u32,
    pub updated_columns: u32,
    pub updated_cells: u32,
}

/// Append values response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendValuesResponse {
    pub spreadsheet_id: String,
    pub table_range: Option<String>,
    pub updates: UpdateValuesResponse,
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
    pub add_sheet: Option<AddSheetRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_sheet: Option<DeleteSheetRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_sheet_properties: Option<UpdateSheetPropertiesRequest>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSheetRequest {
    pub properties: SheetProperties,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSheetRequest {
    pub sheet_id: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSheetPropertiesRequest {
    pub properties: SheetProperties,
    pub fields: String,
}

/// Batch update response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateResponse {
    pub spreadsheet_id: String,
    pub replies: Vec<serde_json::Value>,
}

/// Create spreadsheet request.
#[derive(Debug, Clone, Serialize)]
pub struct CreateSpreadsheetRequest {
    pub properties: SpreadsheetProperties,
}

impl CreateSpreadsheetRequest {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            properties: SpreadsheetProperties {
                title: title.into(),
                locale: None,
                time_zone: None,
            },
        }
    }
}

/// Value input option.
#[derive(Debug, Clone, Copy)]
pub enum ValueInputOption {
    Raw,
    UserEntered,
}

impl ValueInputOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Raw => "RAW",
            Self::UserEntered => "USER_ENTERED",
        }
    }
}

/// Insert data option.
#[derive(Debug, Clone, Copy)]
pub enum InsertDataOption {
    Overwrite,
    InsertRows,
}

impl InsertDataOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Overwrite => "OVERWRITE",
            Self::InsertRows => "INSERT_ROWS",
        }
    }
}
