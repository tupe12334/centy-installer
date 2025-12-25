use serde::{Deserialize, Serialize};

/// Known projects that can be installed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Project {
    Tui,
    TuiManager,
}

impl Project {
    /// Get the project name as used in paths
    pub fn name(&self) -> &'static str {
        match self {
            Project::Tui => "tui",
            Project::TuiManager => "tui-manager",
        }
    }

    /// Get the display name of the project
    pub fn display_name(&self) -> &'static str {
        match self {
            Project::Tui => "Centy TUI",
            Project::TuiManager => "TUI Manager",
        }
    }

    /// Get the binary name for this project
    pub fn binary_name(&self) -> &'static str {
        match self {
            Project::Tui => "centy-tui",
            Project::TuiManager => "tui-manager",
        }
    }

    /// Get the GitHub repository name
    pub fn repo_name(&self) -> &'static str {
        match self {
            Project::Tui => "centy-tui",
            Project::TuiManager => "tui-manager",
        }
    }

    /// Get all available projects
    pub fn all() -> &'static [Project] {
        &[Project::Tui, Project::TuiManager]
    }

    /// Parse a project from a string
    pub fn parse(s: &str) -> Option<Project> {
        match s.to_lowercase().as_str() {
            "tui" | "centy-tui" => Some(Project::Tui),
            "tui-manager" | "tuimanager" | "manager" => Some(Project::TuiManager),
            _ => None,
        }
    }
}

impl std::fmt::Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for Project {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Project::parse(s).ok_or_else(|| format!("Unknown project: {}", s))
    }
}
