use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    ProcessKey,
    CloseSession,
    SessionNotExists,
    SimulateKeySequence,
    CreateSession,
    GetStatus,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

impl std::error::Error for Error {}
