use crate::error::{InstallerError, Result};
use std::path::PathBuf;

/// Represents the installation paths for Centy binaries
/// Structure: ~/.centy/bin/<project>/<version>/<binary>
#[derive(Debug, Clone)]
pub struct InstallPaths {
    /// Base directory: ~/.centy
    base_dir: PathBuf,
}

impl InstallPaths {
    /// Create a new InstallPaths instance
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or(InstallerError::HomeDirNotFound)?;
        let base_dir = home.join(".centy");
        Ok(Self { base_dir })
    }

    /// Get the base centy directory (~/.centy)
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Get the bin directory (~/.centy/bin)
    pub fn bin_dir(&self) -> PathBuf {
        self.base_dir.join("bin")
    }

    /// Get the project directory (~/.centy/bin/<project>)
    pub fn project_dir(&self, project: &str) -> PathBuf {
        self.bin_dir().join(project)
    }

    /// Get the version directory (~/.centy/bin/<project>/<version>)
    pub fn version_dir(&self, project: &str, version: &str) -> PathBuf {
        self.project_dir(project).join(version)
    }

    /// Get the full binary path (~/.centy/bin/<project>/<version>/<binary>)
    pub fn binary_path(&self, project: &str, version: &str, binary: &str) -> PathBuf {
        self.version_dir(project, version).join(binary)
    }

    /// Create all necessary directories for a binary installation
    pub fn ensure_dirs(&self, project: &str, version: &str) -> Result<()> {
        let version_dir = self.version_dir(project, version);
        std::fs::create_dir_all(&version_dir)?;
        Ok(())
    }

    /// List all installed projects
    pub fn list_projects(&self) -> Result<Vec<String>> {
        let bin_dir = self.bin_dir();
        if !bin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut projects = Vec::new();
        for entry in std::fs::read_dir(bin_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    projects.push(name.to_string());
                }
            }
        }
        projects.sort();
        Ok(projects)
    }

    /// List all installed versions for a project
    pub fn list_versions(&self, project: &str) -> Result<Vec<String>> {
        let project_dir = self.project_dir(project);
        if !project_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(project_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    versions.push(name.to_string());
                }
            }
        }
        versions.sort();
        Ok(versions)
    }

    /// List all binaries for a specific project version
    pub fn list_binaries(&self, project: &str, version: &str) -> Result<Vec<String>> {
        let version_dir = self.version_dir(project, version);
        if !version_dir.exists() {
            return Ok(Vec::new());
        }

        let mut binaries = Vec::new();
        for entry in std::fs::read_dir(version_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    binaries.push(name.to_string());
                }
            }
        }
        binaries.sort();
        Ok(binaries)
    }

    /// Check if a specific binary is installed
    pub fn is_installed(&self, project: &str, version: &str, binary: &str) -> bool {
        self.binary_path(project, version, binary).exists()
    }

    /// Remove a specific version of a project
    pub fn remove_version(&self, project: &str, version: &str) -> Result<()> {
        let version_dir = self.version_dir(project, version);
        if version_dir.exists() {
            std::fs::remove_dir_all(version_dir)?;
        }
        Ok(())
    }

    /// Remove all versions of a project
    pub fn remove_project(&self, project: &str) -> Result<()> {
        let project_dir = self.project_dir(project);
        if project_dir.exists() {
            std::fs::remove_dir_all(project_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        let paths = InstallPaths::new().unwrap();

        let bin_path = paths.binary_path("tui", "1.0.0", "centy-tui");
        assert!(bin_path.ends_with(".centy/bin/tui/1.0.0/centy-tui"));
    }
}
