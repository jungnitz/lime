#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolHint {
    Require(bool),
    Prefer(bool),
    Any,
}

impl BoolHint {
    pub fn map(self, f: impl FnOnce(bool) -> bool) -> Self {
        match self {
            Self::Require(v) => Self::Require(f(v)),
            Self::Prefer(v) => Self::Prefer(f(v)),
            _ => self,
        }
    }
}
