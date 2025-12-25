use serde::{Deserialize, Serialize};

/// Known projects that can be installed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Project {
    CentyDaemon,
    Tui,
    CentyDaemonTui,
    TuiManager,
}

impl Project {
    /// Get the project name as used in paths
    pub fn name(&self) -> &'static str {
        match self {
            Project::CentyDaemon => "centy-daemon",
            Project::Tui => "centy-tui",
            Project::CentyDaemonTui => "centy-daemon-tui",
            Project::TuiManager => "tui-manager",
        }
    }

    /// Get the display name of the project
    pub fn display_name(&self) -> &'static str {
        match self {
            Project::CentyDaemon => "Centy Daemon",
            Project::Tui => "Centy TUI",
            Project::CentyDaemonTui => "Centy Daemon TUI",
            Project::TuiManager => "TUI Manager",
        }
    }

    /// Get the binary name for this project
    pub fn binary_name(&self) -> &'static str {
        match self {
            Project::CentyDaemon => "centy-daemon",
            Project::Tui => "centy-tui",
            Project::CentyDaemonTui => "centy-daemon-tui",
            Project::TuiManager => "tui-manager",
        }
    }

    /// Get the GitHub repository name
    pub fn repo_name(&self) -> &'static str {
        match self {
            Project::CentyDaemon => "centy-daemon",
            Project::Tui => "centy-tui",
            Project::CentyDaemonTui => "centy-daemon-tui",
            Project::TuiManager => "tui-manager",
        }
    }

    /// Get all available projects
    pub fn all() -> &'static [Project] {
        &[
            Project::CentyDaemon,
            Project::Tui,
            Project::CentyDaemonTui,
            Project::TuiManager,
        ]
    }

    /// Parse a project from a string
    pub fn parse(s: &str) -> Option<Project> {
        match s.to_lowercase().as_str() {
            "daemon" | "centy-daemon" | "centydaemon" => Some(Project::CentyDaemon),
            "tui" | "centy-tui" | "centytui" => Some(Project::Tui),
            "daemon-tui" | "centy-daemon-tui" | "centydaemontui" => Some(Project::CentyDaemonTui),
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
