/// A set of booleans, i.e. either `{}`, `{true}`, `{false}` or `{true, false}`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BoolSet {
    /// `{}`
    #[default]
    None,
    /// `{<value>}`
    Single(bool),
    /// `{true, false}`
    All,
}

impl BoolSet {
    #[must_use]
    pub fn insert(self, value: bool) -> BoolSet {
        match self {
            Self::None => Self::Single(value),
            Self::Single(b) if value == b => self,
            _ => Self::All,
        }
    }
    #[must_use]
    pub fn insert_optional(self, value: Option<bool>) -> BoolSet {
        if let Some(value) = value {
            self.insert(value)
        } else {
            self
        }
    }
    #[must_use]
    pub fn insert_all(self, set: BoolSet) -> BoolSet {
        match (self, set) {
            (Self::All, _) => Self::All,
            (Self::None, other) => other,
            (Self::Single(s), other) => other.insert(s),
        }
    }
    #[must_use]
    pub fn contains(&self, value: bool) -> bool {
        match self {
            Self::All => true,
            Self::Single(s) if *s == value => true,
            _ => false,
        }
    }
}

impl FromIterator<BoolSet> for BoolSet {
    fn from_iter<T: IntoIterator<Item = BoolSet>>(iter: T) -> Self {
        let mut r = BoolSet::None;
        for set in iter {
            r = r.insert_all(set);
        }
        r
    }
}

impl From<bool> for BoolSet {
    fn from(value: bool) -> Self {
        Self::Single(value)
    }
}
