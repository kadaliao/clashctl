use serde::{Deserialize, Serialize};

/// Preset defines a complexity template
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    /// Default: Simple mode by default, full features available
    Default,
    /// Work: Hide speed test, minimal UI
    Work,
    /// Strict: Disable one-click operations, require Expert mode
    Strict,
    /// Expert: Default to Expert mode, show all details
    Expert,
}

impl Preset {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "default" => Some(Preset::Default),
            "work" => Some(Preset::Work),
            "strict" => Some(Preset::Strict),
            "expert" => Some(Preset::Expert),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Preset::Default => "default",
            Preset::Work => "work",
            Preset::Strict => "strict",
            Preset::Expert => "expert",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Preset::Default => "Default",
            Preset::Work => "Work",
            Preset::Strict => "Strict",
            Preset::Expert => "Expert",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Preset::Default => "Simple mode by default, all features available",
            Preset::Work => "Minimal UI, hide speed test, focus on switching",
            Preset::Strict => "Disable quick operations, require Expert mode",
            Preset::Expert => "Expert mode by default, show all details",
        }
    }

    /// Should show speed test feature
    pub fn show_speed_test(&self) -> bool {
        match self {
            Preset::Default => true,
            Preset::Work => false,
            Preset::Strict => true,
            Preset::Expert => true,
        }
    }

    /// Should allow one-click switching (without entering Expert mode)
    pub fn allow_quick_switch(&self) -> bool {
        match self {
            Preset::Default => true,
            Preset::Work => true,
            Preset::Strict => false,
            Preset::Expert => true,
        }
    }

    /// Default mode for this preset
    pub fn default_mode(&self) -> crate::app::Mode {
        match self {
            Preset::Default => crate::app::Mode::Simple,
            Preset::Work => crate::app::Mode::Simple,
            Preset::Strict => crate::app::Mode::Simple,
            Preset::Expert => crate::app::Mode::Expert,
        }
    }

    /// Should auto-refresh (show changing data frequently)
    pub fn auto_refresh(&self) -> bool {
        match self {
            Preset::Default => true,
            Preset::Work => true,
            Preset::Strict => true,
            Preset::Expert => true,
        }
    }

    /// All available presets
    pub fn all() -> Vec<Preset> {
        vec![
            Preset::Default,
            Preset::Work,
            Preset::Strict,
            Preset::Expert,
        ]
    }

    /// Next preset in cycle
    pub fn next(&self) -> Self {
        match self {
            Preset::Default => Preset::Work,
            Preset::Work => Preset::Strict,
            Preset::Strict => Preset::Expert,
            Preset::Expert => Preset::Default,
        }
    }
}

impl Default for Preset {
    fn default() -> Self {
        Preset::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_from_str() {
        assert_eq!(Preset::from_str("default"), Some(Preset::Default));
        assert_eq!(Preset::from_str("work"), Some(Preset::Work));
        assert_eq!(Preset::from_str("strict"), Some(Preset::Strict));
        assert_eq!(Preset::from_str("expert"), Some(Preset::Expert));
        assert_eq!(Preset::from_str("invalid"), None);
    }

    #[test]
    fn test_preset_behaviors() {
        assert!(Preset::Default.show_speed_test());
        assert!(!Preset::Work.show_speed_test());
        assert!(Preset::Strict.show_speed_test());

        assert!(Preset::Default.allow_quick_switch());
        assert!(!Preset::Strict.allow_quick_switch());
    }

    #[test]
    fn test_preset_cycle() {
        let preset = Preset::Default;
        assert_eq!(preset.next(), Preset::Work);
        assert_eq!(preset.next().next(), Preset::Strict);
        assert_eq!(preset.next().next().next(), Preset::Expert);
        assert_eq!(preset.next().next().next().next(), Preset::Default);
    }
}
