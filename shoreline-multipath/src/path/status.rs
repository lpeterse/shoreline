
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    Closed,
}

impl Status {
    pub fn is_open(&self) -> bool {
        matches!(self, Status::Open)
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Status::Closed)
    }
}
