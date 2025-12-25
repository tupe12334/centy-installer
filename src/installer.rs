use crate::error::{InstallerError, Result};
use crate::paths::InstallPaths;
use crate::project::Project;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tar::Archive;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Configuration for the installer
#[derive(Debug, Clone)]
pub struct InstallerConfig {
    /// GitHub organization or user
    pub github_org: String,
    /// Base URL for downloads (if not using GitHub releases)
    pub download_base_url: Option<String>,
}

impl Default for InstallerConfig {
    fn default() -> Self {
        Self {
            github_org: "centy-io".to_string(),
            download_base_url: None,
        }
    }
}

/// Main installer struct
pub struct Installer {
    paths: InstallPaths,
    config: InstallerConfig,
    client: reqwest::Client,
}

impl Installer {
    /// Create a new installer with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(InstallerConfig::default())
    }

    /// Create a new installer with custom configuration
    pub fn with_config(config: InstallerConfig) -> Result<Self> {
        let paths = InstallPaths::new()?;
        let client = reqwest::Client::builder()
            .user_agent("centy-installer")
            .build()
            .map_err(InstallerError::Http)?;

        Ok(Self {
            paths,
            config,
            client,
        })
    }

    /// Get the installation paths
    pub fn paths(&self) -> &InstallPaths {
        &self.paths
    }

    /// Get the target triple for the current platform
    fn get_target() -> (String, String) {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        // Map Rust OS names to common naming conventions
        let os_name = match os {
            "macos" => "apple-darwin",
            "linux" => "unknown-linux-gnu",
            "windows" => "pc-windows-msvc",
            _ => os,
        };

        // Get archive extension based on OS
        let ext = match os {
            "windows" => "zip",
            _ => "tar.gz",
        };

        (format!("{}-{}", arch, os_name), ext.to_string())
    }

    /// Build the download URL for a binary
    /// Format: {binary}-v{version}-{arch}-{os}.{ext}
    /// Example: centy-daemon-v0.1.6-x86_64-apple-darwin.tar.gz
    fn build_download_url(&self, project: &Project, version: &str) -> (String, String) {
        let (target, ext) = Self::get_target();
        let binary_name = project.binary_name();

        // Ensure version has 'v' prefix
        let version_tag = if version.starts_with('v') {
            version.to_string()
        } else {
            format!("v{}", version)
        };

        let archive_name = format!("{}-{}-{}.{}", binary_name, version_tag, target, ext);

        let url = if let Some(base_url) = &self.config.download_base_url {
            format!("{}/{}/{}/{}", base_url, project.name(), version, archive_name)
        } else {
            // GitHub releases URL
            format!(
                "https://github.com/{}/{}/releases/download/{}/{}",
                self.config.github_org,
                project.repo_name(),
                version_tag,
                archive_name
            )
        };

        (url, ext)
    }

    /// Install a specific version of a project
    pub async fn install(&self, project: Project, version: &str) -> Result<PathBuf> {
        let project_name = project.name();
        let binary_name = project.binary_name();

        println!(
            "Installing {} version {}...",
            project.display_name(),
            version
        );

        // Ensure directories exist (including bin dir for symlinks)
        self.paths.ensure_dirs(project_name, version)?;
        std::fs::create_dir_all(self.paths.bin_dir())?;

        let binary_path = self.paths.binary_path(project_name, version, binary_name);

        // Create temp directory for download
        let temp_dir = TempDir::new().map_err(|e| InstallerError::IoError(e.to_string()))?;

        // Download the archive
        let (url, ext) = self.build_download_url(&project, version);
        let archive_path = temp_dir.path().join(format!("download.{}", ext));
        self.download_binary(&url, &archive_path).await?;

        // Extract the binary from archive
        println!("Extracting...");
        self.extract_binary(&archive_path, &ext, binary_name, &binary_path)?;

        // Make executable
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        // Create symlink
        let symlink_path = self.paths.symlink_path(binary_name);
        self.create_symlink(&binary_path, &symlink_path)?;

        println!(
            "Successfully installed {} {} to {}",
            project.display_name(),
            version,
            binary_path.display()
        );
        println!("Symlink: {}", symlink_path.display());

        Ok(binary_path)
    }

    /// Extract binary from archive
    fn extract_binary(
        &self,
        archive_path: &PathBuf,
        ext: &str,
        binary_name: &str,
        dest_path: &PathBuf,
    ) -> Result<()> {
        match ext {
            "tar.gz" => {
                let file = File::open(archive_path)?;
                let decoder = GzDecoder::new(file);
                let mut archive = Archive::new(decoder);

                // Extract to temp location first
                let temp_extract = archive_path.parent().unwrap().join("extracted");
                std::fs::create_dir_all(&temp_extract)?;
                archive.unpack(&temp_extract)?;

                // Find and move the binary
                let found = self.find_binary_in_dir(&temp_extract, binary_name)?;
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&found, dest_path)?;
                Ok(())
            }
            "zip" => {
                let file = File::open(archive_path)?;
                let mut archive = zip::ZipArchive::new(file)
                    .map_err(|e| InstallerError::ExtractFailed(e.to_string()))?;

                let temp_extract = archive_path.parent().unwrap().join("extracted");
                std::fs::create_dir_all(&temp_extract)?;
                archive
                    .extract(&temp_extract)
                    .map_err(|e| InstallerError::ExtractFailed(e.to_string()))?;

                let found = self.find_binary_in_dir(&temp_extract, binary_name)?;
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&found, dest_path)?;
                Ok(())
            }
            _ => Err(InstallerError::ExtractFailed(format!(
                "Unknown archive format: {}",
                ext
            ))),
        }
    }

    /// Find binary in extracted directory (searches recursively)
    fn find_binary_in_dir(&self, dir: &PathBuf, binary_name: &str) -> Result<PathBuf> {
        // First check directly in dir
        let direct = dir.join(binary_name);
        if direct.exists() && direct.is_file() {
            return Ok(direct);
        }

        // Search recursively
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name() {
                    if name == binary_name {
                        return Ok(path);
                    }
                }
            } else if path.is_dir() {
                if let Ok(found) = self.find_binary_in_dir(&path, binary_name) {
                    return Ok(found);
                }
            }
        }

        Err(InstallerError::BinaryNotFound(format!(
            "{} not found in archive",
            binary_name
        )))
    }

    /// Create symlink to binary
    fn create_symlink(&self, binary_path: &PathBuf, symlink_path: &PathBuf) -> Result<()> {
        // Remove existing symlink if present
        if symlink_path.exists() || symlink_path.is_symlink() {
            std::fs::remove_file(symlink_path)?;
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(binary_path, symlink_path)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(binary_path, symlink_path)?;
        }

        Ok(())
    }

    /// Download a binary from URL to path with progress bar
    async fn download_binary(&self, url: &str, path: &PathBuf) -> Result<()> {
        println!("Downloading from: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| InstallerError::DownloadFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(InstallerError::DownloadFailed(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        let total_size = response.content_length().unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        let mut file = std::fs::File::create(path)?;
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| InstallerError::DownloadFailed(e.to_string()))?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download complete");
        Ok(())
    }

    /// Install from a local file
    pub fn install_from_file(
        &self,
        project: Project,
        version: &str,
        source_path: &PathBuf,
    ) -> Result<PathBuf> {
        let project_name = project.name();
        let binary_name = project.binary_name();

        println!(
            "Installing {} version {} from local file...",
            project.display_name(),
            version
        );

        // Ensure directories exist
        self.paths.ensure_dirs(project_name, version)?;

        let binary_path = self.paths.binary_path(project_name, version, binary_name);

        // Copy the file
        std::fs::copy(source_path, &binary_path)?;

        // Make executable
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        println!(
            "Successfully installed {} {} to {}",
            project.display_name(),
            version,
            binary_path.display()
        );

        Ok(binary_path)
    }

    /// Uninstall a specific version of a project
    pub fn uninstall(&self, project: Project, version: &str) -> Result<()> {
        let project_name = project.name();

        if !self
            .paths
            .is_installed(project_name, version, project.binary_name())
        {
            return Err(InstallerError::VersionNotFound(format!(
                "{} version {}",
                project.display_name(),
                version
            )));
        }

        println!(
            "Uninstalling {} version {}...",
            project.display_name(),
            version
        );

        self.paths.remove_version(project_name, version)?;

        println!(
            "Successfully uninstalled {} {}",
            project.display_name(),
            version
        );

        Ok(())
    }

    /// Uninstall all versions of a project
    pub fn uninstall_all(&self, project: Project) -> Result<()> {
        let project_name = project.name();

        println!("Uninstalling all versions of {}...", project.display_name());

        self.paths.remove_project(project_name)?;

        println!(
            "Successfully uninstalled all versions of {}",
            project.display_name()
        );

        Ok(())
    }

    /// List all installed versions
    pub fn list_installed(&self) -> Result<Vec<(Project, Vec<String>)>> {
        let mut result = Vec::new();

        for project in Project::all() {
            let versions = self.paths.list_versions(project.name())?;
            if !versions.is_empty() {
                result.push((*project, versions));
            }
        }

        Ok(result)
    }

    /// Get the path to an installed binary
    pub fn get_binary_path(&self, project: Project, version: &str) -> Result<PathBuf> {
        let path = self
            .paths
            .binary_path(project.name(), version, project.binary_name());

        if !path.exists() {
            return Err(InstallerError::BinaryNotFound(format!(
                "{} version {}",
                project.display_name(),
                version
            )));
        }

        Ok(path)
    }
}
