use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("path error: {0}")]
    Path(String),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

impl AppError {
    pub fn path(msg: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::Path(format!("{}: {}", msg.into(), path.into().display()))
    }
}

impl From<AppError> for String {
    fn from(value: AppError) -> Self {
        value.to_string()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
