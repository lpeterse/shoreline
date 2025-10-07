use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Init,
    Good,
    Miss,
    Fail,
}

impl Status {
    pub fn is_good(&self) -> bool {
        matches!(self, Status::Good)
    }

    pub fn is_expendable(&self) -> bool {
        matches!(self, Status::Miss | Status::Fail)
    }
}

impl Default for Status {
    fn default() -> Self {
        Status::Init
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Init => write!(f, "INIT"),
            Status::Good => write!(f, "GOOD"),
            Status::Miss => write!(f, "MISS"),
            Status::Fail => write!(f, "FAIL"),
        }
    }
}
