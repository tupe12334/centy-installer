use crate::error::{InstallerError, Result};
use crate::paths::InstallPaths;
use crate::project::Project;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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

    /// Build the download URL for a binary
    fn build_download_url(&self, project: &Project, version: &str) -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        // Arch name from Rust's ARCH constant
        let arch_name = arch;

        // Map Rust OS names to common naming conventions
        let os_name = match os {
            "macos" => "apple-darwin",
            "linux" => "unknown-linux-gnu",
            "windows" => "pc-windows-msvc",
            _ => os,
        };

        let target = format!("{}-{}", arch_name, os_name);
        let binary_name = project.binary_name();

        if let Some(base_url) = &self.config.download_base_url {
            format!(
                "{}/{}/{}/{}-{}",
                base_url,
                project.name(),
                version,
                binary_name,
                target
            )
        } else {
            // GitHub releases URL
            format!(
                "https://github.com/{}/{}/releases/download/v{}/{}-{}",
                self.config.github_org,
                project.repo_name(),
                version,
                binary_name,
                target
            )
        }
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

        // Ensure directories exist
        self.paths.ensure_dirs(project_name, version)?;

        let binary_path = self.paths.binary_path(project_name, version, binary_name);

        // Download the binary
        let url = self.build_download_url(&project, version);
        self.download_binary(&url, &binary_path).await?;

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
