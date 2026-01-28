use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Dracula,
    Nord,
}

impl Theme {
    pub fn as_str(&self) -> &str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Dracula => "dracula",
            Theme::Nord => "nord",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => Theme::Light,
            "dracula" => Theme::Dracula,
            "nord" => Theme::Nord,
            _ => Theme::Dark,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dracula,
            Theme::Dracula => Theme::Nord,
            Theme::Nord => Theme::Dark,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Dracula => "Dracula",
            Theme::Nord => "Nord",
        }
    }

    // Primary colors
    pub fn primary(&self) -> Color {
        match self {
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Blue,
            Theme::Dracula => Color::Magenta,
            Theme::Nord => Color::Cyan,
        }
    }

    pub fn secondary(&self) -> Color {
        match self {
            Theme::Dark => Color::Yellow,
            Theme::Light => Color::Cyan,
            Theme::Dracula => Color::Cyan,
            Theme::Nord => Color::LightBlue,
        }
    }

    pub fn success(&self) -> Color {
        match self {
            Theme::Dark => Color::Green,
            Theme::Light => Color::Green,
            Theme::Dracula => Color::Green,
            Theme::Nord => Color::Green,
        }
    }

    pub fn warning(&self) -> Color {
        match self {
            Theme::Dark => Color::Yellow,
            Theme::Light => Color::Yellow,
            Theme::Dracula => Color::Yellow,
            Theme::Nord => Color::Yellow,
        }
    }

    pub fn error(&self) -> Color {
        match self {
            Theme::Dark => Color::Red,
            Theme::Light => Color::Red,
            Theme::Dracula => Color::Red,
            Theme::Nord => Color::Red,
        }
    }

    pub fn text(&self) -> Color {
        match self {
            Theme::Dark => Color::White,
            Theme::Light => Color::Black,
            Theme::Dracula => Color::White,
            Theme::Nord => Color::White,
        }
    }

    pub fn text_muted(&self) -> Color {
        match self {
            Theme::Dark => Color::Gray,
            Theme::Light => Color::DarkGray,
            Theme::Dracula => Color::Gray,
            Theme::Nord => Color::LightBlue,
        }
    }

    pub fn background(&self) -> Color {
        match self {
            Theme::Dark => Color::Black,
            Theme::Light => Color::White,
            Theme::Dracula => Color::Rgb(40, 42, 54),
            Theme::Nord => Color::Rgb(46, 52, 64),
        }
    }

    pub fn border(&self) -> Color {
        match self {
            Theme::Dark => Color::DarkGray,
            Theme::Light => Color::Gray,
            Theme::Dracula => Color::Rgb(68, 71, 90),
            Theme::Nord => Color::Rgb(76, 86, 106),
        }
    }

    pub fn selected(&self) -> Color {
        match self {
            Theme::Dark => Color::Cyan,
            Theme::Light => Color::Blue,
            Theme::Dracula => Color::Magenta,
            Theme::Nord => Color::Cyan,
        }
    }

    pub fn highlight(&self) -> Color {
        match self {
            Theme::Dark => Color::Yellow,
            Theme::Light => Color::Cyan,
            Theme::Dracula => Color::Rgb(255, 121, 198),
            Theme::Nord => Color::Rgb(136, 192, 208),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}
