/// App mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Simple,
    Expert,
}

impl Mode {
    #[allow(dead_code)]
    pub fn toggle(&self) -> Self {
        match self {
            Mode::Simple => Mode::Expert,
            Mode::Expert => Mode::Simple,
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            Mode::Simple => "Simple",
            Mode::Expert => "Expert",
        }
    }
}
