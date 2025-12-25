pub mod error;
pub mod installer;
pub mod paths;
pub mod project;
pub mod version;

pub use error::{InstallerError, Result};
pub use installer::{Installer, InstallerConfig};
pub use paths::InstallPaths;
pub use project::Project;
pub use version::{Version, VersionManager};
