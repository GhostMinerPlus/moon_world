#[derive(Debug, Clone)]
pub enum ErrorKind {
    Other(String),
    NotFound,
}

pub type Result<T> = std::result::Result<T, moon_err::Error<ErrorKind>>;
