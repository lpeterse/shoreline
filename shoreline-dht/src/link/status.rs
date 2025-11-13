use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Init,
    Good,
    Fail,
    Term,
}

impl Status {
    pub fn is_good(&self) -> bool {
        matches!(self, Status::Good)
    }

    pub fn is_expendable(&self) -> bool {
        matches!(self, Status::Fail | Status::Term)
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
            Status::Fail => write!(f, "FAIL"),
            Status::Term => write!(f, "TERM"),
        }
    }
}
 