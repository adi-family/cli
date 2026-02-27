/// Supported website languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    En,
    Uk,
    Hi,
    Ar,
    Pt,
    Zh,
    Ja,
}

pub const DEFAULT_LANG: Language = Language::En;

pub const SUPPORTED_LANGS: &[Language] = &[
    Language::En,
    Language::Uk,
    Language::Hi,
    Language::Ar,
    Language::Pt,
    Language::Zh,
    Language::Ja,
];

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Uk => "uk",
            Self::Hi => "hi",
            Self::Ar => "ar",
            Self::Pt => "pt",
            Self::Zh => "zh",
            Self::Ja => "ja",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::En),
            "uk" => Some(Self::Uk),
            "hi" => Some(Self::Hi),
            "ar" => Some(Self::Ar),
            "pt" => Some(Self::Pt),
            "zh" => Some(Self::Zh),
            "ja" => Some(Self::Ja),
            _ => None,
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Uk => "Українська",
            Self::Hi => "हिन्दी",
            Self::Ar => "العربية",
            Self::Pt => "Português",
            Self::Zh => "中文",
            Self::Ja => "日本語",
        }
    }

    pub fn english_name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Uk => "Ukrainian",
            Self::Hi => "Hindi",
            Self::Ar => "Arabic",
            Self::Pt => "Portuguese",
            Self::Zh => "Chinese",
            Self::Ja => "Japanese",
        }
    }
}
