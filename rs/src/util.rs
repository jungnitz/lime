#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolSet {
    None,
    Single(bool),
    All,
}

impl BoolSet {
    pub fn insert(self, value: bool) -> BoolSet {
        match self {
            Self::None => Self::Single(value),
            Self::Single(b) if value == b => self,
            _ => Self::All,
        }
    }
    pub fn inser_optional(self, value: Option<bool>) -> BoolSet {
        if let Some(value) = value {
            self.insert(value)
        } else {
            self
        }
    }
}

impl From<bool> for BoolSet {
    fn from(value: bool) -> Self {
        Self::Single(value)
    }
}
