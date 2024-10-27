use std::fmt::Display;

use error_stack::Context;

#[derive(Debug)]
pub enum Error {
    Other,
    NotFound,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Context for Error {}

pub type Result<T> = error_stack::Result<T, Error>;
