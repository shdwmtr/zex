use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum OpError {
    Io(io::Error, PathBuf),
    Trash(trash::Error),
    NameConflict(PathBuf),
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpError::Io(err, path) => write!(f, "{}: {err}", path.display()),
            OpError::Trash(err) => write!(f, "couldn't move to trash: {err:?}"),
            OpError::NameConflict(path) => {
                write!(f, "{} already exists", path.display())
            }
        }
    }
}

pub type OpResult<T> = Result<T, Vec<OpError>>;

pub fn describe(errors: &[OpError]) -> String {
    errors
        .iter()
        .map(|err| err.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}
