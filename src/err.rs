use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Other(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn map_append<E>(append: &'static str) -> impl Fn(E) -> Error
where
    E: Display,
{
    move |e: E| Error::Other(format!("{e}{append}"))
}
