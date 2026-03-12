//! Reusable iced UI components
//!
//! This library provides themed, reusable components built on Iced widgets.
//!
//! ## Modules
//! - `fonts` - Font constants (UI, code, icon fonts)
//! - `colors` - UI color abstraction and gradient utilities
//! - `button` - Button components (primary, secondary, icon, pill)
//! - `card` - Card/container components (modal, backdrop, section)
//! - `input` - Input field components (command, search)
//! - `pill` - Status pill indicators (git, running, env, stats)
//! - `tabs` - Tab bar components (session, simple, nav)

pub mod button;
pub mod card;
pub mod colors;
pub mod fonts;
pub mod input;
pub mod pill;
pub mod tabs;

// Re-export commonly used types at the crate root
pub use button::{
    action_button, close_button, icon_button, pill_button, primary_button, secondary_button,
    tab_button, text_button, ButtonVariant, IconButton,
};
pub use card::{backdrop, card, code_block, modal_card, section, Card, CardStyle};
pub use colors::{subtle_gradient, to_iced, vignette_gradient, UiColors};
pub use fonts::{
    bold_font, custom_font, CODE_FONT, CODE_FONT_BOLD, ICON_FONT, ICON_FONT_BOLD, UI_FONT,
    UI_FONT_BOLD, UI_FONT_MEDIUM, UI_FONT_SEMIBOLD,
};
pub use input::{command_input, search_input, styled_input, InputConfig};
pub use pill::{empty_pill, env_pill, git_pill, running_pill, stats_pill, status_pill};
pub use tabs::{nav_tabs, session_tabs, simple_tabs, TabItem};
