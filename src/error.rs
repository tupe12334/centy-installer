use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstallerError {
    #[error("Failed to determine home directory")]
    HomeDirNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Installation failed: {0}")]
    InstallFailed(String),
}

pub type Result<T> = std::result::Result<T, InstallerError>;
