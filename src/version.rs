use crate::error::{InstallerError, Result};
use crate::project::Project;
use serde::Deserialize;

/// Represents a semantic version
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl Version {
    /// Parse a version string (e.g., "1.2.3" or "1.2.3-beta.1")
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim_start_matches('v');

        let (version_part, prerelease) = if let Some((v, pre)) = s.split_once('-') {
            (v, Some(pre.to_string()))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Err(InstallerError::InvalidVersion(s.to_string()));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| InstallerError::InvalidVersion(s.to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| InstallerError::InvalidVersion(s.to_string()))?;
        let patch = if parts.len() == 3 {
            parts[2]
                .parse()
                .map_err(|_| InstallerError::InvalidVersion(s.to_string()))?
        } else {
            0
        };

        Ok(Self {
            major,
            minor,
            patch,
            prerelease,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pre) = &self.prerelease {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// GitHub release information
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub prerelease: bool,
    pub draft: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Version manager for fetching available versions from GitHub
pub struct VersionManager {
    client: reqwest::Client,
    github_org: String,
}

impl VersionManager {
    pub fn new(github_org: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("centy-installer")
            .build()
            .map_err(InstallerError::Http)?;

        Ok(Self { client, github_org })
    }

    /// Fetch all available releases for a project from GitHub
    pub async fn fetch_releases(&self, project: &Project) -> Result<Vec<GitHubRelease>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases",
            self.github_org,
            project.repo_name()
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(InstallerError::Http)?;

        if !response.status().is_success() {
            return Err(InstallerError::ProjectNotFound(project.name().to_string()));
        }

        let releases: Vec<GitHubRelease> = response.json().await.map_err(InstallerError::Http)?;

        // Filter out drafts
        let releases = releases.into_iter().filter(|r| !r.draft).collect();

        Ok(releases)
    }

    /// Get the latest stable release for a project
    pub async fn get_latest_version(&self, project: &Project) -> Result<String> {
        let releases = self.fetch_releases(project).await?;

        releases
            .into_iter()
            .find(|r| !r.prerelease)
            .map(|r| r.tag_name.trim_start_matches('v').to_string())
            .ok_or_else(|| InstallerError::VersionNotFound("no stable releases found".to_string()))
    }

    /// Get all available versions for a project
    pub async fn list_available_versions(
        &self,
        project: &Project,
        include_prerelease: bool,
    ) -> Result<Vec<String>> {
        let releases = self.fetch_releases(project).await?;

        let versions: Vec<String> = releases
            .into_iter()
            .filter(|r| include_prerelease || !r.prerelease)
            .map(|r| r.tag_name.trim_start_matches('v').to_string())
            .collect();

        Ok(versions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease, None);

        let v = Version::parse("v1.2.3-beta.1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease, Some("beta.1".to_string()));
    }

    #[test]
    fn test_version_ordering() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("1.0.1").unwrap();
        let v3 = Version::parse("1.1.0").unwrap();
        let v4 = Version::parse("2.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }
}
