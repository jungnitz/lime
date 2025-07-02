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
    pub fn and(self, other: BoolHint) -> Option<Self> {
        match (self, other) {
            (Self::Require(v1), Self::Require(v2)) => {
                if v1 == v2 {
                    Some(Self::Require(v1))
                } else {
                    None
                }
            }
            (Self::Require(v), _) => Some(Self::Require(v)),
            (Self::Prefer(_), Self::Require(v2)) => Some(Self::Require(v2)),
            (Self::Prefer(v1), Self::Prefer(v2)) => {
                if v1 == v2 {
                    Some(Self::Prefer(v1))
                } else {
                    Some(Self::Any)
                }
            }
            (Self::Prefer(v), Self::Any) => Some(Self::Prefer(v)),
            (Self::Any, v) => Some(v),
        }
    }
}
