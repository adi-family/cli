//! Typography configuration for terminal themes

/// Typography configuration with hierarchical sizing
#[derive(Debug, Clone)]
pub struct Typography {
    pub command_size: f32,
    pub output_size: f32,
    pub hint_size: f32,
    pub header_size: f32,
    pub label_size: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            command_size: 16.0,
            output_size: 14.0,
            hint_size: 12.0,
            header_size: 20.0,
            label_size: 14.0,
        }
    }
}
