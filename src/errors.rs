use std::ffi;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    ProcessKey,
    CloseSession,
    SessionNotExists,
    SimulateKeySequence,
    CreateSession,
    GetStatus,
    CStringNul(#[from] ffi::NulError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?}", self).as_str())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
